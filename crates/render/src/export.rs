//! Export: an artwork at 1, 2, 4 or 8 output pixels per canvas pixel (the app offers up to 4), a
//! band of rows at a time, each band in pieces at most `Banding::columns` wide, read back and
//! handed on, so a 4x export never holds the whole picture on the GPU. The raster core draws it
//! (`export_raster.rs`) with targets of its own, sharing nothing with the screen's, so a worker
//! holding an `Arc<Core>` runs it while the screen draws on. Since a pixel never depends on which regions are
//! drawn with it, bands equal one piece; at 1x it is the screen's canvas pixel for pixel.

use std::time::Duration;

use openpaint_core::{Artwork, Color};

use crate::core::Core;
use crate::error::RenderError;
use crate::export_raster::RasterExport;
use crate::image::RgbaImage;
use crate::output::Output;
use crate::scene::Scene;
use crate::scene_reuse::Earlier;

/// How an export is cut: output rows per band (what the encoder takes at once) and the widest
/// piece evaluated at once (the target's width).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Banding {
    pub rows: u32,
    pub columns: u32,
}

impl Banding {
    /// 512 rows of pieces at most 4096 wide: an 8 MB target, within every device's limits.
    pub const DEFAULT: Self = Self {
        rows: 512,
        columns: 4096,
    };
}

/// The bands of one export, top first; each is the full output width.
pub struct Bands<'a> {
    core: &'a Core,
    artwork: Artwork,
    background: Color,
    scale: u32,
    size: [u32; 2],
    banding: Banding,
    raster: Box<RasterExport>,
    /// The first output row of the next band; the height once done or failed.
    next: u32,
    /// Time spent planning and submitting pieces (the scene, smudges and plans on the CPU), and
    /// waiting for them to come back (the GPU and the readback).
    planned: Duration,
    waited: Duration,
}

impl Core {
    /// The bands of `artwork` on `background` paper at `scale` output pixels per canvas pixel,
    /// every layer composited, cut by `banding`. Evaluation happens as the bands are taken.
    ///
    /// # Errors
    /// `Invalid` for a scale other than 1, 2, 4 or 8, an empty banding, or a piece the device
    /// cannot hold.
    pub fn export_bands(
        &self,
        artwork: &Artwork,
        background: Color,
        scale: u32,
        banding: Banding,
    ) -> Result<Bands<'_>, RenderError> {
        Output::at(scale, [0, 0])?;
        let canvas = artwork.canvas();
        let size = [canvas.width(), canvas.height()].map(|side| side.saturating_mul(scale));
        if banding.rows == 0 || banding.columns == 0 {
            return Err(RenderError::Invalid("an export band holds no pixels"));
        }
        let piece = [banding.columns.min(size[0]), banding.rows.min(size[1])];
        let scene = Scene::build_knowing(artwork, None, (Vec::new(), Earlier::default()))?;
        let raster = Box::new(RasterExport::new(self, scene, (piece, false))?);
        Ok(Bands {
            core: self,
            artwork: artwork.clone(),
            background,
            scale,
            size,
            banding,
            raster,
            next: 0,
            planned: Duration::ZERO,
            waited: Duration::ZERO,
        })
    }

    /// The `size` pixels of `artwork` from `origin` at 1x on `background` paper, as the screen's
    /// canvas shows them (a practice copy's frame, read off the frame loop): drawn in a worker's
    /// paced regions on targets of their own, so the screen draws on meanwhile.
    ///
    /// # Errors
    /// `Invalid` for a region not inside the canvas; as the export.
    pub fn export_region(
        &self,
        artwork: &Artwork,
        background: Color,
        (origin, size): ([u32; 2], [u32; 2]),
    ) -> Result<RgbaImage, RenderError> {
        let canvas = artwork.canvas();
        let sides = [canvas.width(), canvas.height()];
        let inside = (0..2).all(|axis| {
            size[axis] > 0
                && u64::from(origin[axis]) + u64::from(size[axis]) <= u64::from(sides[axis])
        });
        if !inside {
            return Err(RenderError::Invalid("a region not inside the canvas"));
        }
        let scene = Scene::build_knowing(artwork, None, (Vec::new(), Earlier::default()))?;
        let mut raster = RasterExport::new(self, scene, (size, true))?;
        let paper = [background.r(), background.g(), background.b(), 1.0];
        raster.paced_piece(self, (artwork, 1, origin), size, paper)
    }
}

impl Bands<'_> {
    /// `[width, height]` of the whole output in pixels.
    #[must_use]
    pub fn size(&self) -> [u32; 2] {
        self.size
    }

    /// Time spent so far planning and submitting (CPU), and waiting for the pixels (GPU).
    #[must_use]
    pub fn timing(&self) -> (Duration, Duration) {
        (self.planned, self.waited)
    }

    /// Every band stacked into one picture of the whole output.
    ///
    /// # Errors
    /// As the bands.
    pub fn image(mut self) -> Result<RgbaImage, RenderError> {
        let [width, height] = self.size;
        let mut pixels = Vec::with_capacity(width as usize * height as usize * 4);
        for band in &mut self {
            pixels.extend_from_slice(band?.pixels());
        }
        Ok(RgbaImage::new(self.size, pixels))
    }

    fn band(&mut self, top: u32) -> Result<RgbaImage, RenderError> {
        let [width, height] = self.size;
        let rows = self.banding.rows.min(height - top);
        let mut band = vec![0; width as usize * rows as usize * 4];
        let paper = self.background;
        let paper = [paper.r(), paper.g(), paper.b(), 1.0];
        let mut left = 0;
        while left < width {
            let columns = self.banding.columns.min(width - left);
            let at = (&self.artwork, self.scale, [left, top]);
            let (piece, [planned, waited]) =
                (self.raster).piece(self.core, at, [columns, rows], paper)?;
            self.planned += planned;
            self.waited += waited;
            let stride = piece.size()[0] as usize * 4;
            let (from, to) = (columns as usize * 4, width as usize * 4);
            for (row, source) in piece
                .pixels()
                .chunks_exact(stride)
                .take(rows as usize)
                .enumerate()
            {
                let at = row * to + left as usize * 4;
                band[at..at + from].copy_from_slice(&source[..from]);
            }
            left += columns;
        }
        Ok(RgbaImage::new([width, rows], band))
    }
}

impl Iterator for Bands<'_> {
    type Item = Result<RgbaImage, RenderError>;

    /// The next band, evaluated now; after a failure, none.
    fn next(&mut self) -> Option<Self::Item> {
        let top = self.next;
        if top >= self.size[1] {
            return None;
        }
        let band = self.band(top);
        self.next = match band {
            Ok(_) => top + self.banding.rows.min(self.size[1] - top),
            Err(_) => self.size[1],
        };
        Some(band)
    }
}
