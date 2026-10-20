//! The screen canvas's changes and the smudge residuals they read (S53): a change that reads a
//! residual the arena lacks waits while that residual is evaluated over frames as background work
//! (`engine_derive.rs`), first in scene order; once nothing it reads is lacking, it is drawn. So
//! an artwork opening shows its paint everywhere but near its smudges while their residuals are
//! evaluated, instead of the paper until all are.

use super::CanvasTarget;
use crate::core::Core;
use crate::error::RenderError;
use crate::frame::FrameRequest;
use crate::geom::PixelRect;
use crate::smudge::Wanted;

impl CanvasTarget {
    /// Whether every residual read near the canvas pixels `near` (what a view tier's tiles read,
    /// `reached`, `around_output`) is in the arena; when one is lacking, goes on evaluating a
    /// residual within what is left of the frame budget instead (while the canvas warms, the one
    /// warming takes, so the two never start each other's over), and the tiles wait (S54: a view
    /// tier over a canvas shown from its display cache fills without a frame evaluating every
    /// residual it reads at once).
    ///
    /// # Errors
    /// As the evaluation's.
    pub(crate) fn residuals_ready(
        &mut self,
        core: &Core,
        request: &FrameRequest<'_>,
        near: &[PixelRect],
    ) -> Result<bool, RenderError> {
        let cache = self
            .scene
            .as_ref()
            .ok_or(RenderError::Invalid("no scene was built"))?;
        let wanted = Wanted::Near(near);
        let scene = (&*cache.scene, request.artwork);
        let filters = (&mut self.filters, &wanted);
        if self.smudges.lacking(core, scene, filters).is_none() {
            return Ok(true);
        }
        let whole = [PixelRect::full(self.size())];
        if self.warming.is_none() || self.derive_lacking(core, request, &whole)?.is_none() {
            self.derive_lacking(core, request, near)?;
        }
        Ok(false)
    }

    /// After a canvas shown complete from its display cache (S54): while nothing waits, the
    /// residuals of the whole canvas are evaluated within the frame budget, as many as there are
    /// smudges at most, so a stroke, a tier or an undo near a smudge later finds them at hand, as
    /// after a fill.
    pub(super) fn warm(
        &mut self,
        core: &Core,
        request: &FrameRequest<'_>,
    ) -> Result<(), RenderError> {
        let Some(done) = self.warming else {
            return Ok(());
        };
        let smudges = (self.scene.as_ref()).map_or(0, |cache| cache.scene.smudges.len());
        let whole = [PixelRect::full(self.size())];
        match self.derive_lacking(core, request, &whole)? {
            Some(true) if done + 1 < smudges => self.warming = Some(done + 1),
            None | Some(true) => self.warming = None,
            Some(false) => {}
        }
        Ok(())
    }

    /// Goes on evaluating the first residual `near` reads that the arena lacks (and fits
    /// whole): `None` when none is lacking (or they do not fit, so `prepare` takes them at
    /// once), else whether it is in the arena now.
    fn derive_lacking(
        &mut self,
        core: &Core,
        request: &FrameRequest<'_>,
        near: &[PixelRect],
    ) -> Result<Option<bool>, RenderError> {
        let cache = self
            .scene
            .as_ref()
            .ok_or(RenderError::Invalid("no scene was built"))?;
        let wanted = Wanted::Near(near);
        let lacking = self.smudges.lacking(
            core,
            (&*cache.scene, request.artwork),
            (&mut self.filters, &wanted),
        );
        let Some(lacking) = lacking else {
            return Ok(None);
        };
        let scene = (&*cache.scene, &cache.ops, cache.serial);
        let deriving = (lacking.first, &lacking.needed, &mut self.filters);
        self.smudges
            .derive(core, scene, request.artwork, deriving)
            .map(Some)
    }
}
