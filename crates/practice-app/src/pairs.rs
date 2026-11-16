//! A copy as a blind pair's card shows it (S147, 比一比), read from its sheet on the storage worker
//! as the book reads a sheet: the exercise as its first Compare showed it, small. Its frame's shape;
//! the paint strokes and fills laid in its frame before that Compare (the copy as first made, the
//! sketch pen's lines with them); the original laid over in see-through red where it is lines (a
//! line's, a group's), the exact answer where its kind lays one (hidden parts dashed), else its
//! picture's outline; its pack picture under it faintly; a colour's swatch beside it. Kind-blind:
//! what it lays is what the original and its picture say, never which kind it is.

use std::sync::Arc;

use openpaint_core::{Artwork, Layer, StrokeTool};
use openpaint_practice::{
    CopyRef, DrawnCopy, Figure, Frame, Given, Layout, LinePen, Outcome, Pack, Point, Rgb8, Seen,
    Sheet, SheetMark, SheetPicture, local_day,
};

use crate::copy::rgb8;
use crate::pictures::PictureImage;

/// How strongly the picture shows under a card's copy: the overlay's first strength.
pub const UNDER: f32 = crate::projection::SHOWN;
/// Most points a stroke keeps on a card.
const CARD_POINTS: usize = 48;

/// A copy as its card shows it.
#[derive(Debug, Clone, PartialEq)]
pub struct CopyCard {
    pub copy: CopyRef,
    /// Its frame's width and height in frame units (1 along the longer side).
    pub extent: Point,
    /// What was laid in it before its first Compare, in the order it lies, in frame units.
    pub marks: Vec<CardMark>,
    /// The original or the answer laid over it in red, each line with whether it is dashed.
    pub laid: Vec<(Vec<Point>, bool)>,
    /// Its pack picture and where it lies in the frame (top left, bottom right, frame units).
    pub picture: Option<(Arc<PictureImage>, [Point; 2])>,
    /// A colour's original swatch.
    pub swatch: Option<Rgb8>,
    /// The local day it was compared on and its result.
    pub day: i64,
    pub result: Option<Outcome>,
}

/// A stroke or a fill of a copy, in frame units.
#[derive(Debug, Clone, PartialEq)]
pub enum CardMark {
    Line {
        points: Vec<Point>,
        colour: Rgb8,
        /// Its width, frame units.
        width: f64,
    },
    Fill {
        rect: [Point; 2],
        colour: Rgb8,
    },
}

/// What a card is read from: the sheet's artwork and record, what the copy's exercise printed, its
/// pack picture as the sheet holds it and that picture read for the screen.
#[derive(Clone, Copy)]
pub struct CardSource<'a> {
    pub artwork: &'a Artwork,
    pub record: &'a Sheet,
    pub printed: &'a [SheetMark],
    pub picture: Option<&'a SheetPicture>,
    pub image: Option<&'a Arc<PictureImage>>,
}

impl CopyCard {
    /// Copy `copy` as `source` holds it, its day on a clock `offset` minutes from UTC; none where
    /// the sheet holds no such exercise.
    #[must_use]
    pub fn of(source: &CardSource<'_>, copy: CopyRef, offset: i32) -> Option<Self> {
        let (artwork, record) = (source.artwork, source.record);
        let index = usize::from(copy.exercise);
        let laid = record.laid(index)?;
        let canvas = artwork.canvas();
        let frame = Layout::on(record, [canvas.width(), canvas.height()])
            .pairs
            .get(index)?
            .copy;
        let before = laid.first.as_ref().map(|first| first.corrected_from);
        let marks = (artwork.layers().iter())
            .flat_map(|layer| card_marks(layer, &frame, before))
            .collect();
        let original = laid.exercise.original();
        let picture = (source.picture).zip(source.image).map(|(shown, image)| {
            let corners = shown.fit.corners();
            (Arc::clone(image), [corners[0], corners[2]])
        });
        let at = laid.first.as_ref().map_or(record.started, |first| first.at);
        let printed = source.printed.iter().find_map(|mark| match mark.drawn {
            Figure::Swatch(colour) => Some(colour),
            _ => None,
        });
        Some(Self {
            copy,
            extent: frame.extent(),
            marks,
            laid: laid_over(&*original, source.picture),
            picture,
            swatch: printed.or_else(|| original.swatch()),
            day: local_day(at, offset),
            result: laid.result.map(|(outcome, _)| outcome),
        })
    }
}

