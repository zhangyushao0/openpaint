//! The session's side of practice (`practice_sheet.rs`): a practice sheet opens on its shown
//! pair, its strokes keep the pen's own samples (no StreamLine, QuickShape or guide), the pen
//! draws only in the shown exercise's empty frame and every stroke and fill stays inside it, the
//! eyedropper asks the sheet first, and the session record goes to the store after each change.

use openpaint_core::{Artwork, RegionMask, StrokeStyle};
use openpaint_practice::SessionRecord;

use super::Session;
use crate::error::AppError;
use crate::input::{ContactPhase, Device};
use crate::practice::PracticeIntent;
use crate::practice_sheet::{PracticePick, PracticeSheet};
use crate::stroke::StrokeIntent;
use crate::view::Viewport;

impl Session {
    /// The practice sheet `artwork` of session `record` (`before`: the finished sessions of its
    /// stage before it), opening on its shown pair in `viewport`.
    #[must_use]
    pub fn practising(
        artwork: Artwork,
        brush: StrokeStyle,
        (record, before): (SessionRecord, Vec<SessionRecord>),
        viewport: &Viewport,
    ) -> Self {
        let canvas = artwork.canvas();
        let mut session = Self::open(artwork, brush);
        session.strokes.set_raw();
        let practice = PracticeSheet::new(record, before, canvas);
        session.view = practice.opening_view(viewport);
        session.practice = Some(practice);
        session
    }

    /// True on a practice sheet.
    #[must_use]
    pub const fn is_practice(&self) -> bool {
        self.practice.is_some()
    }

    /// Applies a practice intent at `now` (seconds since 1970 UTC, when a check is stored).
    ///
    /// # Errors
    /// `Unavailable` off a practice sheet, for Compare with nothing to compare and for One more
    /// before the session is done.
    pub fn practise(&mut self, intent: PracticeIntent, now: u64) -> Result<(), AppError> {
        self.end_drafts(&crate::intent::Intent::Practice(intent));
        let artwork = self.history.current().clone();
        let practice = self.practice.as_mut().ok_or(AppError::Unavailable {
            action: crate::projection::EditorAction::Practice,
            availability: crate::projection::Availability::NothingLeft,
        })?;
        practice.handle(intent, &artwork, (&mut self.view, now))
    }

    /// What the eyedropper reads at `point` (canvas pixels): on a practice sheet an original is
    /// read only once checked; anywhere else, the canvas.
    pub fn practice_pick(&mut self, point: [f32; 2]) -> PracticePick {
        self.practice
            .as_mut()
            .map_or(PracticePick::Canvas, |practice| practice.pick(point))
    }

    /// The session record for the store, once after each change.
    pub fn practice_save_request(&mut self) -> Option<SessionRecord> {
        self.practice.as_mut()?.save_request()
    }

    /// What strokes and fills stay inside: the selection, and on a practice sheet the shown
    /// exercise's empty frame (only the frame where the two do not meet).
    pub(super) fn clip(&self) -> Option<RegionMask> {
        let selected = self.selection.mask();
        match self.practice.as_ref().and_then(PracticeSheet::clip) {
            Some(frame) => selected
                .and_then(|selection| frame.intersect(selection))
                .or_else(|| Some(frame.clone())),
            None => selected.cloned(),
        }
    }

    /// Whether a practice sheet refuses `stroke`: a pen landing outside the shown exercise's
    /// empty frame, or in a checked one (the sheet's notice says so).
    pub(super) fn refuses(&mut self, stroke: &StrokeIntent) -> bool {
        let (Some(practice), StrokeIntent::Contact(contact)) = (self.practice.as_mut(), stroke)
        else {
            return false;
        };
        let landing = contact.phase == ContactPhase::Down
            && matches!(contact.device, Device::Pen | Device::Eraser);
        match contact.samples.first() {
            Some(first) if landing => !practice.may_draw(first.position),
            _ => false,
        }
    }
}
