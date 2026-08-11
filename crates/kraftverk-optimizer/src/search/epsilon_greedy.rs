//! ε-greedy multi-armed bandit over discrete parameter arms.

use indexmap::IndexMap;
use kraftverk_core::candidate::{Candidate, ParamChange, ParamValue};
use kraftverk_core::error::Result;
use kraftverk_system::Platform;
use rand::Rng;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;

use crate::strategy::{SearchContext, SearchDecision, SearchStrategy};

#[derive(Debug, Clone)]
struct Arm {
    change: ParamChange,
    pulls: u64,
    total_reward: f64,
}

/// ε-greedy bandit. Configurable fixed or decaying ε.
pub struct EpsilonGreedyStrategy {
    seed: u64,
    rng: ChaCha8Rng,
    epsilon: f64,
    decay: f64,
    min_epsilon: f64,
    arms: Vec<Arm>,
    initialized: bool,
    last_arm: Option<usize>,
    experiments: usize,
}

impl EpsilonGreedyStrategy {
    pub fn new(seed: u64) -> Self {
        Self::with_epsilon(seed, 0.2, 0.995, 0.05)
    }

    pub fn with_epsilon(seed: u64, epsilon: f64, decay: f64, min_epsilon: f64) -> Self {
        Self {
            seed,
            rng: ChaCha8Rng::seed_from_u64(seed),
            epsilon: epsilon.clamp(0.0, 1.0),
            decay: decay.clamp(0.0, 1.0),
            min_epsilon: min_epsilon.clamp(0.0, 1.0),
            arms: vec![],
            initialized: false,
            last_arm: None,
            experiments: 0,
        }
    }

    fn ensure_arms(&mut self, platform: &dyn Platform) -> Result<()> {
        if self.initialized {
            return Ok(());
        }
        let topo = platform.topology()?;
        let logical = topo.cpu.logical_cpus.max(1);
        let mut arms = Vec::new();

        let cur_workers = platform
            .read_param("bench.worker_threads")
            .ok()
            .and_then(|v| v.as_usize())
            .unwrap_or(logical.clamp(1, 8));
        for &n in &[1usize, 2, (logical / 2).max(1), logical, cur_workers] {
            if n != cur_workers && n >= 1 && n <= logical * 2 && n <= 64 {
                arms.push(Arm {
                    change: ParamChange {
                        key: "bench.worker_threads".into(),
                        previous: ParamValue::Int(cur_workers as i64),
                        next: ParamValue::Int(n as i64),
                        rationale: format!("ε-greedy arm worker_threads={n}"),
                    },
                    pulls: 0,
                    total_reward: 0.0,
                });
            }
        }

        if let Ok(ParamValue::String(cur)) = platform.read_param("process.priority") {
            for level in ["above_normal", "normal"] {
                if level != cur {
                    arms.push(Arm {
                        change: ParamChange {
                            key: "process.priority".into(),
                            previous: ParamValue::String(cur.clone()),
                            next: ParamValue::String(level.into()),
                            rationale: format!("ε-greedy arm priority={level}"),
                        },
                        pulls: 0,
                        total_reward: 0.0,
                    });
                }
            }
        }

        // Dedup by (key, next display)
        let mut seen = std::collections::HashSet::new();
        arms.retain(|a| seen.insert(format!("{}={}", a.change.key, a.change.next.display())));

        self.arms = arms;
        self.initialized = true;
        Ok(())
    }

    fn mean(arm: &Arm) -> f64 {
        if arm.pulls == 0 {
            f64::INFINITY // optimistic init
        } else {
            arm.total_reward / arm.pulls as f64
        }
    }

    /// Record reward for the last proposed arm (called via meta feedback path).
    pub fn observe_reward(&mut self, reward: f64) {
        if let Some(i) = self.last_arm {
            if let Some(arm) = self.arms.get_mut(i) {
                arm.pulls += 1;
                arm.total_reward += reward;
            }
        }
        self.epsilon = (self.epsilon * self.decay).max(self.min_epsilon);
        self.experiments += 1;
    }

