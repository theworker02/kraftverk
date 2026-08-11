//! Platform capability abstraction.
//!
//! All OS-specific branching lives behind the [`Platform`] trait so callers
//! never scatter `cfg(windows)` / `cfg(unix)` checks.

pub mod capabilities;
pub mod host;
pub mod mock;
pub mod native;
pub mod recovery;
pub mod topology;
pub mod tuner;

pub use capabilities::{Capabilities, Capability, FeatureSupport};
pub use host::detect_platform;
pub use mock::MockPlatform;
pub use native::NativePlatform;
pub use recovery::{RecoveryJournal, RecoveryRecord};
pub use topology::{CpuTopology, Topology};
pub use tuner::{ApplyGuard, Platform, TunerEffect};
