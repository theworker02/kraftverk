//! MockPlatform for deterministic optimizer / rollback tests.

use std::collections::HashMap;

use kraftverk_core::candidate::{ParamChange, ParamValue};
use kraftverk_core::error::{Error, Result};
use parking_lot::Mutex;
use rand::Rng;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;

use crate::capabilities::{Capabilities, Capability, FeatureSupport};
use crate::topology::{CpuTopology, Topology};
use crate::tuner::Platform;

#[derive(Debug, Clone)]
pub struct MockConfig {
    pub seed: u64,
    /// Base multiplicative noise stddev around 1.0.
    pub noise_stddev: f64,
    /// If set, applying this key fails.
    pub fail_apply_key: Option<String>,
    /// Simulate thermal throttle after N applies.
    pub throttle_after_applies: Option<usize>,
    pub throttle_multiplier: f64,
}

impl Default for MockConfig {
    fn default() -> Self {
        Self {
            seed: 7,
            noise_stddev: 0.01,
            fail_apply_key: None,
            throttle_after_applies: None,
            throttle_multiplier: 0.85,
        }
    }
}

/// In-memory platform that simulates measurable performance deltas.
pub struct MockPlatform {
    name: String,
    params: HashMap<String, ParamValue>,
    /// Ideal score multipliers for specific param values (key -> (value_display -> mult)).
    effects: HashMap<String, HashMap<String, f64>>,
    topology: Topology,
    capabilities: Capabilities,
    cfg: MockConfig,
    apply_count: usize,
    rng: Mutex<ChaCha8Rng>,
    pub unsupported: Vec<String>,
}

impl MockPlatform {
    pub fn new(cfg: MockConfig) -> Self {
        let logical = 8;
        let mut params = HashMap::new();
        params.insert("bench.worker_threads".into(), ParamValue::Int(4));
        params.insert("bench.rayon_threads".into(), ParamValue::Int(4));
        params.insert(
            "process.priority".into(),
            ParamValue::String("normal".into()),
        );
        params.insert("process.affinity".into(), ParamValue::String("all".into()));

        // Sweet spot at 6 workers → +3%; 8 → +1%; 2 → -2%.
        let mut worker_fx = HashMap::new();
        worker_fx.insert("2".into(), 0.98);
        worker_fx.insert("4".into(), 1.00);
        worker_fx.insert("6".into(), 1.03);
        worker_fx.insert("8".into(), 1.01);

        let mut rayon_fx = HashMap::new();
        rayon_fx.insert("2".into(), 0.99);
        rayon_fx.insert("4".into(), 1.00);
        rayon_fx.insert("6".into(), 1.02);
        rayon_fx.insert("8".into(), 1.015);

        let mut prio_fx = HashMap::new();
        prio_fx.insert("normal".into(), 1.00);
        prio_fx.insert("above_normal".into(), 1.01);
        prio_fx.insert("high".into(), 1.005);

        let mut effects = HashMap::new();
        effects.insert("bench.worker_threads".into(), worker_fx);
        effects.insert("bench.rayon_threads".into(), rayon_fx);
        effects.insert("process.priority".into(), prio_fx);

        let mut caps = Capabilities::milestone1_safe();
        caps.features.push(Capability {
            id: "mock.temp_sensor".into(),
            name: "Mock temperature sensor".into(),
            support: FeatureSupport::Supported,
            notes: "Simulated for tests.".into(),
        });

        let rng = ChaCha8Rng::seed_from_u64(cfg.seed);
        Self {
            name: "mock".into(),
            params,
            effects,
            topology: Topology {
                cpu: CpuTopology {
                    physical_cores: 4,
                    logical_cpus: logical,
                    packages: 1,
                },
                total_memory_bytes: 16 << 30,
            },
            capabilities: caps,
            cfg,
            apply_count: 0,
            rng: Mutex::new(rng),
            unsupported: vec!["gpu.clock".into()],
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(MockConfig::default())
    }

    fn noise_factor(&self) -> f64 {
        let mut rng = self.rng.lock();
        let u: f64 = rng.gen::<f64>() * 2.0 - 1.0;
        1.0 + u * self.cfg.noise_stddev
    }
}

impl Platform for MockPlatform {
    fn name(&self) -> &str {
        &self.name
    }

