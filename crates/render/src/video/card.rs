//! The video's end card on the CPU, made once a video: the desk, the picture fitted above the
//! caption's band with the canvas's shadow as the screen lays it, the band along the bottom.

use crate::image::RgbaImage;
use crate::thumbnail::thumbnail_size;

/// The card's video pixels per point: its height over 900, as a tablet's screen of about 900
/// points high shows the canvas.
const POINTS_HIGH: f32 = 900.0;
/// Room between the card's picture and the edges of the space above the band, in points.
const MARGIN: f32 = 40.0;
/// The canvas's shadow on the desk as the screen lays it (the UI's `design::sheet_shadow`):
/// its strength at the sheet's edge, then each ring's width and share of it and its drop, in
/// points.
const SHADOW: f32 = 34.0;
const RINGS: [(f32, f32, f32); 2] = [(2.0, 0.7, 0.0), (24.0, 0.45, 4.0)];

/// The end card's layout: where the picture lies and how large, and a point in video pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Card {
    video: [u32; 2],
    band: u32,
    at: [u32; 2],
    pub(super) size: [u32; 2],
    point: f32,
}

impl Card {
    /// The picture of a `canvas` fitted, never enlarged, within the space above a band `band`
    /// high less the margin, centred there.
    pub(super) fn of(video: [u32; 2], band: u32, canvas: [u32; 2]) -> Self {
        let point = video[1] as f32 / POINTS_HIGH;
        let margin = (MARGIN * point).round() as u32;
        let above = video[1] - band;
        let room = [video[0], above].map(|side| side.saturating_sub(2 * margin).max(1));
        let size = thumbnail_size(canvas, room);
        let at = [(video[0] - size[0]) / 2, (above - size[1]) / 2];
        Self {
            video,
            band,
            at,
            size,
            point,
        }
    }

    /// The card: the desk, the picture's shadow, the picture, the band.
    pub(super) fn compose(
        &self,
        desk: [u8; 4],
        picture: &RgbaImage,
        caption: &RgbaImage,
    ) -> RgbaImage {
        let [width, height] = self.video.map(|side| side as usize);
        let mut pixels = [desk[0], desk[1], desk[2], 255].repeat(width * height);
        for (size, share, drop) in RINGS {
            self.shadow(&mut pixels, (size * self.point, share, drop * self.point));
        }
        let [x, y] = self.at.map(|at| at as usize);
        let row = self.size[0] as usize * 4;
        for (line, source) in picture.pixels().chunks_exact(row).enumerate() {
            let start = ((y + line) * width + x) * 4;
            pixels[start..start + row].copy_from_slice(source);
        }
        let band = (height - self.band as usize) * width * 4;
        pixels[band..].copy_from_slice(caption.pixels());
        RgbaImage::new(self.video, pixels)
    }

    /// One ring of the picture's shadow laid over `pixels`: black at the sheet's edge, fading
    /// out over `size` pixels, the ring dropped by `drop`.
    fn shadow(&self, pixels: &mut [u8], (size, share, drop): (f32, f32, f32)) {
        let strength = SHADOW * share / 255.0;
        let [x0, y0] = self.at.map(|at| at as f32);
        let [x1, y1] = [x0 + self.size[0] as f32, y0 + self.size[1] as f32];
        let (top, bottom) = (y0 + drop, y1 + drop);
        let reach = size.ceil() as i64 + 1;
        let span = |low: f32, high: f32, side: u32| {
            let from = (low as i64 - reach).max(0) as u32;
            from..((high as i64 + reach).max(0) as u32).min(side)
        };
        let width = self.video[0] as usize;
        let ([left, top_row], [right, bottom_row]) = (self.at, [x1 as u32, y1 as u32]);
        for y in span(top, bottom, self.video[1]) {
            let across = span(x0, x1, self.video[0]);
            // Where the picture lies it covers the shadow: only its flanks there.
            let hidden = if (top_row..bottom_row).contains(&y) {
                left..right
            } else {
                0..0
            };
            let flanks = (across.start..hidden.start.max(across.start))
                .chain(hidden.end.min(across.end)..across.end);
            for x in flanks {
                let [cx, cy] = [x as f32 + 0.5, y as f32 + 0.5];
                let dx = (x0 - cx).max(cx - x1).max(0.0);
                let dy = (top - cy).max(cy - bottom).max(0.0);
                let distance = dx.hypot(dy);
                let alpha = strength * (1.0 - distance / size).clamp(0.0, 1.0);
                let at = (y as usize * width + x as usize) * 4;
                for channel in &mut pixels[at..at + 3] {
                    *channel = (f32::from(*channel) * (1.0 - alpha)).round() as u8;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_card_fits_the_picture_above_the_band_and_never_enlarges_it() {
        let card = Card::of([1920, 1080], 134, [2880, 1800]);
        assert_eq!(card.point, 1.2);
        // 48 px of margin: the picture within 1824 x 850, the canvas's shape.
        assert_eq!(card.size, [1360, 850]);
        assert_eq!(card.at, [280, 48]);
        let small = Card::of([1920, 1080], 134, [400, 300]);
        assert_eq!(small.size, [400, 300]);
    }
}
