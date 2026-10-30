//! Pictures on a practice sheet (S80): a single shape's or a colour copy's picture as its pack
//! holds it (its size, its outline and the credit to show under it), fitted in its frame; the
//! picture's pixels as the store read them for the screen; the packs kept as the Today card's
//! options list them; and the aligned overlay's fit, found off the frame loop (`FitJob`).

use std::sync::Arc;

use openpaint_practice::{
    Fit, ItemRef, Pack, PackItem, PictureFit, Point, ValueMasses, align, align_shape,
};

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
    /// Its value masses (a composition's whose masses read, S96); none where its pack has none.
    pub masses: Option<MassesShown>,
}

/// A picture's value masses as its frame shows them: the answer, and its mass edges in frame
/// units (where the picture lies in its frame).
#[derive(Debug, Clone, PartialEq)]
pub struct MassesShown {
    pub answer: Arc<ValueMasses>,
    pub edges: Vec<Vec<Point>>,
}

impl SheetPicture {
    /// The picture `item` names, as `packs` hold it, fitted in a square frame or `filling` one
    /// of its own shape (a colour copy's); `None` when no pack kept holds it.
    #[must_use]
    pub fn of(item: &ItemRef, packs: &[Pack], filling: bool) -> Option<Self> {
        let found: &PackItem = packs.iter().find_map(|pack| pack.find(item))?;
        let fit = if filling {
            PictureFit::filling(found.size)
        } else {
            PictureFit::of(found.size)
        };
        let outline = (found.outline.as_ref()).map(|outline| {
            (outline.lines().into_iter())
                .map(|line| line.iter().map(|p| fit.to_frame(*p)).collect())
                .collect()
        });
        let masses = found.masses.as_ref().map(|answer| {
            let [x, y] = fit.origin;
            let [w, h] = fit.size;
            let at = |[u, v]: Point| [x + u * w, y + v * h];
            MassesShown {
                edges: (answer.edges().into_iter())
                    .map(|edge| edge.into_iter().map(at).collect())
                    .collect(),
                answer: Arc::clone(answer),
            }
        });
        Some(Self {
            item: item.clone(),
            size: found.size,
            fit,
            outline,
            credit: found.shown_credit().map(str::to_owned),
            masses,
        })
    }
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
