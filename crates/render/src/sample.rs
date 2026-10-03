//! Layer sampling: one layer's own pixels at native resolution, for a fill to flood over. Like
//! export it evaluates into a target of its own through the single evaluator, so a worker
//! holding an `Arc<Core>` runs it while the screen draws on.

use openpaint_core::{Artwork, LayerId};

use crate::core::Core;
use crate::error::RenderError;
use crate::evaluator::{CanvasTarget, RegionEvaluator};
use crate::image::RgbaImage;

impl Core {
    /// `layer` of `artwork` alone, at full opacity and whether it is shown or not, as
    /// premultiplied 8-bit RGBA at canvas size; where it has no paint, transparent black.
    /// Waits for the GPU.
    ///
    /// # Errors
    /// `Invalid` when the layer is not in the artwork or the device cannot hold a target of
    /// the canvas size; `Device` when the GPU fails.
    pub fn layer_pixels(
        &self,
        artwork: &Artwork,
        layer: LayerId,
    ) -> Result<RgbaImage, RenderError> {
        let canvas = artwork.canvas();
        let mut target = CanvasTarget::new(self, [canvas.width(), canvas.height()])?;
        RegionEvaluator::new(self, &mut target).evaluate_layer(artwork, layer)?;
        target.read(self)
    }
}
