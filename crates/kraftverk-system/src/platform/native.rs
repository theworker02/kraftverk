//! Native platform: process-scoped safe tunables for Windows and Linux.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

use kraftverk_core::candidate::{ParamChange, ParamValue};
use kraftverk_core::error::{Error, Result};
use tracing::info;
#[allow(unused_imports)]
use tracing::warn;

use crate::capabilities::Capabilities;
use crate::topology::{CpuTopology, Topology};
use crate::tuner::Platform;

/// Process-local tunables that KraftBench reads.
static WORKER_THREADS: AtomicUsize = AtomicUsize::new(0);
static RAYON_THREADS: AtomicUsize = AtomicUsize::new(0);

/// Priority labels we support in Milestone 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PriorityLevel {
    Normal,
    AboveNormal,
    High,
}

impl PriorityLevel {
    fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "normal" => Some(Self::Normal),
            "above_normal" | "above-normal" => Some(Self::AboveNormal),
            "high" => Some(Self::High),
            _ => None,
        }
    }

    #[allow(dead_code)]
    fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::AboveNormal => "above_normal",
            Self::High => "high",
        }
    }
}

pub struct NativePlatform {
    name: String,
    params: HashMap<String, ParamValue>,
    topology: Topology,
    /// Saved OS priority so we can restore accurately.
    saved_priority: Option<PriorityLevel>,
    /// Saved affinity mask description.
    saved_affinity: Option<String>,
}

impl NativePlatform {
    pub fn detect() -> Result<Self> {
        let logical = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);
        let default_workers = logical.clamp(1, 16);

        WORKER_THREADS.store(default_workers, Ordering::SeqCst);
        RAYON_THREADS.store(default_workers, Ordering::SeqCst);

        let mut params = HashMap::new();
        params.insert(
            "bench.worker_threads".into(),
            ParamValue::Int(default_workers as i64),
        );
        params.insert(
            "bench.rayon_threads".into(),
            ParamValue::Int(default_workers as i64),
        );
        params.insert(
            "process.priority".into(),
            ParamValue::String("normal".into()),
        );
        params.insert("process.affinity".into(), ParamValue::String("all".into()));

        let name = if cfg!(windows) {
            "windows"
        } else if cfg!(target_os = "linux") {
            "linux"
        } else {
            "unsupported_os"
        };

        Ok(Self {
            name: name.into(),
            params,
            topology: Topology {
                cpu: CpuTopology {
                    physical_cores: logical,
                    logical_cpus: logical,
                    packages: 1,
                },
                total_memory_bytes: 0,
            },
            saved_priority: None,
            saved_affinity: None,
        })
    }

    pub fn worker_threads() -> usize {
        let v = WORKER_THREADS.load(Ordering::SeqCst);
        if v == 0 {
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1)
        } else {
            v
        }
    }

    pub fn rayon_threads() -> usize {
        let v = RAYON_THREADS.load(Ordering::SeqCst);
        if v == 0 {
            Self::worker_threads()
        } else {
            v
        }
    }

    fn apply_priority(&mut self, level: PriorityLevel) -> Result<()> {
        #[cfg(windows)]
        {
            windows_set_priority(level)
        }
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            unix_set_priority(level)
        }
        #[cfg(target_os = "macos")]
        {
            let _ = level;
            warn!("process.priority: best-effort unsupported detail on macOS in M1");
            Ok(())
        }
        #[cfg(not(any(windows, unix)))]
        {
            let _ = level;
            Err(Error::unsupported(
                "process.priority not available on this OS",
            ))
        }
    }

    fn apply_affinity(&mut self, spec: &str) -> Result<()> {
        #[cfg(windows)]
        {
            windows_set_affinity(spec, self.topology.cpu.logical_cpus)
        }
        #[cfg(unix)]
        {
            unix_set_affinity(spec, self.topology.cpu.logical_cpus)
        }
        #[cfg(not(any(windows, unix)))]
        {
            let _ = spec;
            Err(Error::unsupported(
                "process.affinity not available on this OS",
            ))
        }
    }
}

impl Platform for NativePlatform {
    fn name(&self) -> &str {
        &self.name
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::milestone1_safe()
    }

    fn topology(&self) -> Result<Topology> {
        Ok(self.topology.clone())
    }

    fn read_param(&self, key: &str) -> Result<ParamValue> {
        self.params
            .get(key)
            .cloned()
            .ok_or_else(|| Error::unsupported(format!("unknown or unsupported param {key}")))
    }

