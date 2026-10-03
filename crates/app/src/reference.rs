//! The reference image (Procreate's Actions > Canvas > Reference): a picture the user chooses
//! from the system's picker, shown in a window floating over the canvas, where the eyedropper can
//! pick its colours. It is kept for this run of the app, not with an artwork, and changes
//! nothing in the artwork (no history step).

use std::sync::Arc;

use openpaint_core::Color;

use crate::project::PortError;
use crate::transfer::{HostAnswer, HostRequest, ImageError, OpenJob};

/// Longest side of a reference image as kept: larger images are scaled down to it when opened.
/// The window shows it at most a few hundred points large, zoomed up to 8 times.
pub const MAX_REFERENCE_SIDE: u32 = 2048;
/// Most pixels an image file may decode to: a 50-megapixel camera picture.
pub const MAX_IMAGE_PIXELS: u64 = 50_000_000;
/// Largest image file read, in bytes.
pub const MAX_IMAGE_BYTES: u64 = 64 << 20;

/// An opened reference image: 8-bit RGBA, rows top first, opaque (transparent parts lie on
/// white paper, as they show), at most `MAX_REFERENCE_SIDE` on a side. Cloning shares the
/// pixels; two images are equal only when they are the same opened image.
#[derive(Debug, Clone)]
pub struct ReferenceImage {
    size: [u32; 2],
    pixels: Arc<[u8]>,
}

impl ReferenceImage {
    /// The decoded image of `size` (RGBA `pixels`, unpremultiplied) on white, scaled down to fit
    /// `MAX_REFERENCE_SIDE`: each pixel the rounded mean of its box. `None` unless `pixels`
    /// holds exactly `width * height * 4` bytes and no side is zero.
    #[must_use]
    pub fn fitted(size: [u32; 2], pixels: &[u8]) -> Option<Self> {
        let expected = u64::from(size[0]) * u64::from(size[1]) * 4;
        if size.contains(&0) || pixels.len() as u64 != expected {
            return None;
        }
        let largest = size[0].max(size[1]);
        let fit = if largest > MAX_REFERENCE_SIDE {
            size.map(|side| {
                let scaled = u64::from(side) * u64::from(MAX_REFERENCE_SIDE) / u64::from(largest);
                (scaled as u32).max(1)
            })
        } else {
            size
        };
        Some(Self {
            size: fit,
            pixels: reduce(size, pixels, fit).into(),
        })
    }

    /// `[width, height]` in pixels.
    #[must_use]
    pub const fn size(&self) -> [u32; 2] {
        self.size
    }

    #[must_use]
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// The colour of the pixel at `[x, y]`; `None` outside the image.
    #[must_use]
    pub fn color_at(&self, [x, y]: [u32; 2]) -> Option<Color> {
        if x >= self.size[0] || y >= self.size[1] {
            return None;
        }
        let at = ((u64::from(y) * u64::from(self.size[0]) + u64::from(x)) * 4) as usize;
        let channel = |offset: usize| f32::from(self.pixels[at + offset]) / 255.0;
        Color::rgb(channel(0), channel(1), channel(2)).ok()
    }
}

impl PartialEq for ReferenceImage {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.pixels, &other.pixels)
    }
}

/// `pixels` of `size` flattened on white and reduced to `fit` by box means in integers.
fn reduce(size: [u32; 2], pixels: &[u8], fit: [u32; 2]) -> Vec<u8> {
    let [width, height] = size.map(u64::from);
    let [w, h] = fit.map(u64::from);
    let mut out = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        let rows = (y * height / h)..((y + 1) * height / h);
        for x in 0..w {
            let columns = (x * width / w)..((x + 1) * width / w);
            let mut sum = [0u64; 3];
            for row in rows.clone() {
                for column in columns.clone() {
                    let at = ((row * width + column) * 4) as usize;
                    let alpha = u64::from(pixels[at + 3]);
                    for (channel, total) in sum.iter_mut().enumerate() {
                        // Over white: c * a + 255 * (255 - a), in units of 255.
                        let value = u64::from(pixels[at + channel]);
                        *total += (value * alpha + 255 * (255 - alpha) + 127) / 255;
                    }
                }
            }
            let count = (rows.end - rows.start) * (columns.end - columns.start);
            out.extend(sum.map(|total| ((total + count / 2) / count) as u8));
            out.push(u8::MAX);
        }
    }
    out
}

/// What the user asks of the reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceIntent {
    /// Open (the image of this session, or the picker when there is none) or close the window.
    Show(bool),
    /// Choose a new image with the system's picker; it replaces the one shown.
    Choose,
    /// Close the notice that an image could not be opened.
    Dismiss,
}

