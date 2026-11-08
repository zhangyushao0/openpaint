//! A mode: something the open artwork is for beyond painting, which the session asks at a few
//! fixed points and otherwise leaves alone (D112). The session holds at most one; the composition
//! root gives it with the artwork (`Session::with_mode`) and reaches it again by its type
//! (`Session::mode`, `mode_mut`, `lend`). A mode never sees the session's insides: what it may
//! change of the artwork and the view goes through `Lent`.

use std::any::Any;
use std::fmt;

use openpaint_core::{Artwork, RegionMask, StrokeStyle};

use crate::fill::Fills;
use crate::history::History;
use crate::view::CanvasView;

/// What the session asks of the mode it holds.
pub trait Mode: Any + Send + fmt::Debug {
    /// Strokes keep the pen's own samples: no StreamLine, QuickShape or guide. Asked once, as the
    /// session opens.
    fn raw(&self) -> bool;

    /// A pen (or the eraser) lands at `at`, canvas pixels; false refuses the stroke it begins.
    fn lands(&mut self, at: [f32; 2]) -> bool;

    /// What strokes and fills stay inside, beside the selection.
    fn clip(&self) -> Option<&RegionMask>;

    /// The next stroke's style instead of the toolbox's `brush`, if the mode draws with its own.
    fn style(&self, brush: &StrokeStyle) -> Option<StrokeStyle>;

    /// The layers the mode keeps, amended into `current` (never a step of their own); none when
    /// nothing changes. Never asked while a stroke is drawn.
    fn amended(&self, current: &Artwork) -> Option<Artwork>;

    /// A fill began (dragged from the colour button).
    fn laid(&mut self);
}

/// What a mode may use of the session while it acts (`Session::lend`): the artwork and selection
/// to read, the view to move, a version to record and the history to cut.
pub struct Lent<'a> {
    pub(crate) history: &'a mut History,
    pub(crate) selection: Option<&'a RegionMask>,
    pub(crate) drawing: bool,
    pub(crate) view: &'a mut CanvasView,
    /// A fill being placed, dropped with a version recorded or the history cut.
    pub(crate) fills: &'a mut Fills,
}

impl Lent<'_> {
    /// The current version.
    #[must_use]
    pub fn artwork(&self) -> &Artwork {
        self.history.current()
    }

    /// The pixels selected now, which strokes and fills stay inside.
    #[must_use]
    pub const fn selection(&self) -> Option<&RegionMask> {
        self.selection
    }

    /// True while a stroke is being drawn.
    #[must_use]
    pub const fn drawing(&self) -> bool {
        self.drawing
    }

    /// How the artwork shows.
    pub const fn view(&mut self) -> &mut CanvasView {
        self.view
    }

    /// `artwork` as the next version, one step of history.
    pub fn record(&mut self, artwork: Artwork) {
        self.fills.interrupt();
        self.history.record(artwork);
    }

    /// Undo and redo reach no version before the current one from now on (a fill still being
    /// found is dropped with them).
    pub fn keep_only_current(&mut self) {
        self.fills.interrupt();
        self.history.keep_only_current();
    }
}
