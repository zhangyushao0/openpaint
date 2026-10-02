//! Where two canvases differ, as an image to look at: differing pixels pure red, equal pixels
//! a faded copy of the actual canvas so the red stands out against the drawing.

use openpaint_render::RgbaImage;

const MARK: [u8; 4] = [255, 0, 0, 255];

/// A diff image (8-bit RGBA, rows top first) and how many pixels differ.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diff {
    pub size: [u32; 2],
    pub pixels: Vec<u8>,
    pub differing: u64,
}

/// `reference` is 8-bit RGBA of `reference_size`. `None` when the sizes differ.
#[must_use]
pub fn diff(reference_size: [u32; 2], reference: &[u8], actual: &RgbaImage) -> Option<Diff> {
    if reference_size != actual.size() || reference.len() != actual.pixels().len() {
        return None;
    }
    let mut differing = 0;
    let mut pixels = Vec::with_capacity(reference.len());
    let (old_pixels, _) = reference.as_chunks::<4>();
    let (new_pixels, _) = actual.pixels().as_chunks::<4>();
    for (old, new) in old_pixels.iter().zip(new_pixels) {
        if old == new {
            pixels.extend([fade(new[0]), fade(new[1]), fade(new[2]), 255]);
        } else {
            differing += 1;
            pixels.extend(MARK);
        }
    }
    Some(Diff {
        size: reference_size,
        pixels,
        differing,
    })
}

/// Three quarters of the way to white.
fn fade(value: u8) -> u8 {
    191 + value / 4
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faded_white_stays_white_and_black_turns_light_grey() {
        assert_eq!(fade(255), 254);
        assert_eq!(fade(0), 191);
        assert!(fade(0) < MARK[0]);
    }
}
