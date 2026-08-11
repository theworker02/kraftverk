//! Machine topology description.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuTopology {
    pub physical_cores: usize,
    pub logical_cpus: usize,
    pub packages: usize,
}

impl Default for CpuTopology {
    fn default() -> Self {
        let logical = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);
        Self {
            physical_cores: logical,
            logical_cpus: logical,
            packages: 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Topology {
    pub cpu: CpuTopology,
    pub total_memory_bytes: u64,
}
