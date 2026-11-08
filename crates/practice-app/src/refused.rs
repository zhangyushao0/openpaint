//! `Refused`: why the practice sheet did not do what was asked of it. The runtime logs it; what
//! the user is told is the sheet's notice.

use std::fmt;

use openpaint_app::Availability;
use openpaint_core::CoreError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refused {
    /// Practising is not offered now, for this reason: nothing to act on (or no sheet open), a
    /// stroke being drawn, the session still running.
    Unavailable(Availability),
    /// The artwork refused the fill.
    Core(CoreError),
}

impl fmt::Display for Refused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable(availability) => {
                write!(f, "practising is unavailable: {availability}")
            }
            Self::Core(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for Refused {}

impl From<CoreError> for Refused {
    fn from(error: CoreError) -> Self {
        Self::Core(error)
    }
}
