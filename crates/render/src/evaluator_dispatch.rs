//! One evaluation of a request into a target (`RegionEvaluator::dispatch`): the scene made
//! ready, the in-progress stroke's part, the screen's waiting tiles and background work, and the
//! snapshot the evaluation resumes the stroke's layer from.

use super::{CanvasTarget, EvaluationWork, RegionEvaluator};
use crate::active_snapshot::Use;
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
        let cache = self.target.scene(core, request.artwork, place.clone())?;
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
            self.target.background(core, request, all.is_empty())?
        } else {
            Vec::new()
        };
        if all.is_empty() && !step.reset {
            return Ok(evaluated);
        }
        let stroke = (request.active.is_some() && drawn).then_some(place);
        let held = self.target.snapshot_use(output, stroke, previewing);
        let now = (screen && previewing && fresh.ops.is_empty()).then(|| &all[later..]);
        let drawing = (request, (output, step.reset));
        let drawn = self
            .target
            .draw(core, drawing, (fresh, held), (&all, now))?
            .unwrap_or(all);
        self.target.waiting.remove(&drawn);
        evaluated.extend(drawn);
        Ok(evaluated)
    }
}

impl CanvasTarget {
    /// How an evaluation at `output` uses the snapshot of the in-progress stroke's layer: the
    /// stroke drawn on the canvas resumes its layer from it (S48); so does what an evaluation
    /// without one needs (a lift's last spans) while the snapshot holds that layer's content but
    /// for strokes appended (S56); a preview has its own.
    fn snapshot_use(
        &self,
        output: Output,
        stroke: Option<ActivePlace>,
        previewing: bool,
    ) -> Option<Use> {
        if output != Output::CANVAS || !self.bindings.keeps {
            return None;
        }
        match stroke {
            Some(place) => Some(Use {
                place: (place.layer, place.on_mask),
                drawing: true,
            }),
            None if !previewing => self.held.as_ref().map(|snapshot| Use {
                place: snapshot.place(),
                drawing: false,
            }),
            None => None,
        }
    }
}
