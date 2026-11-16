//! The practice book as one UI frame sees it (S89): the book of the log's records on the device's
//! clock (`openpaint_practice::Book`: the days, the lines shown, the seals), today's local day,
//! each record's sheet, what was read of the sheets the book asked for (`SheetRead`, on the storage
//! worker: the store reads a sheet without writing), each skill's curve once fitted there and the
//! cards of its re-test's copies (S147).
//!
//! A sheet read keeps what the book shows of it: its strokes and fills as marks for a small
//! picture of it, each single line's original and first copy (the strokes before its first
//! Compare, the sketch pen's left out) and each colour's original and first pick (the eye's own,
//! before any correction), whatever version wrote its record; the originals as the sheet printed
//! them (S120d), no generator run.

use std::collections::BTreeMap;
use std::sync::Arc;

use openpaint_core::{Artwork, Layer, StrokeTool};
use openpaint_practice::{
    Book, CopyRef, Figure, Given, In, Kept, Layout, LinePen, Point, Progress, Rgb8, Sheet,
};

use crate::copy::{ALL_STROKES, colour_copy, line_pieces, rgb8};
use crate::lesson::LessonRow;
use crate::pairs::CopyCard;
use openpaint_app::ProjectId;
use openpaint_app::Title;

/// Most points a stroke keeps in a sheet's small picture.
const MARK_POINTS: usize = 24;

/// The practice book for one UI frame.
///
/// A day's sheet shows once, its picture the rows its lessons used.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BookProjection {
    pub book: Arc<Book>,
    /// The local day now (`openpaint_practice::local_day`).
    pub today: i64,
    /// Every sheet, by id, as the book names and opens it.
    pub records: Arc<[BookRecord]>,
    /// The sheets read so far, by record.
    pub read: Arc<BTreeMap<u64, Arc<SheetRead>>>,
    /// Each skill's curve (S147), once fitted for the records as they stand.
    pub progress: Option<Arc<Progress>>,
    /// The copies' cards read for the re-test pairs beside the curves, by copy.
    pub cards: Arc<BTreeMap<CopyRef, Arc<CopyCard>>>,
}

impl BookProjection {
    /// Sheet or copy `id`.
    #[must_use]
    pub fn record(&self, id: u64) -> Option<&BookRecord> {
        self.records.iter().find(|record| record.id == id)
    }
}

/// A sheet as the book names and opens it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookRecord {
    pub id: u64,
    /// Its sheet, which opens it.
    pub sheet: ProjectId,
    /// Each lesson on it, in order (a day's sheet holds the day's, S121c), as its row's label
    /// names it (S122: "lesson n · skill"); none on a colour copy's.
    pub rows: Vec<LessonRow>,
    /// A colour copy's name (its sheet's title); none for a day's sheet.
    pub copy: Option<Title>,
}

/// What the book shows of a sheet, read from its records.
#[derive(Debug, Clone, PartialEq)]
pub struct SheetRead {
    /// The sheet's size in pixels, its frames (top left to bottom right) and what lies on it in
    /// drawing order, in its pixels.
    pub size: [u32; 2],
    pub frames: Vec<[[f32; 2]; 2]>,
    pub marks: Vec<Mark>,
    /// Each single line compared, in the order of the exercises.
    pub lines: Vec<LineRead>,
    /// Each colour compared: its original and the first colour laid.
    pub colours: Vec<(Rgb8, Rgb8)>,
}

/// A stroke or a fill of a sheet, for its small picture.
#[derive(Debug, Clone, PartialEq)]
pub enum Mark {
    /// Some of a paint stroke's points, its colour and its diameter.
    Line {
        points: Vec<[f32; 2]>,
        colour: Rgb8,
        width: f32,
    },
    /// A fill's reach, from its top left to its bottom right.
    Fill { rect: [[f32; 2]; 2], colour: Rgb8 },
}

/// A single line compared: its original and the copy as first drawn, in frame units.
#[derive(Debug, Clone, PartialEq)]
pub struct LineRead {
    pub index: u32,
    pub original: Vec<Vec<Point>>,
    pub copy: Vec<Vec<Point>>,
}

