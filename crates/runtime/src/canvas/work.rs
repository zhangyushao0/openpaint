//! What is open on the canvas: an artwork of the studio's, or a practice sheet standing on its own
//! session (`openpaint_practice_app::SheetSession`, D112). Both products are peers here: the
//! renderer, the workers and the store reach the session of either; the frame's painting intents
//! go through the sheet where one is open.

use openpaint_app::{AppError, Intent, Session};
use openpaint_core::StrokeStyle;
use openpaint_practice_app::SheetSession;

/// What is open on the canvas. Both boxed: clippy's `large_enum_variant` otherwise.
#[derive(Debug)]
pub(crate) enum Work {
    Artwork(Box<Session>),
    Sheet(Box<SheetSession>),
}

impl Work {
    /// The session the open artwork or sheet is drawn in.
    pub(crate) fn session(&self) -> &Session {
        match self {
            Self::Artwork(session) => session,
            Self::Sheet(sheet) => sheet.session(),
        }
    }

    /// The session, for what the workers answer and the store asks; intents go through
    /// `dispatch`.
    pub(crate) fn session_mut(&mut self) -> &mut Session {
        match self {
            Self::Artwork(session) => session,
            Self::Sheet(sheet) => sheet.session_mut(),
        }
    }

    /// The practice sheet, when one is open.
    pub(crate) fn sheet(&self) -> Option<&SheetSession> {
        match self {
            Self::Artwork(_) => None,
            Self::Sheet(sheet) => Some(sheet),
        }
    }

    /// The practice sheet, when one is open.
    pub(crate) fn sheet_mut(&mut self) -> Option<&mut SheetSession> {
        match self {
            Self::Artwork(_) => None,
            Self::Sheet(sheet) => Some(sheet),
        }
    }

    /// One intent of the frame's: through the sheet where one is open.
    pub(crate) fn dispatch(&mut self, intent: Intent) -> Result<(), AppError> {
        match self {
            Self::Artwork(session) => session.dispatch(intent),
            Self::Sheet(sheet) => sheet.dispatch(intent),
        }
    }

    /// The toolbox's style for the strokes to come (a sheet's pen draws instead while it does).
    pub(crate) fn set_brush(&mut self, brush: StrokeStyle) {
        match self {
            Self::Artwork(session) => session.set_brush(brush),
            Self::Sheet(sheet) => sheet.set_brush(brush),
        }
    }

    /// The diameter of the pen a sheet draws with instead of the brush, while it does.
    pub(crate) fn pen(&self) -> Option<f32> {
        self.sheet().and_then(SheetSession::pen)
    }
}
