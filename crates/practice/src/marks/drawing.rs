//! Marks the kinds build theirs from (S97, S98): what is off as rings and run-ons, segments, the
//! horizon run on, a box's edges; and the comparison of a picture laid over its copy as strongly as
//! the user sets, which colour copies and whole copies share.

use crate::geometry::off::Off;
use crate::geometry::perspective::PerspectiveBox;
use crate::marks::asked::{Asked, Compare};
use crate::marks::card::{CardItem, Say, WheelSize};
use crate::marks::compared::{Compared, Comparison, Seen, Verdict};
use crate::marks::mark::{Clip, Figure, In, LinePen, Place, SheetMark, Source};
use crate::maths::path::Point;
use crate::records::metric::Metrics;

/// How far either side of the frame a construction runs the horizon on (frame units): past the
/// farthest vanishing point (`FAR`); the sheet's edge cuts it.
const RUN_ON: f64 = 3.5;
/// A point's cross's arm, frame units.
const CROSS: f64 = 0.03;

/// What is off over the copy (S97, S98), in the marking red: a point or a middle ringed, an edge
/// run on dashed past where it heads (that point ringed), a true vertical; `clip`ped.
pub(crate) fn off_marks(off: &[Off], clip: Clip) -> Vec<SheetMark> {
    let mut marks = Vec::new();
    for off in off {
        match off {
            Off::Ring(p) => marks.push(SheetMark::cut(In::Copy, Figure::Ring(*p))),
            Off::RunOn { line, past } => {
                marks.push(SheetMark::dashed(In::Copy, line.to_vec(), LinePen::Off));
                marks.extend(past.map(|p| SheetMark::cut(In::Copy, Figure::Ring(p))));
            }
            Off::Plumb(line) => marks.push(SheetMark::line(In::Copy, line.to_vec(), LinePen::Off)),
        }
    }
    marks.into_iter().map(|mark| mark.clipped(clip)).collect()
}

/// The segment `a` to `b` in `pen`, cut at the sheet's edge.
pub(crate) fn segment(on: In, [a, b]: [Point; 2], pen: LinePen) -> SheetMark {
    SheetMark::line(on, vec![a, b], pen)
}

/// The horizon at `height` run on faint far either side of the copy's frame.
pub(crate) fn run_on(height: f64) -> SheetMark {
    segment(
        In::Copy,
        [[-RUN_ON, height], [1.0 + RUN_ON, height]],
        LinePen::Faint,
    )
}

/// `seen`'s edges in `pen`, the hidden ones dashed in `hidden` or left out.
pub(crate) fn box_edges(
    on: In,
    seen: &PerspectiveBox,
    (pen, hidden): (LinePen, Option<LinePen>),
) -> Vec<SheetMark> {
    (seen.edges().into_iter().zip(seen.hidden()))
        .filter_map(|((a, b, _), is_hidden)| match (is_hidden, hidden) {
            (false, _) => Some(segment(on, [a, b], pen)),
            (true, Some(hidden)) => Some(SheetMark::dashed(on, vec![a, b], hidden)),
            (true, None) => None,
        })
        .collect()
}

/// A picture laid over its copy in place as strongly as the user sets (a long colour copy, a whole
/// copy, its division hidden under it, S95): its corners (frame units, its frame's shape, S99); on
/// the card the slider and the point the eyedropper compared.
#[derive(Debug, Clone, PartialEq)]
pub struct PictureCompared {
    pub corners: [Point; 4],
    /// Its numbers: a whole copy keeps its own division's.
    pub metrics: Metrics,
    /// Who judges it: the eye, where no answer but the picture itself is held (a whole copy, a
    /// painted kind whose areas or masses no pack holds now); nobody on a long colour copy, a
    /// project no result is kept for (S158).
    pub verdict: Verdict,
}

impl Compared for PictureCompared {
    fn marks(&self, seen: &Seen<'_>) -> Vec<SheetMark> {
        let place = Place::Corners(self.corners);
        let drawn = Figure::Picture {
            source: Source::Pack,
            place,
            shown: seen.shown,
        };
        vec![SheetMark::free(In::Copy, drawn).clipped(Clip::Laid)]
    }

    fn card(&self, seen: &Seen<'_>) -> Vec<CardItem> {
        let probe = seen.probe.map(|(original, copy)| CardItem::Wheel {
            original,
            copy,
            size: WheelSize::Middle,
        });
        std::iter::once(CardItem::Slider).chain(probe).collect()
    }

    fn metrics(&self) -> Metrics {
        self.metrics.clone()
    }

    fn verdict(&self) -> Verdict {
        self.verdict
    }
}

/// `asked`'s copy compared by its pack picture laid over it as strongly as the user sets, with
/// `metrics` (a whole copy's its division's, else none), the eye judging; none without its
/// picture.
pub(crate) fn picture_laid(asked: &Asked<'_>, metrics: Metrics) -> Option<Compare> {
    laid_over(asked, metrics, Verdict::Eye)
}

/// A long colour copy compared by its picture laid over it as `picture_laid` lays it, no result
/// asked or kept, as a drawing from life's and a cold-warm copy's (S158: nothing reads a
/// project's result).
pub(crate) fn project_laid(asked: &Asked<'_>) -> Option<Compare> {
    laid_over(asked, Metrics::new(), Verdict::Nobody)
}

fn laid_over(asked: &Asked<'_>, metrics: Metrics, verdict: Verdict) -> Option<Compare> {
    let corners = asked.corners()?;
    Some(Compare::Made(Comparison::of(PictureCompared {
        corners,
        metrics,
        verdict,
    })))
}

/// A checked estimate's or point's card: unless nothing was read, the picture speaks; then that no
/// mark (`marked`: a mark is drawn) or no straight line was found. Construction after.
pub(crate) fn nothing_read(nothing: bool, marked: bool) -> Vec<CardItem> {
    let said = nothing.then(|| Say::plain(if marked { "no-mark" } else { "no-edges" }));
    (said.map(CardItem::Says).into_iter())
        .chain([CardItem::Construction])
        .collect()
}

/// A cross at `p` in the copy's frame, in `pen`.
pub(crate) fn cross(p: Point, pen: LinePen) -> Vec<SheetMark> {
    [[CROSS, 0.0], [0.0, CROSS]]
        .map(|[dx, dy]| {
            segment(
                In::Copy,
                [[p[0] - dx, p[1] - dy], [p[0] + dx, p[1] + dy]],
                pen,
            )
        })
        .to_vec()
}

/// The exercise's pack picture in its original's frame.
pub(crate) fn pictured() -> Vec<SheetMark> {
    vec![SheetMark::free(In::Original, Figure::Pictured)]
}
