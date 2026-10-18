//! A reference's evaluation (`RegionEvaluator::dispatch`, S65: the evaluator's one use): the
//! scene made ready, its smudges' residuals and filters' results the evaluator's own, and the
//! whole target evaluated by the evaluator at the piece of the output it holds.

use super::RegionEvaluator;
use crate::error::RenderError;
use crate::frame::FrameRequest;
use crate::geom::PixelRect;
use crate::output::Output;
use crate::scene::ActivePlace;

impl RegionEvaluator<'_> {
    /// Evaluates the whole target as the piece of the output at `output`.
    pub(super) fn dispatch(
        &mut self,
        request: &FrameRequest<'_>,
        output: Output,
    ) -> Result<(), RenderError> {
        let core = self.core;
        let place = ActivePlace::of_request(request.artwork, None);
        self.target.scene(core, request.artwork, place)?;
        let whole = PixelRect::full(self.target.size());
        self.target.draw(core, (request, output), &whole)
    }
}
