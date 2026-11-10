//! The session's side of its mode (`mode.rs`): a session opened with one keeps its view as the
//! mode opens it, its strokes keep the pen's own samples when the mode asks, a pen landing where
//! the mode refuses draws nothing, every stroke and fill stays inside the mode's clip (and the
//! selection; a fill's region is found within the clip's box), the next stroke takes the mode's style instead of the toolbox's brush, a fill
//! beginning is the mode's to know, and the mode's layers are amended into the version shown
//! after every intent. The composition root reaches the mode by its type.

use openpaint_core::{Artwork, RegionMask, StrokeStyle};

use super::Session;
use crate::input::{ContactPhase, Device};
use crate::mode::{Lent, Mode};
use crate::stroke::StrokeIntent;
use crate::view::CanvasView;

impl Session {
    /// `artwork` as the store holds it, with an empty history, held by `mode`, showing as `view`;
    /// `brush` is the toolbox's.
    #[must_use]
    pub fn with_mode(
        artwork: Artwork,
        brush: StrokeStyle,
        mode: Box<dyn Mode>,
        view: CanvasView,
    ) -> Self {
        let mut session = Self::open(artwork, brush);
        if mode.raw() {
            session.strokes.set_raw();
        }
        session.view = view;
        session.mode = Some(mode);
        session.restyle();
        session.mode_layers();
        session
    }

    /// The mode the session holds, if it is a `T`.
    #[must_use]
    pub fn mode<T: Mode>(&self) -> Option<&T> {
        let mode: &dyn std::any::Any = self.mode.as_deref()?;
        mode.downcast_ref()
    }

    /// The mode the session holds, if it is a `T`, to change what only the mode keeps (nothing
    /// of the artwork, the view or the next stroke's style).
    pub fn mode_mut<T: Mode>(&mut self) -> Option<&mut T> {
        let mode: &mut dyn std::any::Any = self.mode.as_deref_mut()?;
        mode.downcast_mut()
    }

    /// `f` given the mode, if it is a `T`, and what it may use of the session (`Lent`). As any
    /// edit, a shape, adjustment or transform under way ends first; afterwards the next stroke
    /// takes the mode's style and its layers are amended.
    pub fn lend<T: Mode, R>(&mut self, f: impl FnOnce(&mut T, &mut Lent<'_>) -> R) -> Option<R> {
        self.mode::<T>()?;
        self.end_every_draft();
        let drawing = self.strokes.active().is_some();
        let mode: &mut dyn std::any::Any = self.mode.as_deref_mut()?;
        let mode = mode.downcast_mut::<T>()?;
        let mut lent = Lent {
            history: &mut self.history,
            selection: self.selection.mask(),
            drawing,
            view: &mut self.view,
            fills: &mut self.fills,
        };
        let result = f(mode, &mut lent);
        self.restyle();
        self.mode_layers();
        Some(result)
    }

    /// The toolbox's style, which strokes start with unless the mode draws with its own.
    #[must_use]
    pub const fn brush(&self) -> &StrokeStyle {
        &self.brush
    }

    /// The mode's layers as it keeps them, amended into the version shown: never a step of their
    /// own, never while a stroke is drawn.
    pub(super) fn mode_layers(&mut self) {
        if self.strokes.active().is_some() {
            return;
        }
        let amended = (self.mode.as_ref()).and_then(|mode| mode.amended(self.history.current()));
        if let Some(artwork) = amended {
            self.history.amend(artwork);
        }
    }

    /// The next stroke's style: the mode's, else the toolbox's brush.
    pub(super) fn restyle(&mut self) {
        let style = (self.mode.as_ref()).and_then(|mode| mode.style(&self.brush));
        self.strokes
            .set_style(style.unwrap_or_else(|| self.brush.clone()));
    }

    /// What strokes and fills stay inside: the selection, and the mode's clip (only the clip
    /// where the two do not meet).
    pub(super) fn clip(&self) -> Option<RegionMask> {
        let selected = self.selection.mask();
        match self.mode.as_ref().and_then(|mode| mode.clip()) {
            Some(clip) => selected
                .and_then(|selection| clip.intersect(selection))
                .or_else(|| Some(clip.clone())),
            None => selected.cloned(),
        }
    }

    /// The bounding box of the mode's clip, `(origin, size)` in canvas pixels: a fill's region is
    /// found within it, so a flood never leaks round the clip through the paper outside.
    pub(super) fn mode_bounds(&self) -> Option<([u32; 2], [u32; 2])> {
        let clip = self.mode.as_ref()?.clip()?;
        Some((clip.origin(), clip.size()))
    }

    /// Whether the mode refuses `stroke`: a pen landing where it may not.
    pub(super) fn refuses(&mut self, stroke: &StrokeIntent) -> bool {
        let (Some(mode), StrokeIntent::Contact(contact)) = (self.mode.as_mut(), stroke) else {
            return false;
        };
        let landing = contact.phase == ContactPhase::Down
            && matches!(contact.device, Device::Pen | Device::Eraser);
        match contact.samples.first() {
            Some(first) if landing => !mode.lands(first.position),
            _ => false,
        }
    }

    /// A fill begins (dragged from the colour button): the mode knows.
    pub(super) fn filling_frame(&mut self) {
        if let Some(mode) = self.mode.as_mut() {
            mode.laid();
        }
    }
}
