//! Bayesian optimization over numeric parameters using a simple GP + EI.

use indexmap::IndexMap;
use kraftverk_core::candidate::{Candidate, ParamChange, ParamValue};
use kraftverk_core::error::Result;
use kraftverk_system::Platform;
use rand::Rng;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;

use crate::strategy::{SearchContext, SearchDecision, SearchStrategy};

#[derive(Debug, Clone)]
struct Bound {
    key: String,
    low: f64,
    high: f64,
    previous: ParamValue,
}

/// Simple RBF Gaussian process surrogate with Expected Improvement acquisition.
pub struct BayesianStrategy {
    seed: u64,
    rng: ChaCha8Rng,
    bounds: Vec<Bound>,
    /// Observations: x in [0,1]^d normalized, y = reward.
    xs: Vec<Vec<f64>>,
    ys: Vec<f64>,
    initialized: bool,
    lengthscale: f64,
    noise: f64,
    best_y: f64,
}

impl BayesianStrategy {
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            rng: ChaCha8Rng::seed_from_u64(seed),
            bounds: vec![],
            xs: vec![],
            ys: vec![],
            initialized: false,
            lengthscale: 0.35,
            noise: 1e-3,
            best_y: f64::NEG_INFINITY,
        }
    }

    fn ensure_bounds(&mut self, platform: &dyn Platform) -> Result<()> {
        if self.initialized {
            return Ok(());
        }
        let topo = platform.topology()?;
        let logical = topo.cpu.logical_cpus.max(1) as f64;
        let cur_workers = platform
            .read_param("bench.worker_threads")
            .ok()
            .and_then(|v| v.as_usize())
            .unwrap_or(logical.clamp(1.0, 8.0) as usize);
        let cur_rayon = platform
            .read_param("bench.rayon_threads")
            .ok()
            .and_then(|v| v.as_usize())
            .unwrap_or(cur_workers);

        self.bounds = vec![
            Bound {
                key: "bench.worker_threads".into(),
                low: 1.0,
                high: (logical * 2.0).clamp(2.0, 64.0),
                previous: ParamValue::Int(cur_workers as i64),
            },
            Bound {
                key: "bench.rayon_threads".into(),
                low: 1.0,
                high: (logical * 2.0).clamp(2.0, 64.0),
                previous: ParamValue::Int(cur_rayon as i64),
            },
        ];
        self.initialized = true;
        Ok(())
    }

    #[allow(dead_code)]
    fn encode(&self, values: &[f64]) -> Vec<f64> {
        values
            .iter()
            .zip(self.bounds.iter())
            .map(|(v, b)| {
                let span = (b.high - b.low).max(1e-9);
                ((*v - b.low) / span).clamp(0.0, 1.0)
            })
            .collect()
    }

    fn decode(&self, x: &[f64]) -> Vec<f64> {
        x.iter()
            .zip(self.bounds.iter())
            .map(|(u, b)| {
                let v = b.low + u.clamp(0.0, 1.0) * (b.high - b.low);
                v.round().clamp(b.low, b.high)
            })
            .collect()
    }

    fn kernel(a: &[f64], b: &[f64], lengthscale: f64) -> f64 {
        let mut d2 = 0.0;
        for (x, y) in a.iter().zip(b.iter()) {
            let d = x - y;
            d2 += d * d;
        }
        (-0.5 * d2 / (lengthscale * lengthscale)).exp()
    }

    /// GP posterior mean/var at z (normalized).
    #[allow(clippy::needless_range_loop)]
    fn predict(&self, z: &[f64]) -> (f64, f64) {
        let n = self.xs.len();
        if n == 0 {
            return (0.0, 1.0);
        }
        // Solve (K + noise I) alpha = y via Gauss elimination (n is tiny).
        let mut k = vec![vec![0.0; n]; n];
        for i in 0..n {
            for j in 0..n {
                k[i][j] = Self::kernel(&self.xs[i], &self.xs[j], self.lengthscale);
                if i == j {
                    k[i][j] += self.noise;
                }
            }
        }
        let alpha = solve_linear(&k, &self.ys).unwrap_or_else(|| vec![0.0; n]);
        let mut mean = 0.0;
        let mut kstar = vec![0.0; n];
        for i in 0..n {
            kstar[i] = Self::kernel(z, &self.xs[i], self.lengthscale);
            mean += kstar[i] * alpha[i];
        }
        // var = k(z,z) - k* K^{-1} k*^T
        let vsol = solve_linear(&k, &kstar).unwrap_or_else(|| vec![0.0; n]);
        let mut quad = 0.0;
        for i in 0..n {
            quad += kstar[i] * vsol[i];
        }
        let var = (1.0 - quad).max(1e-9);
        (mean, var)
    }

    /// Expected Improvement (closed form with Φ/φ approx).
    fn expected_improvement(&self, z: &[f64]) -> f64 {
        let (mu, var) = self.predict(z);
        let sigma = var.sqrt();
        if sigma < 1e-12 {
            return 0.0;
        }
        let best = if self.best_y.is_finite() {
            self.best_y
        } else {
            mu
        };
        let u = (mu - best) / sigma;
        let pdf = (-0.5 * u * u).exp() / (std::f64::consts::TAU).sqrt();
        let cdf = 0.5 * (1.0 + erf(u / std::f64::consts::SQRT_2));
        sigma * (u * cdf + pdf)
    }

    fn propose_x(&mut self) -> Vec<f64> {
        let dim = self.bounds.len().max(1);
        if self.xs.is_empty() {
            // Seeded space-filling first point.
            return (0..dim)
                .map(|i| {
                    ((self.seed.wrapping_mul(31).wrapping_add(i as u64 * 17)) % 1000) as f64
                        / 1000.0
                })
                .collect();
        }
        // Random candidates + EI
        let mut best_x = vec![0.5; dim];
        let mut best_ei = f64::NEG_INFINITY;
        for _ in 0..64 {
            let cand: Vec<f64> = (0..dim).map(|_| self.rng.gen::<f64>()).collect();
            let ei = self.expected_improvement(&cand);
            if ei > best_ei {
                best_ei = ei;
                best_x = cand;
            }
        }
        // Also evaluate UCB-style candidates near past good points.
        for x in &self.xs {
            let mut cand = x.clone();
            for v in &mut cand {
                *v = (*v + self.rng.gen_range(-0.1..0.1)).clamp(0.0, 1.0);
            }
            let (mu, var) = self.predict(&cand);
            let ucb = mu + 1.96 * var.sqrt();
            // Blend EI with UCB score.
            let score = self.expected_improvement(&cand) + 0.05 * ucb;
            if score > best_ei {
                best_ei = score;
                best_x = cand;
            }
        }
        best_x
    }

    pub fn observe(&mut self, x_norm: Vec<f64>, y: f64) {
        self.xs.push(x_norm);
        self.ys.push(y);
        if y > self.best_y {
            self.best_y = y;
        }
    }
}