    fn capabilities(&self) -> Capabilities {
        self.capabilities.clone()
    }

    fn topology(&self) -> Result<Topology> {
        Ok(self.topology.clone())
    }

    fn read_param(&self, key: &str) -> Result<ParamValue> {
        self.params
            .get(key)
            .cloned()
            .ok_or_else(|| Error::unsupported(format!("unknown param {key}")))
    }

    fn apply_change(&mut self, change: &ParamChange) -> Result<()> {
        if let Some(bad) = &self.cfg.fail_apply_key {
            if &change.key == bad {
                return Err(Error::Platform(format!(
                    "simulated apply failure for {bad}"
                )));
            }
        }
        if self.unsupported.iter().any(|k| k == &change.key) {
            return Err(Error::unsupported(format!("{} is unsupported", change.key)));
        }
        self.params.insert(change.key.clone(), change.next.clone());
        self.apply_count += 1;
        Ok(())
    }

    fn verify_change(&self, change: &ParamChange) -> Result<bool> {
        Ok(self.params.get(&change.key) == Some(&change.next))
    }

    fn rollback_change(&mut self, change: &ParamChange) -> Result<()> {
        self.params
            .insert(change.key.clone(), change.previous.clone());
        Ok(())
    }

    fn score_multiplier(&self) -> f64 {
        let mut mult = 1.0;
        for (key, val) in &self.params {
            if let Some(map) = self.effects.get(key) {
                if let Some(m) = map.get(&val.display()) {
                    mult *= *m;
                }
            }
        }
        if let Some(after) = self.cfg.throttle_after_applies {
            if self.apply_count >= after {
                mult *= self.cfg.throttle_multiplier;
            }
        }
        mult * self.noise_factor()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recovery::RecoveryJournal;
    use crate::tuner::ApplyGuard;
    use kraftverk_core::candidate::Candidate;
    use tempfile::tempdir;

    #[test]
    fn apply_verify_rollback() {
        let mut p = MockPlatform::with_defaults();
        let change = ParamChange {
            key: "bench.worker_threads".into(),
            previous: ParamValue::Int(4),
            next: ParamValue::Int(6),
            rationale: "test".into(),
        };
        p.apply_change(&change).unwrap();
        assert!(p.verify_change(&change).unwrap());
        assert_eq!(
            p.read_param("bench.worker_threads").unwrap(),
            ParamValue::Int(6)
        );
        p.rollback_change(&change).unwrap();
        assert_eq!(
            p.read_param("bench.worker_threads").unwrap(),
            ParamValue::Int(4)
        );
    }

    #[test]
    fn guard_rolls_back_on_drop() {
        let dir = tempdir().unwrap();
        let mut journal = RecoveryJournal::open(dir.path().join("journal.json")).unwrap();
        let mut p = MockPlatform::with_defaults();
        let candidate = Candidate {
            id: "c1".into(),
            label: "workers=6".into(),
            changes: vec![ParamChange {
                key: "bench.worker_threads".into(),
                previous: ParamValue::Int(4),
                next: ParamValue::Int(6),
                rationale: "test".into(),
            }],
            meta: Default::default(),
        };
        {
            let guard = ApplyGuard::apply(&mut p, candidate, &mut journal, "exp1").unwrap();
            // Drop without commit → rollback.
            drop(guard);
        }
        assert_eq!(
            p.read_param("bench.worker_threads").unwrap(),
            ParamValue::Int(4)
        );
    }

    #[test]
    fn sweet_spot_improves_score() {
        let mut p = MockPlatform::new(MockConfig {
            noise_stddev: 0.0,
            ..MockConfig::default()
        });
        let base = p.score_multiplier();
        p.apply_change(&ParamChange {
            key: "bench.worker_threads".into(),
            previous: ParamValue::Int(4),
            next: ParamValue::Int(6),
            rationale: "t".into(),
        })
        .unwrap();
        let improved = p.score_multiplier();
        assert!(improved > base);
    }
}
