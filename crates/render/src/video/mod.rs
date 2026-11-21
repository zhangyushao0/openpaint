//! The replay's video frames (S163): `VideoTarget` draws each moment of the replay into a canvas
//! target of its own, kept from frame to frame (consecutive moments share their records, so a
//! frame that only adds strokes is the scene's append and the stroke being drawn the target's
//! live path), scales it to the video's size and reads it back as RGBA8; and the video's end
//! card: the picture its last frame showed, fitted on the desk with the canvas's shadow, above
//! the band of the caption the caller painted.
//!
//! The scale pass (`shaders/video.wgsl`): each video pixel is the mean of the canvas pixels it
//! covers, each weighted by how much of it it covers, in integers, so `video_reference`
//! reproduces it exactly on the CPU and a video of the canvas's own size is the canvas pixel for
//! pixel (`scale.rs`).
//!
//! Its memory is bounded and counted: the target is the video worker's, drawn paced (each region
//! waited for, so the GPU holds one region of the video's work at a time and the screen's frames
//! find it free between them) and read back through buffers kept from frame to frame into the
//! one frame image it lends; its snapshots under the stroke drawn are held to `SNAPSHOTS`, its
//! arena to the room it was given (the budget's share of the residuals, stepped each frame as the
//! budget steps the screen's); `bytes` is everything it holds, which the GPU memory budget counts
//! while the video is made. The card's layout and composition: `card.rs`.

use openpaint_core::Artwork;

use crate::budget::CacheBytes;
use crate::canvas::CanvasTarget;
use crate::change::changed_regions;
use crate::core::Core;
use crate::error::RenderError;
use crate::frame::FrameRequest;
use crate::geom::PixelRect;
use crate::image::RgbaImage;

mod card;
mod scale;

use card::Card;
use scale::{Bindings, ScalePass};

/// The bytes the snapshots under the stroke being drawn may hold.
const SNAPSHOTS: u64 = 64 << 20;

/// A canvas of one size drawn frame after frame and each frame scaled to the video's size.
pub struct VideoTarget {
    canvas: CanvasTarget,
    /// The version its canvas holds, once a frame was drawn.
    drawn: Option<Artwork>,
    video: [u32; 2],
    scale: ScalePass,
    /// The scale pass's bindings for the video's size, and the last frame read through them.
    frames: Bindings,
    image: RgbaImage,
    /// The bytes its arena keeps within.
    arena: u64,
}

impl VideoTarget {
    /// A target for an artwork of `canvas` pixels whose frames are `video` pixels, its arena (the
    /// smudges' residuals and the filters' results) kept within `arena` bytes.
    ///
    /// # Errors
    /// `Invalid` for a side of 0 or beyond the device's textures or buffers.
    pub fn new(
        core: &Core,
        canvas: [u32; 2],
        (video, arena): ([u32; 2], u64),
    ) -> Result<Self, RenderError> {
        let mut target = CanvasTarget::paced(core, canvas)?;
        target.set_snapshot_room(SNAPSHOTS);
        let scale = ScalePass::new(&core.gpu.device);
        let frames = scale.bindings(core, &target, video)?;
        Ok(Self {
            canvas: target,
            drawn: None,
            video,
            scale,
            image: frames.image(),
            frames,
            arena,
        })
    }

    /// `[width, height]` of its frames.
    #[must_use]
    pub const fn size(&self) -> [u32; 2] {
        self.video
    }

    /// `request` drawn (only what changed since the frame before, the stroke being drawn by the
    /// live path), scaled to the video's size and read back: opaque RGBA8, rows top first. The
    /// image is the target's, written over by the next frame; when this returns the GPU holds
    /// none of the frame's work.
    ///
    /// # Errors
    /// `Invalid` when the request's canvas is not the target's; as the GPU.
    pub fn frame(
        &mut self,
        core: &Core,
        request: &FrameRequest<'_>,
    ) -> Result<&RgbaImage, RenderError> {
        let regions = match &self.drawn {
            Some(before) => changed_regions(before, request.artwork),
            None => vec![PixelRect::full(self.canvas.size())],
        };
        self.drawn = None;
        self.canvas.set_arena_room(core, self.arena)?;
        self.canvas.draw(core, request, &regions)?;
        self.drawn = Some(request.artwork.clone());
        self.scale.run(core, &self.frames, &mut self.image)?;
        Ok(&self.image)
    }

