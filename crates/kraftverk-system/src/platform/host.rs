//! Platform detection entry point.

use kraftverk_core::error::Result;

use crate::native::NativePlatform;

/// Detect and construct the host platform implementation.
pub fn detect_platform() -> Result<NativePlatform> {
    NativePlatform::detect()
}
