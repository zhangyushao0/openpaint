//! One evaluation of a request into a target (`RegionEvaluator::dispatch`): the scene made
//! ready, the in-progress stroke's part (S63: a scene routed to the evaluator only), and the
//! screen's waiting tiles and background work.

use super::{EvaluationWork, RegionEvaluator};
use crate::error::RenderError;
use crate::frame::FrameRequest;
use crate::geom::PixelRect;
use crate::output::Output;
use crate::scene::{ActivePlace, Fresh};

impl RegionEvaluator<'_> {
    /// Evaluates `regions` (with the tiles left waiting), and what the in-progress stroke
    /// needs. For one `frame` of the screen (S53): what the stroke needs at once; the regions
    /// join the waiting tiles, of which the frame takes what its background budget allows
    /// (`evaluator_background.rs`), and while a stroke is drawn the waiting tiles wait. Returns
    /// every region it evaluated, in target pixels.
    pub(super) fn dispatch(
        &mut self,
        request: &FrameRequest<'_>,
        output: Output,
        (regions, frame): (&[PixelRect], bool),
    ) -> Result<Vec<PixelRect>, RenderError> {
        let core = self.core;
        // S56: without a stroke the scene holds the next stroke's place on the active layer, so
        // the pen touching down keeps it.
        let place = ActivePlace::of_request(request.artwork, request.active);
        self.target.work = EvaluationWork::default();
        let cache = self.target.scene(core, request.artwork, place)?;
        let (drawn, clip) = (cache.scene.has_active(), cache.scene.active_clip);
        let previewing = cache.previewing();
        let step = self
            .target
            .track
            .advance(request.artwork, request.active, drawn);
        let fresh = request
            .active
            .filter(|_| drawn)
            .map_or_else(Fresh::default, |active| {
                Fresh::new(active, step.kept.start, clip)
            });
        let screen = frame && output == Output::CANVAS;
        let background = screen && fresh.ops.is_empty() && !previewing;
        let mut all = regions.to_vec();
        if background {
            self.target.waiting.add(regions);
            all.clear();
        } else if !screen || fresh.ops.is_empty() {
            // Taken whole, or (a preview's frames, S35) shared as S49 shared every frame.
            all.extend(self.target.waiting.take_all());
        }
        let later = all.len();
        all.extend(step.redraw.into_iter().chain(step.fresh_region));
        all.extend(self.target.smudged(core, request, drawn)?);
        let mut evaluated = if background {
            self.target.background(core, request)?
        } else {
            Vec::new()
        };
        if all.is_empty() && !step.reset {
            return Ok(evaluated);
        }
        let now = (screen && previewing && fresh.ops.is_empty()).then(|| &all[later..]);
        let drawing = (request, (output, step.reset));
        let drawn = self
            .target
            .draw(core, drawing, fresh, (&all, now))?
            .unwrap_or(all);
        self.target.waiting.remove(&drawn);
        evaluated.extend(drawn);
        Ok(evaluated)
    }
}
