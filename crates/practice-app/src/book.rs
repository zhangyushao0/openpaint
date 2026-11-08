//! The practice book as one UI frame sees it (S89): the book of the log's records on the device's
//! clock (`openpaint_practice::Book`: the road, the days, the colour's first picks, the lines
//! shown, the seals), today's local day, each record's sheet, and what was read of the sheets the
//! book asked for (`SheetRead`, on the storage worker: the store reads a sheet without writing).
//!
//! A sheet read keeps what the book shows of it: its strokes and fills as marks for a small
//! picture of it, each single line's original and first copy (the strokes before its first
//! Compare, the sketch pen's left out) and each colour's original and first pick (the eye's own,
//! before any correction), whatever version wrote its record.

use std::collections::BTreeMap;
use std::sync::Arc;

use openpaint_core::{Artwork, Layer, StrokeTool};
use openpaint_practice::{Book, Family, Kept, Pack, Point, Rgb8, SessionRecord, Sheet};

use crate::copy::{ALL_STROKES, colour_copy, line_pieces, rgb8};
use crate::projection::laid;
use openpaint_app::ProjectId;
use openpaint_app::Title;

/// Most points a stroke keeps in a sheet's small picture.
const MARK_POINTS: usize = 24;

/// The practice book for one UI frame.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BookProjection {
    pub book: Arc<Book>,
    /// The local day now (`openpaint_practice::local_day`).
    pub today: i64,
    /// Every session, drill and copy, by id, as the book names and opens them.
    pub records: Arc<[BookRecord]>,
    /// The sheets read so far, by record.
    pub read: Arc<BTreeMap<u64, Arc<SheetRead>>>,
}

impl BookProjection {
    /// Session or copy `id`.
    #[must_use]
    pub fn record(&self, id: u64) -> Option<&BookRecord> {
        self.records.iter().find(|record| record.id == id)
    }
}

/// A session or copy as the book names and opens it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookRecord {
    pub id: u64,
    /// Its sheet, which opens it.
    pub sheet: ProjectId,
    /// Its number in its stage, among the copies, or among its kind's drills.
    pub number: u32,
    /// The kind a drill drills (S91); none for a session or a copy.
    pub drill: Option<Family>,
    /// A colour copy's name (its sheet's title); none for a session or a drill.
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
    /// What `record`'s sheet `artwork` shows the book, its pictures' frames as `packs` hold them.
    #[must_use]
    pub fn of(artwork: &Artwork, record: &SessionRecord, packs: &[Pack]) -> Self {
        let canvas = artwork.canvas();
        let size = [canvas.width(), canvas.height()];
        let sheet = laid(record, packs, size);
        let mut lines = Vec::new();
        let mut colours = Vec::new();
        let mut results: Vec<_> = record.results.iter().collect();
        results.sort_by_key(|result| result.index);
        for result in results {
            let index = result.index as usize;
            let (Some(exercise), Some(pair)) =
                (record.exercises.get(index), sheet.pairs.get(index))
            else {
                continue;
            };
            // Only what the book keeps is read: others' originals (an object's) take time to lay.
            let kept = exercise.kind.facts().kept;
            if kept == Kept::Nothing {
                continue;
            }
            let original = exercise.original();
            match (kept, original.lines(), original.swatch()) {
                (Kept::Line, Some(original), _) => {
                    let first = result.corrected_from.map_or(ALL_STROKES, |from| 0..from);
                    let copy = line_pieces(artwork, &pair.copy, first);
                    lines.push(LineRead {
                        index: result.index,
                        original: original.to_vec(),
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
        Self {
            size,
            frames: frames(&sheet, record),
            marks: artwork.layers().iter().flat_map(marks).collect(),
            lines,
            colours,
        }
    }
}

/// `sheet`'s frames as `record`'s exercises show them (top left to bottom right): a kind built in
/// one frame that one (S97), the others both.
fn frames(sheet: &Sheet, record: &SessionRecord) -> Vec<[[f32; 2]; 2]> {
    let one =
        |index: usize| (record.exercises.get(index)).is_some_and(|e| e.kind.family().constructed());
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
