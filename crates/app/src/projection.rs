//! `SessionProjection`: what the UI may offer right now. The UI greys out what is unavailable
//! and never works out the rules itself.

use std::fmt;

/// Something the user can ask for whose availability depends on the session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorAction {
    Undo,
    Redo,
}

/// Whether an action can run now, and why not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    Available,
    /// There is nothing to act on: no earlier version to undo to, none to redo.
    NothingLeft,
    /// A stroke is being drawn; history waits until it is committed or cancelled, so the
    /// stroke can neither be lost nor land on a version the user no longer sees.
    StrokeInProgress,
}

impl Availability {
    #[must_use]
    pub const fn is_available(self) -> bool {
        matches!(self, Self::Available)
    }
}

/// A snapshot of the session for one UI frame. The default is no open session: nothing is
/// available.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionProjection {
    pub(crate) undo: Availability,
    pub(crate) redo: Availability,
}

impl SessionProjection {
    #[must_use]
    pub const fn availability(&self, action: EditorAction) -> Availability {
        match action {
            EditorAction::Undo => self.undo,
            EditorAction::Redo => self.redo,
        }
    }
}

impl Default for SessionProjection {
    fn default() -> Self {
        Self {
            undo: Availability::NothingLeft,
            redo: Availability::NothingLeft,
        }
    }
}

impl fmt::Display for EditorAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Undo => "undo",
            Self::Redo => "redo",
        })
    }
}

impl fmt::Display for Availability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Available => "available",
            Self::NothingLeft => "nothing left",
            Self::StrokeInProgress => "a stroke is in progress",
        })
    }
}
