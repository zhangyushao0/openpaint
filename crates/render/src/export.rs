//! Export: an artwork at its native size (one pixel per canvas pixel), evaluated whole through
//! the single evaluation entry into a target of its own and read back. It shares nothing with
//! the screen's target, so a worker holding an `Arc<Core>` runs it while the screen draws on;
//! since a pixel never depends on which regions are evaluated with it (D16), the result is the
//! screen's canvas pixel for pixel.

use openpaint_core::{Artwork, Color};

use crate::core::Core;
use crate::error::RenderError;
use crate::evaluator::{CanvasTarget, RegionEvaluator};
use crate::frame::FrameRequest;
use crate::geom::PixelRect;
use crate::image::RgbaImage;

impl Core {
    /// `artwork` on `background` paper, every layer composited, as 8-bit RGBA at canvas size.
    /// Waits for the GPU.
    ///
    /// # Errors
    /// `Invalid` when the device cannot hold a target of the canvas size; `Device` when the GPU
    /// fails.
    pub fn export(&self, artwork: &Artwork, background: Color) -> Result<RgbaImage, RenderError> {
        let canvas = artwork.canvas();
        let size = [canvas.width(), canvas.height()];
        let mut target = CanvasTarget::new(self, size)?;
        let request = FrameRequest {
            artwork,
            active: None,
            background,
        };
        RegionEvaluator::new(self, &mut target).evaluate(&request, &[PixelRect::full(size)])?;
        target.read(self)
    }
}
