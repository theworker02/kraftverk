//! Deterministic hill-climbing over a small discrete parameter space.

use indexmap::IndexMap;
use kraftverk_core::candidate::{Candidate, ParamChange, ParamValue};
use kraftverk_core::error::Result;
use kraftverk_system::Platform;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;
use tracing::debug;

use crate::strategy::{SearchContext, SearchDecision, SearchStrategy};

pub struct HillClimbStrategy {
    seed: u64,
    #[allow(dead_code)]
    rng: ChaCha8Rng,
    /// Tried content hashes.
    tried: Vec<String>,
    neighborhood_order: Vec<ParamChange>,
    cursor: usize,
    initialized: bool,
}

impl HillClimbStrategy {
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            rng: ChaCha8Rng::seed_from_u64(seed),
            tried: vec![],
            neighborhood_order: vec![],
            cursor: 0,
            initialized: false,
        }
    }

    fn ensure_neighborhood(&mut self, platform: &dyn Platform, current: &Candidate) -> Result<()> {
        if self.initialized {
            return Ok(());
        }
        let topo = platform.topology()?;
        let logical = topo.cpu.logical_cpus.max(1);
        let mut neighbors = Vec::new();

        let cur_workers = platform
            .read_param("bench.worker_threads")
            .ok()
            .and_then(|v| v.as_usize())
            .unwrap_or(logical.clamp(1, 8));
        for &n in &unique_thread_choices(cur_workers, logical) {
            if n != cur_workers {
                neighbors.push(ParamChange {
                    key: "bench.worker_threads".into(),
                    previous: ParamValue::Int(cur_workers as i64),
                    next: ParamValue::Int(n as i64),
                    rationale: format!(
                        "Hill-climb worker_threads {cur_workers}->{n} to test parallel scaling"
                    ),
                });
            }
        }

        let cur_rayon = platform
            .read_param("bench.rayon_threads")
            .ok()
            .and_then(|v| v.as_usize())
            .unwrap_or(cur_workers);
        for &n in &unique_thread_choices(cur_rayon, logical) {
            if n != cur_rayon {
                neighbors.push(ParamChange {
                    key: "bench.rayon_threads".into(),
                    previous: ParamValue::Int(cur_rayon as i64),
                    next: ParamValue::Int(n as i64),
                    rationale: format!("Hill-climb rayon_threads {cur_rayon}->{n}"),
                });
            }
        }

        if let Ok(ParamValue::String(cur)) = platform.read_param("process.priority") {
            for level in ["above_normal", "normal"] {
                if level != cur {
                    neighbors.push(ParamChange {
                        key: "process.priority".into(),
                        previous: ParamValue::String(cur.clone()),
                        next: ParamValue::String(level.into()),
                        rationale: format!("Try process priority {cur}->{level}"),
                    });
                }
            }
        }

        if logical >= 4 {
            if let Ok(ParamValue::String(cur)) = platform.read_param("process.affinity") {
                for spec in ["first_half", "all"] {
                    if spec != cur {
                        neighbors.push(ParamChange {
                            key: "process.affinity".into(),
                            previous: ParamValue::String(cur.clone()),
                            next: ParamValue::String(spec.into()),
                            rationale: format!("Try affinity {cur}->{spec}"),
                        });
                    }
                }
            }
        }

        let _ = current;

        // Prefer nearby / higher parallelism and above_normal priority first so the
        // plateau budget is not spent on pathological under-subscription (workers=1).
        neighbors.sort_by_key(neighbor_priority);
        // Deterministic light shuffle only among the first few same-band candidates.
        if neighbors.len() > 3 {
            let band_end = neighbors
                .iter()
                .take_while(|c| neighbor_priority(c) < 100)
                .count()
                .max(2);
            let i = (self.seed as usize) % band_end;
            let j = ((self.seed >> 8) as usize) % band_end;
            neighbors.swap(i, j);
        }

        self.neighborhood_order = neighbors;
        self.initialized = true;
        debug!(
            count = self.neighborhood_order.len(),
            "hill-climb neighborhood ready"
        );
        Ok(())
    }
}

fn neighbor_priority(c: &ParamChange) -> i64 {
    match c.key.as_str() {
        "process.priority" => {
            if c.next.display() == "above_normal" {
                5
            } else {
                40
            }
        }
        "bench.worker_threads" | "bench.rayon_threads" => {
            let n = c.next.as_i64().unwrap_or(0);
            let cur = c.previous.as_i64().unwrap_or(0);
            let delta = (n - cur).abs();
            let direction_penalty = if n < cur { 1000 } else { 0 };
            20 + direction_penalty + delta
        }
        "process.affinity" => 500,
        _ => 100,
    }
}

impl SearchStrategy for HillClimbStrategy {
    fn name(&self) -> &str {
        "hill_climb"
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
        if matches!(
            ctx.last_class,
            Some(kraftverk_core::ComparisonClass::UnstableResult)
        ) && ctx.plateau_count >= 2
        {
            return Ok(SearchDecision::Stop {
                reason: "unstable results".into(),
            });
        }

        self.ensure_neighborhood(platform, current_best)?;

        while self.cursor < self.neighborhood_order.len() {
            let change = self.neighborhood_order[self.cursor].clone();
            self.cursor += 1;

            let mut previous = platform.read_param(&change.key)?;
            for c in &current_best.changes {
                if c.key == change.key {
                    previous = c.next.clone();
                }
            }
            if previous == change.next {
                continue;
            }

            let mut changes = current_best.changes.clone();
            if let Some(pos) = changes.iter().position(|c| c.key == change.key) {
                changes[pos] = ParamChange {
                    key: change.key.clone(),
                    previous: changes[pos].previous.clone(),
                    next: change.next.clone(),
                    rationale: change.rationale.clone(),
                };
            } else {
                changes.push(ParamChange {
                    key: change.key.clone(),
                    previous,
                    next: change.next.clone(),
                    rationale: change.rationale.clone(),
                });
            }

            let mut meta = IndexMap::new();
            meta.insert("strategy".into(), "hill_climb".into());
            meta.insert("generation".into(), ctx.generation.to_string());
            meta.insert("seed".into(), self.seed.to_string());

            let candidate = Candidate {
                id: format!("hc-{}-{}", ctx.generation, self.cursor),
                label: changes
                    .iter()
                    .map(|c| format!("{}={}", c.key, c.next.display()))
                    .collect::<Vec<_>>()
                    .join(","),
                changes,
                meta,
            };
            let hash = candidate.content_hash();
            if self.tried.contains(&hash) {
                continue;
            }
            self.tried.push(hash);
            return Ok(SearchDecision::Try(candidate));
        }

        Ok(SearchDecision::Stop {
            reason: "neighborhood exhausted".into(),
        })
    }
}

fn unique_thread_choices(current: usize, logical: usize) -> Vec<usize> {
    let mut v = vec![
        1,
        2,
        current.saturating_sub(2).max(1),
        current,
        (current + 2).min(logical.max(current)),
        logical,
        (logical / 2).max(1),
        (logical * 3 / 4).max(1),
    ];
    v.sort_unstable();
    v.dedup();
    v.into_iter()
        .filter(|&n| n >= 1 && n <= logical.max(1) * 2 && n <= 64)
        .collect()
}