impl SheetRead {
    /// What `record`'s sheet `artwork` shows the book, its frames as its exercises were laid and
    /// their originals as they printed (`printed`, each exercise's in order: a line's ink, a
    /// colour's swatch).
    #[must_use]
    pub fn of(artwork: &Artwork, record: &Sheet, printed: &[Given]) -> Self {
        let canvas = artwork.canvas();
        let sheet = Layout::on(record, [canvas.width(), canvas.height()]);
        let mut lines = Vec::new();
        let mut colours = Vec::new();
        for (index, (laid, given)) in record.exercises().zip(printed).enumerate() {
            let (Some(result), Some(pair)) = (&laid.first, sheet.pairs.get(index)) else {
                continue;
            };
            match (laid.exercise.kind.facts().kept, ink(given), swatch(given)) {
                (Kept::Line, Some(original), _) => {
                    let first = result.corrected_from.map_or(ALL_STROKES, |from| 0..from);
                    let copy = line_pieces(artwork, &pair.copy, first);
                    lines.push(LineRead {
                        index: u32::try_from(index).unwrap_or(u32::MAX),
                        original,
                        copy,
                    });
                }
                (Kept::Pick, _, Some(original)) => {
                    let laid = colour_copy(artwork, &pair.copy);
                    if let Some(first) = laid.picks.first() {
                        colours.push((original, *first));
                    }
                }
                _ => {}
            }
        }
        // The rows used, the paper laid ahead below them left out (S121c).
        let used = Layout::used(record);
        let size = if record.is_copy() {
            sheet.size
        } else {
            [used[0].min(sheet.size[0]), used[1].min(sheet.size[1])]
        };
        Self {
            size,
            frames: frames(&sheet, record),
            marks: artwork.layers().iter().flat_map(marks).collect(),
            lines,
            colours,
        }
    }
}

/// The lines an exercise printed in ink on its original's frame (a line's); none without.
fn ink(given: &Given) -> Option<Vec<Vec<Point>>> {
    let lines: Vec<Vec<Point>> = (given.marks.iter())
        .filter_map(|mark| match (&mark.on, &mark.drawn) {
            (In::Original, Figure::Line { points, pen, .. }) if *pen == LinePen::Ink => {
                Some(points.clone())
            }
            _ => None,
        })
        .collect();
    (!lines.is_empty()).then_some(lines)
}

/// The swatch an exercise printed on its original's frame (a colour's); none without.
fn swatch(given: &Given) -> Option<Rgb8> {
    given
        .marks
        .iter()
        .find_map(|mark| match (&mark.on, &mark.drawn) {
            (In::Original, Figure::Swatch(colour)) => Some(*colour),
            _ => None,
        })
}

/// `sheet`'s frames as `record`'s exercises show them (top left to bottom right): a kind built in
/// one frame that one (S97), the others both.
fn frames(sheet: &Layout, record: &Sheet) -> Vec<[[f32; 2]; 2]> {
    let one =
        |index: usize| (record.laid(index)).is_some_and(|l| l.exercise.kind.facts().constructed);
    (sheet.pairs.iter().enumerate())
        .flat_map(|(index, pair)| {
            let original = (!one(index)).then_some(pair.original);
            original.into_iter().chain([pair.copy])
        })
        .map(|frame| {
            let [x, y] = frame.origin.map(|v| v as f32);
            let [w, h] = frame.size.map(|v| v as f32);
            [[x, y], [x + w, y + h]]
        })
        .collect()
}

/// `layer`'s paint strokes and fills in the order they lie on it (a fill after the strokes
/// under it); none while it is hidden.
fn marks(layer: &Layer) -> Vec<Mark> {
    if !layer.props().visible() {
        return Vec::new();
    }
    let mut placed: Vec<(usize, Mark)> = (layer.strokes().iter().enumerate())
        .filter(|(_, stroke)| stroke.style().tool() == StrokeTool::Paint)
        .map(|(at, stroke)| {
            let samples = stroke.samples();
            let step = samples.len().div_ceil(MARK_POINTS).max(1);
            let mut points: Vec<[f32; 2]> = (samples.iter().step_by(step))
                .map(|sample| [sample.position().x(), sample.position().y()])
                .collect();
            if let Some(last) = samples.last().filter(|_| (samples.len() - 1) % step != 0) {
                points.push([last.position().x(), last.position().y()]);
            }
            let style = stroke.style();
            let line = Mark::Line {
                points,
                colour: rgb8(style.color()),
                width: style.size(),
            };
            (2 * at + 1, line)
        })
        .collect();
    for edit in layer.edits() {
        if let Some(fill) = edit.fill() {
            let ([x, y], [w, h]) = (fill.mask().origin(), fill.mask().size());
            let [x, y, w, h] = [x, y, w, h].map(|v| v as f32);
            let rect = [[x, y], [x + w, y + h]];
            let colour = rgb8(fill.color());
            placed.push((2 * fill.after(), Mark::Fill { rect, colour }));
        }
    }
    placed.sort_by_key(|(at, _)| *at);
    placed.into_iter().map(|(_, mark)| mark).collect()
}