/// What the canvas's notice says about the reference.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ReferenceNotice {
    #[default]
    None,
    /// The picker is open, or the chosen image is being read.
    Opening,
    Failed(ImageError),
}

#[derive(Debug, Default)]
enum State {
    #[default]
    Idle,
    Choosing,
    Opening,
    Failed(ImageError),
}

#[derive(Debug, Default)]
pub(crate) struct Reference {
    image: Option<ReferenceImage>,
    shown: bool,
    state: State,
}

impl Reference {
    /// Applies `intent`; returns what to ask the platform for. Choosing is only called when no
    /// picker or transfer runs (`Transfers` checks).
    pub(crate) fn dispatch(&mut self, intent: &ReferenceIntent) -> Option<HostRequest> {
        match intent {
            ReferenceIntent::Show(true) if self.image.is_some() => self.shown = true,
            ReferenceIntent::Show(false) => self.shown = false,
            ReferenceIntent::Show(true) | ReferenceIntent::Choose => {
                self.state = State::Choosing;
                return Some(HostRequest::OpenImage);
            }
            ReferenceIntent::Dismiss => {
                if matches!(self.state, State::Failed(_)) {
                    self.state = State::Idle;
                }
            }
        }
        None
    }

    pub(crate) fn waiting(&self) -> bool {
        matches!(self.state, State::Choosing)
    }

    pub(crate) fn busy(&self) -> bool {
        matches!(self.state, State::Choosing | State::Opening)
    }

    /// The platform's answer while `waiting`: an image is read next; cancelling changes nothing.
    pub(crate) fn answer(&mut self, answer: HostAnswer) -> Option<OpenJob> {
        if !self.waiting() {
            return None;
        }
        self.state = State::Idle;
        match answer {
            HostAnswer::Image(source) => {
                self.state = State::Opening;
                Some(OpenJob { source })
            }
            HostAnswer::Cancelled => None,
            HostAnswer::Failed(error) => {
                self.state = State::Failed(ImageError::Picker(error));
                None
            }
            HostAnswer::File(_) => {
                let error = PortError::Device {
                    detail: "a file to write came back for an image".into(),
                };
                self.state = State::Failed(ImageError::Picker(error));
                None
            }
        }
    }

    /// The chosen image was read: it replaces the reference and the window shows it. A failure
    /// keeps the image before.
    pub(crate) fn opened(&mut self, result: Result<ReferenceImage, ImageError>) {
        if !matches!(self.state, State::Opening) {
            return;
        }
        match result {
            Ok(image) => {
                self.image = Some(image);
                self.shown = true;
                self.state = State::Idle;
            }
            Err(error) => self.state = State::Failed(error),
        }
    }

    pub(crate) fn image(&self) -> Option<&ReferenceImage> {
        self.image.as_ref()
    }

    pub(crate) fn shown(&self) -> bool {
        self.shown && self.image.is_some()
    }

    pub(crate) fn notice(&self) -> ReferenceNotice {
        match &self.state {
            State::Idle => ReferenceNotice::None,
            State::Choosing | State::Opening => ReferenceNotice::Opening,
            State::Failed(error) => ReferenceNotice::Failed(error.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn large_images_are_scaled_down_by_box_means_on_white() {
        // 4100 x 2 pixels: black left half, transparent right half.
        let size = [4100, 2];
        let mut pixels = Vec::new();
        for _ in 0..2 {
            for x in 0..4100 {
                pixels.extend(if x < 2050 {
                    [0, 0, 0, 255]
                } else {
                    [0, 0, 0, 0]
                });
            }
        }
        let image = ReferenceImage::fitted(size, &pixels).expect("fits");
        assert_eq!(image.size(), [2048, 1]);
        let first = image.color_at([0, 0]).expect("inside");
        assert_eq!([first.r(), first.g(), first.b()], [0.0; 3]);
        let last = image.color_at([2047, 0]).expect("inside");
        assert_eq!(
            [last.r(), last.g(), last.b()],
            [1.0; 3],
            "transparent shows the paper"
        );
        assert!(image.pixels().chunks(4).all(|pixel| pixel[3] == 255));
        assert_eq!(image.color_at([2048, 0]), None);
    }

    #[test]
    fn small_images_keep_their_pixels() {
        let pixels = [10, 20, 30, 255, 200, 100, 50, 255];
        let image = ReferenceImage::fitted([2, 1], &pixels).expect("fits");
        assert_eq!(image.size(), [2, 1]);
        assert_eq!(image.pixels(), pixels);
        assert!(
            ReferenceImage::fitted([2, 2], &pixels).is_none(),
            "wrong length"
        );
        assert_eq!(image.clone(), image, "the same opened image");
        let again = ReferenceImage::fitted([2, 1], &pixels).expect("fits");
        assert_ne!(again, image, "another opened image");
    }
}
