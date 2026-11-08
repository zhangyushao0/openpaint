//! Pictures on a practice sheet (S80): a pack's picture's pixels as the store read them for the
//! screen, and a single shape's with its ground faded (S99); where a picture lies in its frame is
//! the practice crate's (`SheetPicture`); the packs kept as the Today card's options list them;
//! and the aligned overlay's fit, found off the frame loop (`FitJob`).

use openpaint_practice::{Fit, Pack, Point, SheetPicture, ToFit, align, align_shape, faded_ground};

/// The longest side a picture is read at for the screen, in pixels: a frame shows it at most
/// about 900 pixels on the tablet, and the reduced picture keeps every session's few pictures
/// within some 30 MB.
pub const SHOWN_SIDE: u32 = 1024;
/// How strongly a single shape's ground shows in its original's frame (S99, a trial put to the
/// author): faded towards the paper, still there for context; the object at full strength.
pub const GROUND_SHOWN: f32 = 0.3;

/// `image` (`picture` as read for the screen) with its ground, outside its outline, at
/// `GROUND_SHOWN` (S99): a single shape's original, the object whole against the frame; none
/// without an outline.
#[must_use]
pub fn grounded(picture: &SheetPicture, image: &PictureImage) -> Option<PictureImage> {
    let outline = picture.outline.as_ref()?;
    // Frame units to the image's pixels: the picture fills its frame.
    let scale = [0, 1].map(|axis| f64::from(image.size[axis]) / picture.fit.size[axis]);
    let lines: Vec<Vec<Point>> = (outline.iter())
        .map(|line| {
            line.iter()
                .map(|p| [p[0] * scale[0], p[1] * scale[1]])
                .collect()
        })
        .collect();
    let shown = f64::from(GROUND_SHOWN);
    Some(PictureImage {
        size: image.size,
        pixels: faded_ground(image.size, &image.pixels, &lines, shown),
    })
}

/// A picture's pixels as the screen shows them: 8-bit sRGB with straight alpha, a pack's reduced
/// to at most `SHOWN_SIDE`; or a copy's pixels as the canvas shows them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PictureImage {
    pub size: [u32; 2],
    pub pixels: Vec<u8>,
}

impl PictureImage {
    /// The colour at the point `p` given in shares of the width and height (0 to 1), the nearest
    /// pixel's; none outside.
    #[must_use]
    pub fn colour_at(&self, p: Point) -> Option<[u8; 3]> {
        let inside = (0.0..1.0).contains(&p[0]) && (0.0..1.0).contains(&p[1]);
        let [w, h] = self.size.map(|side| side as usize);
        let [x, y] = [(p[0] * w as f64) as usize, (p[1] * h as f64) as usize];
        let at = 4 * (y.min(h.saturating_sub(1)) * w + x.min(w.saturating_sub(1)));
        let texel = self.pixels.get(at..at + 3).filter(|_| inside)?;
        Some([texel[0], texel[1], texel[2]])
    }
}

/// A pack kept, as the Today card's options list it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackShown {
    pub id: String,
    pub title: String,
    /// The user's own pictures.
    pub personal: bool,
    pub pictures: usize,
    /// How many of them have an outline; none for a pack of kinds never outlined (compositions,
    /// S95).
    pub outlines: Option<usize>,
}

impl PackShown {
    #[must_use]
    pub fn of(pack: &Pack) -> Self {
        let outlined = pack.items.iter().any(|item| item.kind.outlined());
        Self {
            id: pack.id.clone(),
            title: pack.title.clone(),
            personal: pack.personal(),
            pictures: pack.items.len(),
            outlines: outlined.then(|| pack.items.iter().filter(|i| i.outline.is_some()).count()),
        }
    }
}

/// The aligned overlay's fit for the worker: `original` (a line, a group, or a shape's outline,
/// in frame units) laid onto the lines of `copy`.
#[derive(Debug, Clone, PartialEq)]
pub struct FitJob {
    /// Which Compare asked: a fit for an older one is dropped.
    pub sequence: u64,
    pub original: Vec<Vec<Point>>,
    pub copy: Vec<Vec<Point>>,
    /// A single shape's outline (`align_shape`), else lines (`align`).
    pub shape: bool,
}

impl FitJob {
    /// The fit for the Compare numbered `sequence` of `asked` onto the lines of `copy`.
    #[must_use]
    pub(crate) fn of(sequence: u64, asked: ToFit, copy: Vec<Vec<Point>>) -> Self {
        Self {
            sequence,
            original: asked.original,
            copy,
            shape: asked.shape,
        }
    }

    /// The fit; `None` when either side has no length.
    #[must_use]
    pub fn run(&self) -> Option<Fit> {
        let original: Vec<&[Point]> = self.original.iter().map(Vec::as_slice).collect();
        let copy: Vec<&[Point]> = self.copy.iter().map(Vec::as_slice).collect();
        if self.shape {
            align_shape(&original, &copy)
        } else {
            align(&original, &copy)
        }
    }
}