impl SearchStrategy for BayesianStrategy {
    fn name(&self) -> &str {
        "bayesian"
    }

    fn seed(&self) -> u64 {
        self.seed
    }

    fn next_candidate(
        &mut self,
        platform: &dyn Platform,
        ctx: &SearchContext,
        current_best: &Candidate,
    ) -> Result<SearchDecision> {
        if ctx.experiments_done >= ctx.max_experiments {
            return Ok(SearchDecision::Stop {
                reason: "max experiments reached".into(),
            });
        }
        if ctx.elapsed_secs >= ctx.time_budget_secs {
            return Ok(SearchDecision::Stop {
                reason: "time budget exhausted".into(),
            });
        }
        if ctx.plateau_count >= ctx.plateau_limit {
            return Ok(SearchDecision::Stop {
                reason: "plateau detected".into(),
            });
        }

        self.ensure_bounds(platform)?;

        // Observe previous best score as reward for last proposal when available.
        if !self.xs.is_empty() && self.ys.len() < self.xs.len() {
            // Shouldn't happen — we push jointly.
        } else if ctx.experiments_done > 0 && self.ys.len() == self.xs.len() && ctx.best_score > 0.0
        {
            // Update last observation's y if we stored a placeholder.
            if let Some(last) = self.ys.last_mut() {
                if *last == 0.0 && ctx.best_score > 0.0 {
                    *last = ctx.best_score;
                    if ctx.best_score > self.best_y {
                        self.best_y = ctx.best_score;
                    }
                }
            }
        }

        let x = self.propose_x();
        let values = self.decode(&x);
        // Record pending observation with placeholder; optimizer score updates next round.
        self.observe(x.clone(), ctx.best_score.max(0.0));

        let mut changes = current_best.changes.clone();
        for (bound, val) in self.bounds.iter().zip(values.iter()) {
            let next = ParamValue::Int(*val as i64);
            if let Some(pos) = changes.iter().position(|c| c.key == bound.key) {
                changes[pos].next = next;
                changes[pos].rationale = format!("Bayesian EI proposal for {}", bound.key);
            } else {
                changes.push(ParamChange {
                    key: bound.key.clone(),
                    previous: bound.previous.clone(),
                    next,
                    rationale: format!("Bayesian EI proposal for {}", bound.key),
                });
            }
        }

        let mut meta = IndexMap::new();
        meta.insert("strategy".into(), "bayesian".into());
        meta.insert("seed".into(), self.seed.to_string());
        meta.insert("n_obs".into(), self.xs.len().to_string());
        meta.insert(
            "acquisition".into(),
            format!("EI={:.6}", self.expected_improvement(&x)),
        );

        Ok(SearchDecision::Try(Candidate {
            id: format!("bo-{}", ctx.generation),
            label: changes
                .iter()
                .map(|c| format!("{}={}", c.key, c.next.display()))
                .collect::<Vec<_>>()
                .join(","),
            changes,
            meta,
        }))
    }
}

