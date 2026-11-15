//! An estimate's marks (S90, S98): copied by eye, it keeps its original beside the copy: a point's
//! dot in the original's frame; a proportion's two segments (the end the share counts from ringed,
//! the other barred), the original's divided by a dot; an angle's line in the original's frame;
//! what the copy's frame gives (its segment, the dot an angle's copy runs through) printed grey.
//! Compared, the exact answer in see-through red (a cross for a point, a bar across a segment, the
//! line) and only what is off; Construction lays the way to see it faint in both frames (the point
//! run to the edges with their halves and thirds ticked, the segments' halves and thirds, the
//! lines run on and an upright cross through each).

use std::sync::Arc;

use crate::kinds::estimates::flat::{Flat, FlatAnswer, FlatCheck};
use crate::marks::card::CardItem;
use crate::marks::compared::{Compared, DrawnCopy, Seen, Verdict};
use crate::marks::drawing::{cross, nothing_read, off_marks, segment};
use crate::marks::mark::{Clip, Figure, In, LinePen, SheetMark};
use crate::maths::path::{Point, across_frame};
use crate::records::metric::Metrics;

/// Frame units: a point's cross's arm, a bar across a segment's half, a tick's half (at a half,
/// at a third), a cross's arm through an angle's line.
const BAR: f64 = 0.035;
const HALF_TICK: f64 = 0.03;
const THIRD_TICK: f64 = 0.018;
const CROSS_ARM: f64 = 0.1;

/// The estimate's givens in its two frames.
pub(crate) fn givens(flat: &Flat) -> Vec<SheetMark> {
    let given = |on: In, at: Point, pen: LinePen| SheetMark::free(on, Figure::Given { at, pen });
    match *flat {
        Flat::Point { at } => vec![given(In::Original, at, LinePen::Ink)],
        Flat::Proportion {
            original,
            share,
            copy,
        } => {
            let mut marks = barred(In::Original, original, LinePen::Ink);
            marks.extend(barred(In::Copy, copy, LinePen::Printed));
            marks.push(given(In::Original, along(original, share), LinePen::Ink));
            marks
        }
        Flat::Angle { original, through } => vec![
            segment(In::Original, original, LinePen::Ink),
            given(In::Copy, through, LinePen::Printed),
        ],
    }
}

/// An estimate compared.
#[derive(Debug, Clone, PartialEq)]
pub struct FlatCompared {
    pub flat: Flat,
    pub check: FlatCheck,
}

impl FlatCompared {
    pub fn of(flat: &Flat, copy: &DrawnCopy) -> Arc<dyn Compared> {
        let lines: Vec<&[Point]> = copy.all.iter().map(Vec::as_slice).collect();
        Arc::new(Self {
            flat: *flat,
            check: FlatCheck::of(flat, &lines),
        })
    }
}

impl Compared for FlatCompared {
    fn marks(&self, seen: &Seen<'_>) -> Vec<SheetMark> {
        let flat = &self.flat;
        let mut marks = Vec::new();
        if seen.construction {
            marks.extend(way(In::Original, flat));
            marks.extend(way(In::Copy, flat));
        }
        match (flat.answer(), flat) {
            (FlatAnswer::Point(p), Flat::Proportion { copy, .. }) => {
                let [nx, ny] = normal(*copy);
                let ends = [-BAR, BAR].map(|s| [p[0] + s * nx, p[1] + s * ny]);
                marks.push(segment(In::Copy, ends, LinePen::Answer));
            }
            (FlatAnswer::Point(p), _) => marks.extend(cross(p, LinePen::Answer)),
            (FlatAnswer::Line(line), _) => marks.push(segment(In::Copy, line, LinePen::Answer)),
        }
        marks.extend(off_marks(&self.check.off, Clip::Sheet));
        marks
    }

    fn card(&self, _seen: &Seen<'_>) -> Vec<CardItem> {
        let marked = matches!(self.flat, Flat::Point { .. } | Flat::Proportion { .. });
        let reason = self.check.reason(&self.flat).map(CardItem::Says);
        (reason.into_iter())
            .chain(nothing_read(self.check.metrics.is_empty(), marked))
            .collect()
    }

    fn metrics(&self) -> Metrics {
        self.check.metrics.clone()
    }

    fn verdict(&self) -> Verdict {
        self.check.verdict(&self.flat)
    }
}

/// the original's at its givens.
fn way(on: In, flat: &Flat) -> Vec<SheetMark> {
    let faint = |line: [Point; 2]| segment(on, line, LinePen::Faint);
    let copy = on == In::Copy;
    let mut marks = Vec::new();
    match (*flat, flat.answer()) {
        (Flat::Point { at: p }, _) => {
            marks.push(faint([[0.0, p[1]], [1.0, p[1]]]));
            marks.push(faint([[p[0], 0.0], [p[0], 1.0]]));
            for share in [0.5, 1.0 / 3.0, 2.0 / 3.0] {
                let tick = if share == 0.5 { HALF_TICK } else { THIRD_TICK };
                marks.push(faint([[share, 0.0], [share, tick]]));
                marks.push(faint([[0.0, share], [tick, share]]));
            }
        }
        (
            Flat::Proportion {
                original,
                copy: given,
                ..
            },
            _,
        ) => marks.extend(ticked(on, if copy { given } else { original })),
        (Flat::Angle { original, through }, FlatAnswer::Line(line)) => {
            let (middle, [a, b]) = if copy {
                (through, line)
            } else {
                (along(original, 0.5), original)
            };
            marks.extend(across_frame(a, [b[0] - a[0], b[1] - a[1]]).map(faint));
            for d in [[CROSS_ARM, 0.0], [0.0, CROSS_ARM]] {
                let ends = [-1.0, 1.0].map(|s| [middle[0] + s * d[0], middle[1] + s * d[1]]);
                marks.push(faint(ends));
            }
        }
        (Flat::Angle { .. }, FlatAnswer::Point(_)) => {}
    }
    marks
}

/// `line` in the frame `on` with a short bar across its last end and its first ringed (the share
/// is counted from it, in both frames alike).
fn barred(on: In, line: [Point; 2], pen: LinePen) -> Vec<SheetMark> {
    let [nx, ny] = normal(line);
    let [first, last] = line;
    let bar = [-1.0, 1.0].map(|s| [last[0] + s * THIRD_TICK * nx, last[1] + s * THIRD_TICK * ny]);
    let counted = Figure::Counted {
        at: first,
        radius: THIRD_TICK,
        pen,
    };
    vec![
        segment(on, line, pen),
        segment(on, bar, pen),
        SheetMark::free(on, counted),
    ]
}

/// `line`'s halves and thirds ticked across it, faint.
fn ticked(on: In, line: [Point; 2]) -> Vec<SheetMark> {
    let [nx, ny] = normal(line);
    [0.5, 1.0 / 3.0, 2.0 / 3.0]
        .map(|share| {
            let half = if share == 0.5 { HALF_TICK } else { THIRD_TICK };
            let p = along(line, share);
            let tick = [-1.0, 1.0].map(|s| [p[0] + s * half * nx, p[1] + s * half * ny]);
            segment(on, tick, LinePen::Faint)
        })
        .to_vec()
}

fn along([a, b]: [Point; 2], share: f64) -> Point {
    [a[0] + (b[0] - a[0]) * share, a[1] + (b[1] - a[1]) * share]
}

/// `line`'s unit normal.
fn normal([a, b]: [Point; 2]) -> Point {
    let d = [b[0] - a[0], b[1] - a[1]];
    let long = d[0].hypot(d[1]).max(f64::MIN_POSITIVE);
    [-d[1] / long, d[0] / long]
}