    /// Everything it holds on the GPU: its canvas with the scale's buffers, its snapshots, its
    /// arena and its raster targets, as `video`.
    #[must_use]
    pub fn bytes(&self) -> CacheBytes {
        let raster = self.canvas.raster_held().map_or(0, |(bytes, _)| bytes);
        let held = self.canvas.bytes();
        CacheBytes {
            video: held.total() + raster + self.frames.bytes(),
            ..CacheBytes::default()
        }
    }

    /// The end card at the video's size: the `desk` colour, the picture the last frame showed
    /// fitted above the band with the canvas's shadow, and `caption` (the band, the video's width,
    /// painted by the caller) along the bottom.
    ///
    /// # Errors
    /// `Invalid` before a frame was drawn, or for a band not of the video's width and lower than
    /// it; as the GPU.
    pub fn card(
        &mut self,
        core: &Core,
        (desk, caption): ([u8; 4], &RgbaImage),
    ) -> Result<RgbaImage, RenderError> {
        let [width, height] = self.video;
        let band = caption.size();
        if band[0] != width || band[1] >= height {
            return Err(RenderError::Invalid("a caption band not across the video"));
        }
        if self.drawn.is_none() {
            return Err(RenderError::Invalid("a card before any frame was drawn"));
        }
        let card = Card::of(self.video, band[1], self.canvas.size());
        let bindings = self.scale.bindings(core, &self.canvas, card.size)?;
        let mut picture = bindings.image();
        self.scale.run(core, &bindings, &mut picture)?;
        Ok(card.compose(desk, &picture, caption))
    }
}

/// `image` scaled to `size` as the scale pass scales a canvas (see the module), on the CPU.
#[must_use]
pub fn video_reference(image: &RgbaImage, size: [u32; 2]) -> RgbaImage {
    let [n_x, n_y] = image.size().map(u64::from);
    let [m_x, m_y] = size.map(u64::from);
    let overlap = |o: u64, i: u64, n: u64, m: u64| {
        let (start, end) = ((o * n).max(i * m), ((o + 1) * n).min((i + 1) * m));
        end.saturating_sub(start)
    };
    let pixels = image.pixels();
    let mut out = Vec::with_capacity((m_x * m_y * 4) as usize);
    for y in 0..m_y {
        let rows = (y * n_y / m_y)..((y + 1) * n_y).div_ceil(m_y);
        for x in 0..m_x {
            let columns = (x * n_x / m_x)..((x + 1) * n_x).div_ceil(m_x);
            let mut total = [0u64; 4];
            for row in rows.clone() {
                let mut sum = [0u64; 4];
                for column in columns.clone() {
                    let at = ((row * n_x + column) * 4) as usize;
                    let weight = overlap(x, column, n_x, m_x);
                    for (channel, sum) in sum.iter_mut().enumerate() {
                        *sum += u64::from(pixels[at + channel]) * weight;
                    }
                }
                let weight = overlap(y, row, n_y, m_y);
                for (total, sum) in total.iter_mut().zip(sum) {
                    *total += (sum * 256 + n_x / 2) / n_x * weight;
                }
            }
            out.extend(total.map(|total| ((total + n_y * 128) / (n_y * 256)) as u8));
        }
    }
    RgbaImage::new(size, out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_reference_keeps_an_image_of_its_own_size_and_averages_by_area() {
        let pixels: Vec<u8> = (0..6 * 2)
            .flat_map(|i| [i * 20, 255 - i * 20, 7, 255])
            .collect();
        let image = RgbaImage::new([6, 2], pixels);
        assert_eq!(video_reference(&image, [6, 2]), image);
        // Six columns into four: the second video pixel covers half of column 1 and all of 2.
        let scaled = video_reference(&image, [4, 1]);
        let red = |x: usize| scaled.pixels()[x * 4];
        // Columns 1 and 2 hold 20 and 40 on the first row, 140 and 160 on the second.
        let expected = ((20.0 + 140.0) / 2.0 * 0.5 + (40.0 + 160.0) / 2.0) / 1.5;
        assert_eq!(f64::from(red(1)), f64::round(expected));
        let (pixels, _) = scaled.pixels().as_chunks::<4>();
        assert!(pixels.iter().all(|pixel| pixel[3] == 255));
    }
}