#[allow(clippy::needless_range_loop)]
fn solve_linear(a: &[Vec<f64>], b: &[f64]) -> Option<Vec<f64>> {
    let n = b.len();
    if n == 0 || a.len() != n {
        return None;
    }
    let mut m: Vec<Vec<f64>> = a
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let mut r = row.clone();
            r.push(b[i]);
            r
        })
        .collect();
    for col in 0..n {
        let mut pivot = col;
        for i in col..n {
            if m[i][col].abs() > m[pivot][col].abs() {
                pivot = i;
            }
        }
        if m[pivot][col].abs() < 1e-12 {
            return None;
        }
        m.swap(col, pivot);
        let div = m[col][col];
        for j in col..=n {
            m[col][j] /= div;
        }
        for i in 0..n {
            if i == col {
                continue;
            }
            let f = m[i][col];
            for j in col..=n {
                m[i][j] -= f * m[col][j];
            }
        }
    }
    Some((0..n).map(|i| m[i][n]).collect())
}

/// Abramowitz & Stegun erf approximation.
fn erf(x: f64) -> f64 {
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();
    let a1 = 0.254829592;
    let a2 = -0.284496736;
    let a3 = 1.421413741;
    let a4 = -1.453152027;
    let a5 = 1.061405429;
    let p = 0.3275911;
    let t = 1.0 / (1.0 + p * x);
    let y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * (-x * x).exp();
    sign * y
}

#[cfg(test)]
mod tests {
    use super::*;
    use kraftverk_system::MockPlatform;

    #[test]
    fn ei_proposes_and_is_seed_reproducible() {
        let platform = MockPlatform::with_defaults();
        let mut a = BayesianStrategy::new(99);
        let mut b = BayesianStrategy::new(99);
        let ctx = SearchContext {
            generation: 1,
            experiments_done: 0,
            plateau_count: 0,
            best_score: 0.0,
            last_class: None,
            elapsed_secs: 0,
            max_experiments: 10,
            time_budget_secs: 1000,
            plateau_limit: 10,
        };
        let best = Candidate::identity();
        let d1 = a.next_candidate(&platform, &ctx, &best).unwrap();
        let d2 = b.next_candidate(&platform, &ctx, &best).unwrap();
        match (d1, d2) {
            (SearchDecision::Try(c1), SearchDecision::Try(c2)) => {
                assert_eq!(c1.label, c2.label);
            }
            _ => panic!("expected proposals"),
        }

        // After observing a high reward in one region, EI should move.
        a.xs.clear();
        a.ys.clear();
        a.best_y = f64::NEG_INFINITY;
        a.ensure_bounds(&platform).unwrap();
        a.observe(vec![0.1, 0.1], 10.0);
        a.observe(vec![0.9, 0.9], 100.0);
        let x = a.propose_x();
        // Should tend toward high-reward region (not near 0.1).
        let dist_high: f64 = x.iter().map(|v| (v - 0.9).abs()).sum();
        let dist_low: f64 = x.iter().map(|v| (v - 0.1).abs()).sum();
        assert!(
            dist_high <= dist_low + 0.5,
            "expected EI near high reward, x={x:?}"
        );
    }

    #[test]
    fn kernel_and_solve_sane() {
        let k = BayesianStrategy::kernel(&[0.0], &[0.0], 0.5);
        assert!((k - 1.0).abs() < 1e-9);
        let sol = solve_linear(&[vec![2.0, 0.0], vec![0.0, 2.0]], &[2.0, 4.0]).unwrap();
        assert!((sol[0] - 1.0).abs() < 1e-9);
        assert!((sol[1] - 2.0).abs() < 1e-9);
    }
}
