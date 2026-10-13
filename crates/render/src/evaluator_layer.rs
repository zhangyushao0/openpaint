//! One layer evaluated alone into a target: what a fill samples, a transform lifts and the layer
//! panel's pictures show.

use openpaint_core::{Artwork, LayerId};

use super::RegionEvaluator;
use crate::affine::Affine;
use crate::error::RenderError;
use crate::geom::PixelRect;
use crate::output::Output;
use crate::plan::Plan;
use crate::program::{Dispatch, scene_ops};
use crate::scene::{Fresh, Scene};
use crate::smudge::Wanted;

/// Rows evaluated at once: each band needs only the smudge residuals near it in the arena (S51).
const BAND_ROWS: i32 = 512;

impl RegionEvaluator<'_> {
    /// Evaluates `layer` of `artwork` alone over the whole target, at full opacity, on nothing
    /// (transparent black), whether the layer is shown or not: premultiplied layer pixels for a
    /// fill to sample. With `mask`, its mask alone on white: the mask's grey, opaque. The
    /// target then holds no frame of the artwork. It goes in bands of rows, each with the
    /// residuals near it (S51); a pixel never depends on which pixels are evaluated with it.
    pub(crate) fn evaluate_layer(
        &mut self,
        artwork: &Artwork,
        layer: LayerId,
        mask: bool,
    ) -> Result<(), RenderError> {
        let size = self.target.size();
        let canvas = artwork.canvas();
        if size != [canvas.width(), canvas.height()] {
            return Err(RenderError::Invalid(
                "the target's size differs from the canvas size",
            ));
        }
        let scene = Scene::layer(artwork, layer, mask)?;
        let ops = scene_ops(&self.core.gpu, &scene, None);
        let target = (size, Output::CANVAS);
        self.target.scene = None;
        self.target.track.forget();
        let smudges = &mut self.target.smudges;
        smudges.end_active();
        let filters = &mut self.target.filters;
        let full = PixelRect::full(size);
        let mut top = 0;
        while top < full.max[1] {
            let band = PixelRect {
                min: [0, top],
                max: [full.max[0], (top + BAND_ROWS).min(full.max[1])],
            };
            top = band.max[1];
            let wanted = Wanted::Near(std::slice::from_ref(&band));
            smudges.prepare(self.core, (&scene, &ops), artwork, (&mut *filters, &wanted))?;
            let plan = Plan::build(&scene, &Fresh::default(), target, &[band]);
            self.core.evaluate.dispatch(
                &self.core.gpu,
                &mut self.target.bindings,
                &Dispatch {
                    ops: &ops,
                    fresh: &[],
                    plan: &plan,
                    only: None,
                    output: Output::CANVAS,
                    background: if mask { [1.0; 4] } else { [0.0; 4] },
                    reset: false,
                    residuals: smudges.residuals(),
                    base: Affine::IDENTITY,
                    snapshot: None,
                },
            )?;
        }
        Ok(())
    }
}
