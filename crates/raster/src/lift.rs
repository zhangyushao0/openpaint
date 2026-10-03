//! Lifting pixels for a transform: the layer's painted pixels (or those inside a selection) in
//! their tight bounding rectangle, as the patch the transform places again.

use openpaint_core::{CanvasSize, Patch, RegionMask};

/// The pixels of `layer` (premultiplied 8-bit RGBA at `canvas` size, as the renderer samples a
/// layer) that hold paint, inside `clip` when given, in their tight bounding rectangle; outside
/// `clip` the patch is transparent. `None` when no such pixel holds paint, or `layer` is not
/// one pixel per canvas pixel.
#[must_use]
pub fn lift(canvas: CanvasSize, layer: &[u8], clip: Option<&RegionMask>) -> Option<Patch> {
    let (width, height) = (canvas.width() as usize, canvas.height() as usize);
    if layer.len() != width * height * 4 {
        return None;
    }
    let taken = |x: usize, y: usize| {
        layer[(y * width + x) * 4 + 3] > 0
            && clip.is_none_or(|clip| clip.contains([x as u32, y as u32]))
    };
    // Only the clip's rectangle can hold lifted pixels.
    let ([x0, y0], [w, h]) = clip.map_or(([0, 0], [canvas.width(), canvas.height()]), |clip| {
        (clip.origin(), clip.size())
    });
    let (mut min, mut max) = ([usize::MAX; 2], [0usize; 2]);
    for y in y0 as usize..(y0 + h) as usize {
        for x in x0 as usize..(x0 + w) as usize {
            if taken(x, y) {
                min = [min[0].min(x), min[1].min(y)];
                max = [max[0].max(x + 1), max[1].max(y + 1)];
            }
        }
    }
    if min[0] == usize::MAX {
        return None;
    }
    let size = [max[0] - min[0], max[1] - min[1]];
    let mut pixels = Vec::with_capacity(size[0] * size[1] * 4);
    for y in min[1]..max[1] {
        for x in min[0]..max[0] {
            let at = (y * width + x) * 4;
            if taken(x, y) {
                pixels.extend_from_slice(&layer[at..at + 4]);
            } else {
                pixels.extend_from_slice(&[0; 4]);
            }
        }
    }
    let side = |value: usize| u32::try_from(value).ok();
    Patch::new(
        canvas,
        [side(min[0])?, side(min[1])?],
        [side(size[0])?, side(size[1])?],
        pixels,
    )
    .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canvas() -> CanvasSize {
        CanvasSize::new(8, 6).expect("size")
    }

    /// A layer painted at (2, 1), (5, 1) half transparent, and (3, 4).
    fn layer() -> Vec<u8> {
        let mut pixels = vec![0u8; 8 * 6 * 4];
        for ([x, y], value) in [([2, 1], [9, 8, 7, 255]), ([5, 1], [4, 4, 4, 128])]
            .into_iter()
            .chain([([3, 4], [1, 2, 3, 255])])
        {
            pixels[(y * 8 + x) * 4..][..4].copy_from_slice(&value);
        }
        pixels
    }

    #[test]
    fn the_painted_pixels_are_lifted_in_their_bounding_rectangle() {
        let patch = lift(canvas(), &layer(), None).expect("paint");
        assert_eq!((patch.origin(), patch.size()), ([2, 1], [4, 4]));
        assert_eq!(&patch.pixels()[..4], &[9, 8, 7, 255]);
        assert_eq!(&patch.pixels()[12..16], &[4, 4, 4, 128]);
        assert_eq!(
            lift(canvas(), &[0; 8 * 6 * 4], None),
            None,
            "nothing painted"
        );
        assert_eq!(lift(canvas(), &[0; 4], None), None, "the wrong size");
    }

    #[test]
    fn inside_a_selection_only_its_pixels_are_lifted() {
        let mut inside = vec![false; 8 * 6];
        for at in [8 + 2, 4 * 8 + 3, 4 * 8 + 4] {
            inside[at] = true;
        }
        let clip = RegionMask::from_pixels(canvas(), &inside).expect("mask");
        let patch = lift(canvas(), &layer(), Some(&clip)).expect("paint");
        assert_eq!((patch.origin(), patch.size()), ([2, 1], [2, 4]));
        let transparent = patch.pixels().chunks(4).filter(|p| p[3] == 0).count();
        assert_eq!(transparent, 6, "only the two painted, selected pixels");
        let mut empty = vec![false; 8 * 6];
        empty[0] = true;
        let nothing = RegionMask::from_pixels(canvas(), &empty).expect("mask");
        assert_eq!(lift(canvas(), &layer(), Some(&nothing)), None);
    }
}
