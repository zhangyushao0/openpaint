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
use crate::kinds::{Described, Facts, Kind, Original, SkillLevel, Tools, Turn, place};
use crate::marks::compared::{Compared, DrawnCopy};
use crate::marks::mark::SheetMark;
use crate::marks::tile::{TileMark, arc};
use crate::maths::path::Point;
use crate::packs::pack::ItemRef;
use crate::sheets::exercise::Exercise;
use crate::sheets::family::Family;
use solid::{Solid, SolidTask};
use std::sync::Arc;

pub(crate) struct Solids;

const FACTS: [Facts; 3] = [
    Facts::of(Family::Cylinder, "cylinder", Tools::Drawing, 150).constructed(),
    Facts::of(Family::Cone, "cone", Tools::Drawing, 150).constructed(),
    Facts::of(Family::Sphere, "sphere", Tools::Drawing, 150).constructed(),
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

    fn family(&self, kind: Kind) -> Family {
        match kind {
            Kind::Solid(solid) => solid.family(),
            _ => Family::Cylinder,
        }
    }

    fn original(&self, exercise: &Exercise) -> Original {
        let Kind::Solid(solid) = exercise.kind else {
            return exercise.picture();
        };
        Original::Solid(Box::new(SolidTask::generate(solid, exercise.seed)))
    }

    fn givens(&self, _exercise: &Exercise, original: &Original) -> Vec<SheetMark> {
        match original {
            Original::Solid(task) => marks::givens(task),
            _ => Vec::new(),
        }
    }

    fn check(&self, original: &Original, copy: &DrawnCopy) -> Option<Arc<dyn Compared>> {
        match original {
            Original::Solid(task) => Some(marks::SolidCompared::of(task, copy)),
            _ => None,
        }
    }

    fn how(&self, family: Family) -> Option<Demo> {
        demo::solid(family)
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

    /// Once round each ellipse, a stroke along each side.
    fn answer(&self, original: &Original) -> Vec<Vec<Point>> {
        let Original::Solid(task) = original else {
            return Vec::new();
        };
        (task.answer.ellipses.iter())
            .map(|ellipse| ellipse.outline(ROUND))
            .chain(task.answer.sides.iter().map(|side| side.to_vec()))
            .collect()
    }

    /// An upright cylinder (its top whole, its bottom's front, two sides), an upright cone (its
    /// base's front, two sides from its point), a sphere with its middle section's front.
    fn tile(&self, family: Family) -> Vec<TileMark> {
        match family {
            Family::Cylinder => vec![
                arc([27.0, 11.0], [11.0, 4.0], (0.0, 360.0)),
                arc([27.0, 30.0], [11.0, 4.0], (0.0, 180.0)),
                TileMark::Line(vec![[16.0, 11.0], [16.0, 30.0]]),
                TileMark::Line(vec![[38.0, 11.0], [38.0, 30.0]]),
            ],
            // The sides touch the base where lines from the point do: 10.4 degrees above its middle.
            Family::Cone => vec![
                arc([27.0, 30.0], [12.0, 4.5], (-10.4, 190.4)),
                TileMark::Line(vec![[27.0, 5.0], [38.8, 29.2]]),
                TileMark::Line(vec![[27.0, 5.0], [15.2, 29.2]]),
            ],
            _ => vec![
                arc([27.0, 20.0], [13.0, 13.0], (0.0, 360.0)),
                arc([27.0, 21.5], [13.0, 4.5], (0.0, 180.0)),
            ],
        }
    }
}
