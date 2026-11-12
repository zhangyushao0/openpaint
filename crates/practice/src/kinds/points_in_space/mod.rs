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
use crate::kinds::{Described, Facts, Kind, Original, SkillLevel, Tools, Turn, place};
use crate::marks::compared::{Compared, DrawnCopy};
use crate::marks::mark::SheetMark;
use crate::marks::tile::{FACE, TileMark, boxed};
use crate::maths::path::Point;
use crate::packs::pack::ItemRef;
use crate::sheets::exercise::Exercise;
use crate::sheets::family::Family;
use space::{SpaceAnswer, SpaceTask, Trick};
use std::sync::Arc;

pub(crate) struct PointsInSpace;

const FACTS: [Facts; 5] = [
    Facts::of(Family::Centre, "centre", Tools::Drawing, 120).constructed(),
    Facts::of(Family::Halves, "halves", Tools::Drawing, 120).constructed(),
    Facts::of(Family::Thirds, "thirds", Tools::Drawing, 120).constructed(),
    Facts::of(Family::Step, "next-face", Tools::Drawing, 120).constructed(),
    Facts::of(Family::Mirror, "mirror", Tools::Drawing, 120).constructed(),
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

    fn family(&self, kind: Kind) -> Family {
        match kind {
            Kind::Space(trick) => trick.family(),
            _ => Family::Centre,
        }
    }

    fn original(&self, exercise: &Exercise) -> Original {
        let Kind::Space(trick) = exercise.kind else {
            return exercise.picture();
        };
        Original::Space(Box::new(SpaceTask::generate(trick, exercise.seed)))
    }

    fn givens(&self, _exercise: &Exercise, original: &Original) -> Vec<SheetMark> {
        match original {
            Original::Space(task) => marks::givens(task),
            _ => Vec::new(),
        }
    }

    fn check(&self, original: &Original, copy: &DrawnCopy) -> Option<Arc<dyn Compared>> {
        match original {
            Original::Space(task) => Some(marks::SpaceCompared::of(task, copy)),
            _ => None,
        }
    }

    fn how(&self, family: Family) -> Option<Demo> {
        demo::trick(family)
    }

    fn lesson(&self, level: SkillLevel, turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        let tricks = match level {
            SkillLevel::First => FIRST,
            SkillLevel::Second => SECOND,
            SkillLevel::Third => [Trick::Mirror, BESIDE[turn.turn % BESIDE.len()]],
        };
        laid(tricks.map(Kind::Space))
    }

    /// The box with its front face shaded, a point on it.
    fn glyph(&self) -> Vec<TileMark> {
        let mut marks = vec![TileMark::Tint(FACE.to_vec(), 0.18)];
        marks.extend(boxed([0.0, 0.0], 1.0));
        marks.push(TileMark::Dot([20.0, 22.2]));
        marks
    }

    /// A small cross on a point, a stroke along each line.
    fn answer(&self, original: &Original) -> Vec<Vec<Point>> {
        let Original::Space(task) = original else {
            return Vec::new();
        };
        match task.answer() {
            SpaceAnswer::Point(at) => crossed(at).to_vec(),
            SpaceAnswer::Lines(lines) => lines.iter().map(|line| line.to_vec()).collect(),
        }
    }

    /// The box with its front face shaded, and the trick's marks on the face.
    fn tile(&self, family: Family) -> Vec<TileMark> {
        let mut marks = vec![TileMark::Tint(FACE.to_vec(), 0.18)];
        marks.extend(boxed([0.0, 0.0], 1.0));
        marks.extend(trick(family));
        marks
    }
}

/// A trick's marks on the box's shaded face.
fn trick(family: Family) -> Vec<TileMark> {
    let [top_back, top_front, bottom_front, bottom_back] = FACE;
    let on =
        |a: [f32; 2], b: [f32; 2], t: f32| [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
    match family {
        Family::Centre => vec![
            TileMark::Line(vec![top_back, bottom_front]),
            TileMark::Line(vec![top_front, bottom_back]),
            TileMark::Dot([20.0, 22.2]),
        ],
        Family::Halves => vec![TileMark::Line(vec![
            on(top_back, bottom_back, 0.5),
            on(top_front, bottom_front, 0.5),
        ])],
        Family::Thirds => [0.38, 0.68]
            .map(|t| {
                TileMark::Line(vec![
                    on(top_back, top_front, t),
                    on(bottom_back, bottom_front, t),
                ])
            })
            .to_vec(),
        Family::Step => vec![TileMark::Line(vec![
            top_back,
            [4.0, 9.9],
            [4.0, 23.8],
            bottom_back,
        ])],
        _ => vec![TileMark::Dot([16.5, 19.0])],
    }
}
