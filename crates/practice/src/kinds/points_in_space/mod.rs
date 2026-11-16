//! Points in space (`Kind::Space`, S90, V大's 点定位): a box's face and the five tricks, their check,
//! their How (`demo.rs`), tiles, lessons and answers as a hand draws them.
//!
//! A lesson (a point about 2 minutes) holds two, the tricks in the course's order: the first
//! level the face's centre and its halves (both by its diagonals), the second its thirds and the
//! next face, the third a mirror with one of the four before in turn.

pub(crate) mod demo;
pub(crate) mod marks;
pub(crate) mod space;
pub(crate) mod space_check;

use crate::course::picker::laid;
use crate::demos::demo::Demo;
use crate::demos::demo_points::crossed;
use crate::kinds::{
    Blank, Described, Facts, Kind, Original, SkillLevel, ToolSet, Turn, held, place,
};
use crate::marks::compared::{Compared, DrawnCopy};
use crate::marks::mark::SheetMark;
use crate::marks::tile::{FACE, TileMark, boxed};
use crate::maths::path::Point;
use crate::packs::pack::ItemRef;
use crate::progress::scaled;
use crate::records::metric::{Metric, Metrics, value};
use crate::sheets::exercise::Exercise;
use space::{SpaceAnswer, SpaceTask, Trick};
use space_check::NEAR_FACE;
use std::sync::Arc;

pub(crate) struct PointsInSpace;

/// A point in space off by about a pen's width on a face of 0.4 of its frame, a share of the face:
/// where the picture's marks begin. The share is a face's, the marks the picture's, so the result
/// band the progress fit holds the number within keeps it honest whatever the face's size.
const CLEAN_FACE: f64 = 0.03;

/// Its topics, a trick each in `Trick::ALL`'s order: each has its own instructions.
const FACTS: [Facts; 5] = [
    Facts::of("centre", ToolSet::DRAWING, 120).constructed(),
    Facts::of("halves", ToolSet::DRAWING, 120).constructed(),
    Facts::of("thirds", ToolSet::DRAWING, 120).constructed(),
    Facts::of("next-face", ToolSet::DRAWING, 120).constructed(),
    Facts::of("mirror", ToolSet::DRAWING, 120).constructed(),
];
/// The first two levels' tricks; the four a mirror takes in turn at the third.
const FIRST: [Trick; 2] = [Trick::Centre, Trick::Halves];
const SECOND: [Trick; 2] = [Trick::Thirds, Trick::Step];
const BESIDE: [Trick; 4] = [Trick::Centre, Trick::Halves, Trick::Thirds, Trick::Step];

impl Described for PointsInSpace {
    fn code(&self) -> u8 {
        12
    }

    fn facts(&self) -> &'static [Facts] {
        &FACTS
    }

    fn key(&self) -> &'static str {
        "points-in-space"
    }

    fn variant(&self, kind: Kind) -> Option<u8> {
        match kind {
            Kind::Space(trick) => Some(place(&Trick::ALL, &trick)),
            _ => None,
        }
    }

    fn of_variant(&self, variant: u8) -> Option<Kind> {
        Trick::ALL
            .get(usize::from(variant))
            .map(|t| Kind::Space(*t))
    }

    fn topic(&self, kind: Kind) -> &'static Facts {
        let Kind::Space(trick) = kind else {
            return &FACTS[0];
        };
        &FACTS[usize::from(place(&Trick::ALL, &trick))]
    }

    fn generate(&self, exercise: &Exercise) -> Arc<dyn Original> {
        let Kind::Space(trick) = exercise.kind else {
            return held(Blank);
        };
        held(SpaceTask::generate(trick, exercise.seed))
    }

    fn how(&self, kind: Kind, _cells: usize) -> Option<Demo> {
        let Kind::Space(trick) = kind else {
            return None;
        };
        demo::trick(trick)
    }

    fn lesson(&self, level: SkillLevel, turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        let tricks = match level {
            SkillLevel::First => FIRST,
            SkillLevel::Second => SECOND,
            SkillLevel::Third => [Trick::Mirror, BESIDE[turn.turn % BESIDE.len()]],
        };
        laid(tricks.map(Kind::Space))
    }

    /// The copy off the answer as a share of the face, cut at `CLEAN_FACE` and `NEAR_FACE`.
    fn measured(&self, _exercise: &Exercise, metrics: &Metrics) -> Option<f64> {
        let off = value(metrics, Metric::SpaceOff)?;
        Some(scaled(off, [CLEAN_FACE, NEAR_FACE]))
    }

    /// The box with its front face shaded, a point on it.
    fn glyph(&self) -> Vec<TileMark> {
        let mut marks = vec![TileMark::Tint(FACE.to_vec(), 0.18)];
        marks.extend(boxed([0.0, 0.0], 1.0));
        marks.push(TileMark::Dot([20.0, 22.2]));
        marks
    }
}

/// A box with a face to fix a point in space on.
impl Original for SpaceTask {
    fn givens(&self) -> Vec<SheetMark> {
        marks::givens(self)
    }

    fn check(&self, copy: &DrawnCopy) -> Option<Arc<dyn Compared>> {
        Some(marks::SpaceCompared::of(self, copy))
    }

    /// A small cross on a point, a stroke along each line.
    fn answer(&self) -> Vec<Vec<Point>> {
        match SpaceTask::answer(self) {
            SpaceAnswer::Point(at) => crossed(at).to_vec(),
            SpaceAnswer::Lines(lines) => lines.iter().map(|line| line.to_vec()).collect(),
        }
    }
}