    fn select_arm(&mut self) -> Option<usize> {
        if self.arms.is_empty() {
            return None;
        }
        // Force explore each arm once.
        if let Some((i, _)) = self.arms.iter().enumerate().find(|(_, a)| a.pulls == 0) {
            return Some(i);
        }
        let explore = self.rng.gen::<f64>() < self.epsilon;
        if explore {
            Some(self.rng.gen_range(0..self.arms.len()))
        } else {
            self.arms
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| {
                    Self::mean(a)
                        .partial_cmp(&Self::mean(b))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(i, _)| i)
        }
    }
}

impl SearchStrategy for EpsilonGreedyStrategy {
    fn name(&self) -> &str {
        "epsilon_greedy"
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

        self.ensure_arms(platform)?;
        // Update last arm reward from search context best_score delta proxy.
        if self.last_arm.is_some() && ctx.experiments_done > self.experiments {
            let reward = ctx.best_score;
            self.observe_reward(reward);
        }

        let Some(idx) = self.select_arm() else {
            return Ok(SearchDecision::Stop {
                reason: "no arms available".into(),
            });
        };
        self.last_arm = Some(idx);
        let change = self.arms[idx].change.clone();

        let mut changes = current_best.changes.clone();
        if let Some(pos) = changes.iter().position(|c| c.key == change.key) {
            changes[pos].next = change.next.clone();
            changes[pos].rationale = change.rationale.clone();
        } else {
            let mut c = change.clone();
            if let Ok(prev) = platform.read_param(&c.key) {
                c.previous = prev;
            }
            changes.push(c);
        }

        let mut meta = IndexMap::new();
        meta.insert("strategy".into(), "epsilon_greedy".into());
        meta.insert("epsilon".into(), format!("{:.4}", self.epsilon));
        meta.insert("arm".into(), idx.to_string());
        meta.insert("seed".into(), self.seed.to_string());

        Ok(SearchDecision::Try(Candidate {
            id: format!("eg-{}-{}", ctx.generation, idx),
            label: format!("{}={}", change.key, change.next.display()),
            changes,
            meta,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kraftverk_system::MockPlatform;

    #[test]
    fn proposals_change_with_rewards_and_seed() {
        let platform = MockPlatform::with_defaults();
        let mut a = EpsilonGreedyStrategy::with_epsilon(7, 0.5, 1.0, 0.5);
        let mut b = EpsilonGreedyStrategy::with_epsilon(7, 0.5, 1.0, 0.5);
        let ctx = SearchContext {
            generation: 1,
            experiments_done: 0,
            plateau_count: 0,
            best_score: 0.0,
            last_class: None,
            elapsed_secs: 0,
            max_experiments: 20,
            time_budget_secs: 1000,
            plateau_limit: 10,
        };
        let best = Candidate::identity();
        let d1 = a.next_candidate(&platform, &ctx, &best).unwrap();
        let d2 = b.next_candidate(&platform, &ctx, &best).unwrap();
        // Same seed → same first proposal (force-explore first zero-pull arm).
        match (d1, d2) {
            (SearchDecision::Try(c1), SearchDecision::Try(c2)) => {
                assert_eq!(c1.label, c2.label);
            }
            _ => panic!("expected Try"),
        }

        // Bias rewards and ensure greedy phase prefers high-reward arm.
        a.ensure_arms(&platform).unwrap();
        assert!(!a.arms.is_empty());
        for i in 0..a.arms.len() {
            a.last_arm = Some(i);
            a.observe_reward(if i == 0 { 100.0 } else { 1.0 });
        }
        a.epsilon = 0.0; // pure greedy
        let mut picks = vec![];
        for g in 0..8 {
            let ctx = SearchContext {
                generation: g,
                experiments_done: g,
                plateau_count: 0,
                best_score: 50.0,
                last_class: None,
                elapsed_secs: 0,
                max_experiments: 50,
                time_budget_secs: 1000,
                plateau_limit: 20,
            };
            if let Ok(SearchDecision::Try(_)) = a.next_candidate(&platform, &ctx, &best) {
                picks.push(a.last_arm.unwrap());
            }
        }
        assert!(
            picks.iter().filter(|&&i| i == 0).count() >= picks.len() / 2,
            "greedy should prefer arm 0, got {picks:?}"
        );
    }
}
