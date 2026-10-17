//! Export (S29): an artwork at 1, 2, 4 or 8 output pixels per canvas pixel (the app offers up to
//! 4), a band of rows at a time, each band in pieces at most `Banding::columns` wide, read back
//! and handed on, so a 4x export never holds the whole picture on the GPU. Since S59 (D86) the
//! raster core draws it (`export_raster.rs`; smudges too since S61, D88) unless its scene goes to
//! the evaluator (`raster_route.rs`, logged with why), which evaluates it through the single
//! evaluation entry into a target of its own. Neither shares anything with the screen's, so a worker holding an
//! `Arc<Core>` runs it while the screen draws on. Since a pixel never depends on which regions are
//! drawn with it, bands equal one piece; at 1x the evaluator's result is the screen's canvas pixel
//! for pixel, the raster core's within one 8-bit level away from antialiased edges (D86).

use std::time::{Duration, Instant};

use openpaint_core::{Artwork, Color};

use crate::core::Core;
use crate::error::RenderError;
use crate::evaluator::{CanvasTarget, RegionEvaluator};
use crate::export_raster::RasterExport;
use crate::frame::FrameRequest;
use crate::geom::PixelRect;
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
    source: Source,
    /// The first output row of the next band; the height once done or failed.
    next: u32,
    /// Time spent planning and submitting pieces (the scene, smudges and binning on the CPU),
    /// and waiting for them to come back (the GPU and the readback).
    planned: Duration,
    waited: Duration,
}

/// What draws an export's pieces.
enum Source {
    Rastered(Box<RasterExport>),
    Evaluated(Box<CanvasTarget>),
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
        let source = match scene.routing.kind() {
            Ok(()) => {
                log::info!("export at {scale}x drawn by the raster core");
                Source::Rastered(Box::new(RasterExport::new(self, scene, piece)?))
            }
            Err(routed) => {
                log::info!("export at {scale}x evaluated: the artwork holds {routed}");
                Source::Evaluated(Box::new(CanvasTarget::exact(self, piece)?))
            }
        };
        Ok(Bands {
            core: self,
            artwork: artwork.clone(),
            background,
            scale,
            size,
            banding,
            source,
            next: 0,
            planned: Duration::ZERO,
            waited: Duration::ZERO,
        })
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
        let request = FrameRequest {
            artwork: &self.artwork,
            active: None,
            background: self.background,
        };
        let mut left = 0;
        while left < width {
            let columns = self.banding.columns.min(width - left);
            let piece = match &mut self.source {
                Source::Rastered(raster) => {
                    let at = (&self.artwork, self.scale, [left, top]);
                    let paper = self.background;
                    let paper = [paper.r(), paper.g(), paper.b(), 1.0];
                    let (piece, [planned, waited]) =
                        raster.piece(self.core, at, [columns, rows], paper)?;
                    self.planned += planned;
                    self.waited += waited;
                    piece
                }
                Source::Evaluated(target) => {
                    let output = Output::at(self.scale, [left, top])?;
                    let started = Instant::now();
                    let region = PixelRect::full([columns, rows]);
                    RegionEvaluator::new(self.core, target)
                        .evaluate_piece(&request, output, region)?;
                    let submitted = Instant::now();
                    let piece = target.read(self.core)?;
                    self.waited += submitted.elapsed();
                    self.planned += submitted - started;
                    piece
                }
            };
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
