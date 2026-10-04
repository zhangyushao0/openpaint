//! `RenderError`: every failure the renderer reports. The platform layer turns it into a
//! user-facing message.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderError {
    /// The GPU driver refused or lost something: no adapter, no device, a failed buffer map.
    /// Carries the driver's message; this is a system boundary.
    Device(String),
    /// The caller asked for something the renderer never accepts, such as an empty target.
    Invalid(&'static str),
    /// Derived data would exceed what the device holds in one binding (`limit` bytes): a smudge
    /// residual or the arena of them.
    Admission { what: &'static str, limit: u64 },
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Device(message) => write!(f, "GPU device error: {message}"),
            Self::Invalid(what) => write!(f, "invalid render request: {what}"),
            Self::Admission { what, limit } => write!(f, "{what} exceeds {limit} bytes"),
        }
    }
}

impl std::error::Error for RenderError {}

/// Wraps a wgpu error at the boundary where it enters the renderer.
pub(crate) fn device(error: impl fmt::Display) -> RenderError {
    RenderError::Device(error.to_string())
}
