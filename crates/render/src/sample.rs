//! Layer sampling: one layer's own pixels (or its mask's grey) at native resolution, for a fill
//! to flood over. Like export it evaluates into a target of its own through the single
//! evaluator, so a worker holding an `Arc<Core>` runs it while the screen draws on.

use openpaint_core::{Artwork, LayerId};

use crate::core::Core;
use crate::error::RenderError;
use crate::evaluator::{CanvasTarget, RegionEvaluator};
use crate::image::RgbaImage;

impl Core {
    /// `layer` of `artwork` alone, its paint at full opacity without its mask and whether it is
    /// shown or not, as premultiplied 8-bit RGBA at canvas size; where it has no paint,
    /// transparent black. Waits for the GPU.
    ///
    /// # Errors
    /// `Invalid` when the layer is not in the artwork or the device cannot hold a target of
    /// the canvas size; `Device` when the GPU fails.
    pub fn layer_pixels(
        &self,
        artwork: &Artwork,
        layer: LayerId,
    ) -> Result<RgbaImage, RenderError> {
        self.sample(artwork, layer, false)
    }

    /// `layer`'s mask alone as what its strokes and fills left (not inverted): opaque grey,
    /// white where nothing was painted. A layer without a mask gives its paint, as
    /// `layer_pixels`. Waits for the GPU.
    ///
    /// # Errors
    /// As `layer_pixels`.
    pub fn mask_pixels(&self, artwork: &Artwork, layer: LayerId) -> Result<RgbaImage, RenderError> {
        self.sample(artwork, layer, true)
    }

    fn sample(
        &self,
        artwork: &Artwork,
        layer: LayerId,
        mask: bool,
    ) -> Result<RgbaImage, RenderError> {
        let canvas = artwork.canvas();
        let mut target = CanvasTarget::new(self, [canvas.width(), canvas.height()])?;
        RegionEvaluator::new(self, &mut target).evaluate_layer(artwork, layer, mask)?;
        target.read(self)
    }
}