    fn apply_change(&mut self, change: &ParamChange) -> Result<()> {
        match change.key.as_str() {
            "bench.worker_threads" => {
                let n = change
                    .next
                    .as_usize()
                    .ok_or_else(|| Error::InvalidConfig("worker_threads must be int".into()))?;
                if n == 0 || n > self.topology.cpu.logical_cpus * 2 {
                    return Err(Error::InvalidConfig(format!(
                        "worker_threads {n} out of safe range"
                    )));
                }
                WORKER_THREADS.store(n, Ordering::SeqCst);
                self.params.insert(change.key.clone(), change.next.clone());
                info!(worker_threads = n, "applied bench.worker_threads");
                Ok(())
            }
            "bench.rayon_threads" => {
                let n = change
                    .next
                    .as_usize()
                    .ok_or_else(|| Error::InvalidConfig("rayon_threads must be int".into()))?;
                if n == 0 || n > self.topology.cpu.logical_cpus * 2 {
                    return Err(Error::InvalidConfig(format!(
                        "rayon_threads {n} out of safe range"
                    )));
                }
                RAYON_THREADS.store(n, Ordering::SeqCst);
                // KraftBench builds a local rayon pool per suite from this value.
                self.params.insert(change.key.clone(), change.next.clone());
                info!(rayon_threads = n, "applied bench.rayon_threads");
                Ok(())
            }
            "process.priority" => {
                let s = match &change.next {
                    ParamValue::String(s) => s.as_str(),
                    _ => {
                        return Err(Error::InvalidConfig(
                            "process.priority must be string".into(),
                        ))
                    }
                };
                let level = PriorityLevel::parse(s).ok_or_else(|| {
                    Error::InvalidConfig(format!("unsupported priority level {s}"))
                })?;
                if self.saved_priority.is_none() {
                    self.saved_priority = Some(PriorityLevel::Normal);
                }
                self.apply_priority(level)?;
                self.params.insert(change.key.clone(), change.next.clone());
                Ok(())
            }
            "process.affinity" => {
                let s = match &change.next {
                    ParamValue::String(s) => s.clone(),
                    _ => {
                        return Err(Error::InvalidConfig(
                            "process.affinity must be string".into(),
                        ))
                    }
                };
                if self.saved_affinity.is_none() {
                    self.saved_affinity = Some("all".into());
                }
                self.apply_affinity(&s)?;
                self.params
                    .insert(change.key.clone(), ParamValue::String(s));
                Ok(())
            }
            other => Err(Error::unsupported(format!(
                "parameter '{other}' is not applied in Milestone 1 native platform"
            ))),
        }
    }

    fn verify_change(&self, change: &ParamChange) -> Result<bool> {
        Ok(self.params.get(&change.key) == Some(&change.next))
    }

    fn rollback_change(&mut self, change: &ParamChange) -> Result<()> {
        let mut restore = change.clone();
        std::mem::swap(&mut restore.previous, &mut restore.next);
        // previous is now in next — apply it.
        let rollback = ParamChange {
            key: change.key.clone(),
            previous: change.next.clone(),
            next: change.previous.clone(),
            rationale: "rollback".into(),
        };
        self.apply_change(&rollback)?;
        Ok(())
    }
}

#[cfg(windows)]
fn windows_set_priority(level: PriorityLevel) -> Result<()> {
    use windows_sys::Win32::System::Threading::{
        GetCurrentProcess, SetPriorityClass, ABOVE_NORMAL_PRIORITY_CLASS, HIGH_PRIORITY_CLASS,
        NORMAL_PRIORITY_CLASS,
    };
    let class = match level {
        PriorityLevel::Normal => NORMAL_PRIORITY_CLASS,
        PriorityLevel::AboveNormal => ABOVE_NORMAL_PRIORITY_CLASS,
        PriorityLevel::High => HIGH_PRIORITY_CLASS,
    };
    let ok = unsafe { SetPriorityClass(GetCurrentProcess(), class) };
    if ok == 0 {
        Err(Error::Platform(
            "SetPriorityClass failed (may need privileges for HIGH)".into(),
        ))
    } else {
        Ok(())
    }
}

#[cfg(windows)]
fn windows_set_affinity(spec: &str, logical: usize) -> Result<()> {
    use windows_sys::Win32::System::Threading::{
        GetCurrentProcess, GetCurrentProcessorNumber, SetProcessAffinityMask,
    };
    let mask = parse_affinity_mask(spec, logical)?;
    let ok = unsafe { SetProcessAffinityMask(GetCurrentProcess(), mask as usize) };
    if ok == 0 {
        // GetCurrentProcessorNumber referenced to keep import useful on older SDKs.
        let _ = unsafe { GetCurrentProcessorNumber() };
        Err(Error::Platform("SetProcessAffinityMask failed".into()))
    } else {
        Ok(())
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
fn unix_set_priority(level: PriorityLevel) -> Result<()> {
    // Nice values: normal=0, above_normal=-5, high=-10. May fail without privileges.
    let nice = match level {
        PriorityLevel::Normal => 0,
        PriorityLevel::AboveNormal => -5,
        PriorityLevel::High => -10,
    };
    let rc = unsafe { libc::setpriority(libc::PRIO_PROCESS, 0, nice) };
    if rc != 0 {
        warn!(
            nice,
            "setpriority failed; continuing without elevated priority"
        );
        // Soft-fail: treat as applied in our bookkeeping but note limitation.
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn unix_set_priority(_level: PriorityLevel) -> Result<()> {
    Ok(())
}

#[cfg(unix)]
fn unix_set_affinity(spec: &str, logical: usize) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        let mask = parse_affinity_mask(spec, logical)?;
        unsafe {
            let mut set: libc::cpu_set_t = std::mem::zeroed();
            for i in 0..logical.min(64) {
                if mask & (1u64 << i) != 0 {
                    libc::CPU_SET(i, &mut set);
                }
            }
            let rc = libc::sched_setaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &set);
            if rc != 0 {
                return Err(Error::Platform("sched_setaffinity failed".into()));
            }
        }
        Ok(())
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (spec, logical);
        warn!("process.affinity unsupported on this unix target in M1");
        Ok(())
    }
}

fn parse_affinity_mask(spec: &str, logical: usize) -> Result<u64> {
    let logical = logical.min(64);
    match spec {
        "all" => Ok(if logical >= 64 {
            u64::MAX
        } else {
            (1u64 << logical) - 1
        }),
        "even" => {
            let mut m = 0u64;
            for i in (0..logical).step_by(2) {
                m |= 1u64 << i;
            }
            Ok(m)
        }
        "odd" => {
            let mut m = 0u64;
            for i in (1..logical).step_by(2) {
                m |= 1u64 << i;
            }
            Ok(m)
        }
        "first_half" => {
            let half = (logical / 2).max(1);
            Ok((1u64 << half) - 1)
        }
        other => Err(Error::InvalidConfig(format!(
            "unsupported affinity spec '{other}' (use all|even|odd|first_half)"
        ))),
    }
}
