//! Pictures on a practice sheet (S80): a single shape's picture as its pack holds it (its size,
//! its outline and the credit to show under it), fitted in its frame; the picture's pixels as the
//! store read them for the screen; the packs kept as the Today card's options list them; and the
//! aligned overlay's fit, found off the frame loop (`FitJob`).

use openpaint_practice::{Fit, ItemRef, Pack, PackItem, PictureFit, Point, align, align_shape};

/// The longest side a picture is read at for the screen, in pixels: a frame shows it at most
/// about 900 pixels on the tablet, and the reduced picture keeps every session's few pictures
/// within some 30 MB.
pub const SHOWN_SIDE: u32 = 1024;

/// A single shape's picture on the sheet.
#[derive(Debug, Clone, PartialEq)]
pub struct SheetPicture {
    pub item: ItemRef,
    /// The picture's size, in its pack's pixels.
    pub size: [u32; 2],
    /// Where it lies in its frame.
    pub fit: PictureFit,
    /// Its outline's closed lines in frame units; none where its pack has none.
    pub outline: Option<Vec<Vec<Point>>>,
    /// The credit that shows small under it: an Attribution licence's.
    pub credit: Option<String>,
}

impl SheetPicture {
    /// The picture `item` names, as `packs` hold it; `None` when no pack kept holds it.
    #[must_use]
    pub fn of(item: &ItemRef, packs: &[Pack]) -> Option<Self> {
        let found: &PackItem = packs.iter().find_map(|pack| pack.find(item))?;
        let fit = PictureFit::of(found.size);
        let outline = (found.outline.as_ref()).map(|outline| {
            (outline.lines().into_iter())
                .map(|line| line.iter().map(|p| fit.to_frame(*p)).collect())
                .collect()
        });
        Some(Self {
            item: item.clone(),
            size: found.size,
            fit,
            outline,
            credit: found.shown_credit().map(str::to_owned),
        })
    }
}

/// A picture's pixels as the screen shows them: 8-bit sRGB with straight alpha, reduced to at
/// most `SHOWN_SIDE`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PictureImage {
    pub size: [u32; 2],
    pub pixels: Vec<u8>,
}

/// A pack kept, as the Today card's options list it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackShown {
    pub id: String,
    pub title: String,
    /// The user's own pictures.
    pub personal: bool,
    pub pictures: usize,
    /// How many of them have an outline.
    pub outlines: usize,
}

impl PackShown {
    #[must_use]
    pub fn of(pack: &Pack) -> Self {
        Self {
            id: pack.id.clone(),
            title: pack.title.clone(),
            personal: pack.personal(),
            pictures: pack.items.len(),
            outlines: pack.items.iter().filter(|i| i.outline.is_some()).count(),
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