/// Copy `copy`'s card read from its stored sheet: `artwork` and `record`, what its exercises
/// printed (`printed`, in order), its picture as `packs` hold it and `image` (read for the screen).
#[must_use]
pub fn stored_card(
    (artwork, record, printed): (&Artwork, &Sheet, &[Given]),
    (packs, image): (&[Pack], Option<PictureImage>),
    copy: CopyRef,
    offset: i32,
) -> Option<CopyCard> {
    let laid = record.laid(usize::from(copy.exercise))?;
    let picture = (laid.exercise.picture.as_ref()).and_then(|item| SheetPicture::of(item, packs));
    let image = image.map(Arc::new);
    let given = printed.get(usize::from(copy.exercise));
    let source = CardSource {
        artwork,
        record,
        printed: given.map_or(&[], |given| given.marks.as_slice()),
        picture: picture.as_ref(),
        image: image.as_ref(),
    };
    CopyCard::of(&source, copy, offset)
}

/// What lies over the copy in red: the original's lines, else the answer its kind lays (its
/// check made without reading a stroke), else the picture's outline.
fn laid_over(
    original: &dyn openpaint_practice::Original,
    picture: Option<&SheetPicture>,
) -> Vec<(Vec<Point>, bool)> {
    if let Some(lines) = original.lines() {
        return lines.iter().map(|line| (line.clone(), false)).collect();
    }
    let seen = Seen {
        construction: false,
        shown: UNDER,
        chosen: 0,
        outline: &[],
        grey: false,
        view: 0,
        fit: None,
        probe: None,
    };
    let answer: Vec<(Vec<Point>, bool)> = (original.check(&DrawnCopy::default()).iter())
        .flat_map(|checked| checked.marks(&seen))
        .filter(|mark| mark.on == openpaint_practice::In::Copy)
        .filter_map(|mark| match mark.drawn {
            Figure::Line {
                points,
                pen: LinePen::Answer | LinePen::Laid,
                dashed,
            } => Some((points, dashed)),
            _ => None,
        })
        .collect();
    if !answer.is_empty() {
        return answer;
    }
    (picture
        .and_then(|picture| picture.outline.clone())
        .into_iter())
    .flatten()
    .map(|line| (line, false))
    .collect()
}

/// `layer`'s paint strokes begun in `frame` (those before stroke `before` where given) and fills
/// there, in the order they lie, in frame units; none while it is hidden.
fn card_marks(layer: &Layer, frame: &Frame, before: Option<u64>) -> Vec<CardMark> {
    if !layer.props().visible() {
        return Vec::new();
    }
    let unit = frame.unit();
    let point = |x: f32, y: f32| frame.to_frame([f64::from(x), f64::from(y)]);
    let mut placed: Vec<(usize, CardMark)> = Vec::new();
    for (at, stroke) in layer.strokes().iter().enumerate() {
        let samples = stroke.samples();
        let Some(first) = samples.first() else {
            continue;
        };
        let start = [
            f64::from(first.position().x()),
            f64::from(first.position().y()),
        ];
        let early = before.is_none_or(|before| stroke.id().get() < before);
        if stroke.style().tool() != StrokeTool::Paint || !frame.holds(start) || !early {
            continue;
        }
        let step = samples.len().div_ceil(CARD_POINTS).max(1);
        let mut points: Vec<Point> = (samples.iter().step_by(step))
            .map(|s| point(s.position().x(), s.position().y()))
            .collect();
        if let Some(last) = samples.last().filter(|_| (samples.len() - 1) % step != 0) {
            points.push(point(last.position().x(), last.position().y()));
        }
        let style = stroke.style();
        let line = CardMark::Line {
            points,
            colour: rgb8(style.color()),
            width: f64::from(style.size()) / unit,
        };
        placed.push((2 * at + 1, line));
    }
    for edit in layer.edits() {
        let Some(fill) = edit.fill() else {
            continue;
        };
        let ([x, y], [w, h]) = (fill.mask().origin(), fill.mask().size());
        let [x, y, w, h] = [x, y, w, h].map(f64::from);
        if !frame.contains([x + w / 2.0, y + h / 2.0], 0.0) {
            continue;
        }
        // A fill floods the canvas before the frame cuts it: held within the frame.
        let extent = frame.extent();
        let held = |p: Point| [p[0].clamp(0.0, extent[0]), p[1].clamp(0.0, extent[1])];
        let rect = [[x, y], [x + w, y + h]].map(|p| held(frame.to_frame(p)));
        let colour = rgb8(fill.color());
        placed.push((2 * fill.after(), CardMark::Fill { rect, colour }));
    }
    placed.sort_by_key(|(at, _)| *at);
    placed.into_iter().map(|(_, mark)| mark).collect()
}
