//! Declared platform capabilities — honest about unsupported features.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeatureSupport {
    Supported,
    Partial,
    Unsupported,
    RequiresPrivilege,
}

impl FeatureSupport {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Supported => "supported",
            Self::Partial => "partial",
            Self::Unsupported => "unsupported",
            Self::RequiresPrivilege => "requires_privilege",
        }
    }

    pub fn is_usable(self) -> bool {
        matches!(self, Self::Supported | Self::Partial)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    pub id: String,
    pub name: String,
    pub support: FeatureSupport,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capabilities {
    pub features: Vec<Capability>,
}

impl Capabilities {
    pub fn get(&self, id: &str) -> Option<&Capability> {
        self.features.iter().find(|c| c.id == id)
    }

    pub fn is_supported(&self, id: &str) -> bool {
        self.get(id).map(|c| c.support.is_usable()).unwrap_or(false)
    }

    pub fn milestone1_safe() -> Self {
        Self {
            features: vec![
                Capability {
                    id: "bench.worker_threads".into(),
                    name: "Benchmark worker thread count".into(),
                    support: FeatureSupport::Supported,
                    notes: "Process-scoped; affects KraftBench parallel workloads only.".into(),
                },
                Capability {
                    id: "bench.rayon_threads".into(),
                    name: "Rayon thread-pool size".into(),
                    support: FeatureSupport::Supported,
                    notes: "Process-scoped rayon pool for parallel benches.".into(),
                },
                Capability {
                    id: "process.priority".into(),
                    name: "Process scheduling priority".into(),
                    support: FeatureSupport::Partial,
                    notes:
                        "Best-effort; may require privileges on some platforms; always rolled back."
                            .into(),
                },
                Capability {
                    id: "process.affinity".into(),
                    name: "CPU affinity mask".into(),
                    support: FeatureSupport::Partial,
                    notes: "Safe subset only; restricted to currently online CPUs.".into(),
                },
                Capability {
                    id: "gpu.clock".into(),
                    name: "GPU clock control".into(),
                    support: FeatureSupport::Unsupported,
                    notes: "Not implemented in Milestone 1.".into(),
                },
                Capability {
                    id: "power.scheme".into(),
                    name: "OS power scheme / plan".into(),
                    support: FeatureSupport::RequiresPrivilege,
                    notes: "Requires kraftverk agent serve (elevated on Windows for powercfg / root on Linux for cpufreq)."
                        .into(),
                },
                Capability {
                    id: "storage.trim".into(),
                    name: "Storage TRIM / optimize".into(),
                    support: FeatureSupport::Unsupported,
                    notes: "Out of scope; Kraftverk is not a disk cleaner.".into(),
                },
                Capability {
                    id: "registry.tweaks".into(),
                    name: "Windows registry performance tweaks".into(),
                    support: FeatureSupport::Unsupported,
                    notes: "Placebo-prone; not in Milestone 1.".into(),
                },
            ],
        }
    }
}
