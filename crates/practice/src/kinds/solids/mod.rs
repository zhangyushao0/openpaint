//! Solids in boxes (`Kind::Solid`, S93): a cylinder, a cone or a sphere, its exact answer, its
//! check (an object's too: objects are boxes simplified, their answer and check these, `objects/`)
//! and its How, and its lessons.
//!
//! A lesson (a solid about 2.5 minutes) holds two, in V大's order: the first level cylinders, the
//! second cones, the third a sphere with a cylinder or a cone in turn.

pub(crate) mod demo;
pub(crate) mod marks;
pub(crate) mod solid;
pub(crate) mod solid_answer;
pub(crate) mod solid_check;

use crate::course::picker::laid;
use crate::demos::demo::Demo;
use crate::kinds::{
    Blank, Described, Facts, Kind, Original, SkillLevel, ToolSet, Turn, held, place,
};
use crate::marks::compared::{Compared, DrawnCopy};
use crate::marks::mark::SheetMark;
use crate::marks::tile::{TileMark, arc};
use crate::maths::path::Point;
use crate::packs::pack::ItemRef;
use crate::sheets::exercise::Exercise;
use solid::{Solid, SolidTask};
use std::sync::Arc;

pub(crate) struct Solids;

/// Its topics, in `Solid::ALL`'s order: each has its own instructions.
const FACTS: [Facts; 3] = [
    Facts::of("cylinder", ToolSet::DRAWING, 150).constructed(),
    Facts::of("cone", ToolSet::DRAWING, 150).constructed(),
    Facts::of("sphere", ToolSet::DRAWING, 150).constructed(),
];
/// The solids a sphere takes in turn at the third level.
const BESIDE: [Solid; 2] = [Solid::Cylinder, Solid::Cone];
/// An ellipse of an answer as a hand draws it once round, in pieces.
const ROUND: usize = 48;

impl Described for Solids {
    fn code(&self) -> u8 {
        13
    }

    fn facts(&self) -> &'static [Facts] {
        &FACTS
    }

    fn key(&self) -> &'static str {
        "solids"
    }

    fn variant(&self, kind: Kind) -> Option<u8> {
        match kind {
            Kind::Solid(solid) => Some(place(&Solid::ALL, &solid)),
            _ => None,
        }
    }

    fn of_variant(&self, variant: u8) -> Option<Kind> {
        Solid::ALL
            .get(usize::from(variant))
            .map(|s| Kind::Solid(*s))
    }

    fn topic(&self, kind: Kind) -> &'static Facts {
        let Kind::Solid(solid) = kind else {
            return &FACTS[0];
        };
        &FACTS[usize::from(place(&Solid::ALL, &solid))]
    }

    fn generate(&self, exercise: &Exercise) -> Arc<dyn Original> {
        let Kind::Solid(solid) = exercise.kind else {
            return held(Blank);
        };
        held(SolidTask::generate(solid, exercise.seed))
    }

    fn how(&self, kind: Kind, _cells: usize) -> Option<Demo> {
        let Kind::Solid(solid) = kind else {
            return None;
        };
        demo::solid(solid)
    }

    /// Each solid's pose by its seed.
    fn lesson(&self, level: SkillLevel, turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        let solids = match level {
            SkillLevel::First => [Solid::Cylinder; 2],
            SkillLevel::Second => [Solid::Cone; 2],
            SkillLevel::Third => [Solid::Sphere, BESIDE[turn.turn % BESIDE.len()]],
        };
        laid(solids.map(Kind::Solid))
    }

    /// An upright cylinder: its top whole, its bottom's front, two sides.
    fn glyph(&self) -> Vec<TileMark> {
        vec![
            arc([27.0, 11.0], [11.0, 4.0], (0.0, 360.0)),
            arc([27.0, 30.0], [11.0, 4.0], (0.0, 180.0)),
            TileMark::Line(vec![[16.0, 11.0], [16.0, 30.0]]),
            TileMark::Line(vec![[38.0, 11.0], [38.0, 30.0]]),
        ]
    }
}

/// A box with a solid to find in it.
impl Original for SolidTask {
    fn givens(&self) -> Vec<SheetMark> {
        marks::givens(self)
    }

    fn check(&self, copy: &DrawnCopy) -> Option<Arc<dyn Compared>> {
        Some(marks::SolidCompared::of(self, copy))
    }

    /// Once round each ellipse, a stroke along each side.
    fn answer(&self) -> Vec<Vec<Point>> {
        (self.answer.ellipses.iter())
            .map(|ellipse| ellipse.outline(ROUND))
            .chain(self.answer.sides.iter().map(|side| side.to_vec()))
            .collect()
    }
}
