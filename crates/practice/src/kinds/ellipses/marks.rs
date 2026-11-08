//! An ellipse's marks (S84, S97): its frame printed grey in the one frame. Compared, the exact
//! ellipse in see-through red and a ring on each side's middle the copy falls short of or runs
//! past; Construction lays the frame's diagonals and middle lines faint, the middles dotted over
//! the red. The card: rounder or flatter, turned which way, which sides it misses; or that it fits.

use std::sync::Arc;

use crate::geometry::ellipse::EllipseGuide;
use crate::geometry::ellipse_sides::{Reach, Side};
use crate::geometry::off::Off;
use crate::kinds::ellipses::ellipse_check::{EllipseCheck, Roundness, Tilt};
use crate::marks::card::{Arg, CardItem, Say};
use crate::marks::compared::{Compared, DrawnCopy, Seen};
use crate::marks::drawing::{off_marks, segment};
use crate::marks::mark::{Clip, Figure, In, LinePen, SheetMark};
use crate::maths::path::Point;

/// An ellipse drawn in pieces.
const PIECES: usize = 96;

/// The guide's frame printed grey in the one frame.
pub(crate) fn givens(guide: &EllipseGuide) -> Vec<SheetMark> {
    let [a, b, c, d] = guide.corners;
    vec![SheetMark::line(
        In::Copy,
        vec![a, b, c, d, a],
        LinePen::Printed,
    )]
}

/// An ellipse compared.
#[derive(Debug, Clone, PartialEq)]
pub struct EllipseCompared {
    pub guide: EllipseGuide,
    pub check: EllipseCheck,
}

impl EllipseCompared {
    /// The ellipse drawn: the corrections once there are some (the latest try, not one averaged
    /// with the first), else every line drawn.
    pub fn of(guide: &EllipseGuide, copy: &DrawnCopy) -> Arc<dyn Compared> {
        let corrections = (copy.corrected.as_ref()).map(|(_, corrections)| corrections);
        let drawn = (corrections.filter(|pieces| !pieces.is_empty())).unwrap_or(&copy.all);
        let lines: Vec<&[Point]> = drawn.iter().map(Vec::as_slice).collect();
        Arc::new(Self {
            guide: *guide,
            check: EllipseCheck::of(guide, &lines),
        })
    }
}

impl Compared for EllipseCompared {
    fn marks(&self, seen: &Seen<'_>) -> Vec<SheetMark> {
        let guide = &self.guide;
        let mut marks = Vec::new();
        if seen.construction {
            let (c, t) = (guide.corners, guide.touches);
            let lines = [[c[0], c[2]], [c[1], c[3]], [t[0], t[2]], [t[1], t[3]]];
            marks.extend(lines.map(|line| segment(In::Copy, line, LinePen::Faint)));
        }
        let outline = guide.exact.outline(PIECES);
        marks.push(SheetMark::line(In::Copy, outline, LinePen::Answer));
        let missed: Vec<Off> = (self.check.sides.iter())
            .map(|missed| Off::Ring(missed.middle))
            .collect();
        marks.extend(off_marks(&missed, Clip::Sheet));
        // The middles dotted over the red, which runs through them.
        let dots = (guide.touches.iter()).filter(|_| seen.construction);
        let dot = |at: &Point| Figure::Dot {
            at: *at,
            small: false,
        };
        marks.extend(dots.map(|at| SheetMark::free(In::Copy, dot(at))));
        marks
    }

    fn card(&self, _seen: &Seen<'_>) -> Vec<CardItem> {
        let check = &self.check;
        if check.fitted.is_none() {
            return vec![CardItem::Says(Say::plain("no-ellipse"))];
        }
        let said = if check.roundness.is_none() && check.tilt.is_none() && check.sides.is_empty() {
            Say::plain("ellipse-fits")
        } else {
            differs(check)
        };
        vec![CardItem::Says(said), CardItem::Construction]
    }
}

/// How the ellipse differs: its roundness, its tilt and each side it misses, as words.
fn differs(check: &EllipseCheck) -> Say {
    let roundness = check.roundness.map(|r| match r {
        Roundness::Rounder => "rounder",
        Roundness::Flatter => "flatter",
    });
    let tilt = check.tilt.map(|t| match t {
        Tilt::Clockwise => "clockwise",
        Tilt::Anticlockwise => "anticlockwise",
    });
    let sides = check.sides.iter().map(|missed| {
        let side = match missed.side {
            Side::Top => "top",
            Side::Right => "right",
            Side::Bottom => "bottom",
            Side::Left => "left",
        };
        let reach = match missed.reach {
            Reach::Short => "short",
            Reach::Past => "past",
        };
        [Arg::Word(side), Arg::Word(reach)]
    });
    let args = [roundness, tilt]
        .map(|word| Arg::Word(word.unwrap_or("")))
        .into_iter()
        .chain(sides.flatten())
        .collect();
    Say {
        key: "ellipse-differs",
        args,
    }
}
