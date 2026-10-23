//! Layer sampling: one layer's own pixels (or its mask's grey) at native resolution, for a fill
//! to flood over, a transform to lift and the layer panel's pictures. Like export it draws into a
//! target of its own, so a worker holding an `Arc<Core>` runs it while the screen draws on: an
//! export of the layer's own scene at 1x (`export_raster.rs`).

use openpaint_core::{Artwork, LayerId};

use crate::core::Core;
use crate::error::RenderError;
use crate::export_raster::RasterExport;
use crate::image::RgbaImage;
use crate::readback::read_rect;
use crate::scene::Scene;

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

    /// A small picture of `layer` alone: what `layer_pixels`, or with `mask`
    /// `mask_pixels`, gives, reduced on the GPU to at most `max` pixels (`thumbnail_size`, box
    /// means as `thumbnail`); only the small picture is read back. Made on a worker while the
    /// screen draws on, so each part of its drawing is waited for before the next. Waits for
    /// the GPU.
    ///
    /// # Errors
    /// As `layer_pixels`.
    pub fn layer_thumbnail(
        &self,
        artwork: &Artwork,
        (layer, mask): (LayerId, bool),
        max: [u32; 2],
    ) -> Result<RgbaImage, RenderError> {
        let sampled = self.sampled(artwork, (layer, mask), true)?;
        self.thumbnail_of(sampled.texture(), max)
    }

    fn sample(
        &self,
        artwork: &Artwork,
        layer: LayerId,
        mask: bool,
    ) -> Result<RgbaImage, RenderError> {
        let sampled = self.sampled(artwork, (layer, mask), false)?;
        let canvas = artwork.canvas();
        read_rect(
            &self.gpu,
            sampled.texture(),
            [0, 0],
            [canvas.width(), canvas.height()],
        )
    }

    /// The layer's (or its mask's) pixels drawn into a target of the canvas's size; `paced`,
    /// each part waited for before the next.
    fn sampled(
        &self,
        artwork: &Artwork,
        (layer, mask): (LayerId, bool),
        paced: bool,
    ) -> Result<RasterExport, RenderError> {
        let scene = Scene::layer(artwork, layer, mask)?;
        let canvas = artwork.canvas();
        let size = [canvas.width(), canvas.height()];
        let mut export = RasterExport::new(self, scene, (size, paced))?;
        // On nothing; a mask's grey on white.
        let background = if mask { [1.0; 4] } else { [0.0; 4] };
        export.draw(self, (artwork, 1, [0, 0]), size, (background, paced))?;
        Ok(export)
    }
}
