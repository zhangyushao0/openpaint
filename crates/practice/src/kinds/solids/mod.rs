//! Solids in boxes (`Kind::Solid`, S93): a cylinder, a cone or a sphere, its exact answer, its
//! check (an object's too: objects are boxes simplified, their answer and check these, `objects/`)
//! and its How.

pub(crate) mod demo_solids;
pub(crate) mod marks;
pub(crate) mod solid;
pub(crate) mod solid_answer;
pub(crate) mod solid_check;

use crate::demos::demo::Demo;
use crate::kinds::{Described, Facts, Kind, Original, Turn, Work, place};
use crate::marks::compared::{Compared, DrawnCopy};
use crate::marks::mark::SheetMark;
use crate::packs::pack::ItemRef;
use crate::sessions::family::Family;
use crate::sessions::plan::Exercise;
use solid::{Solid, SolidTask};
use std::sync::Arc;

pub(crate) struct Solids;

const FACTS: [Facts; 3] = [
    Facts::of(Family::Cylinder, "cylinder", Work::Geometry).constructed(),
    Facts::of(Family::Cone, "cone", Work::Geometry).constructed(),
    Facts::of(Family::Sphere, "sphere", Work::Geometry).constructed(),
];
/// How many of a solid a drill holds.
const DRILLED: usize = 3;

impl Described for Solids {
    fn code(&self) -> u8 {
        13
    }

    fn facts(&self) -> &'static [Facts] {
        &FACTS
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
        demo_solids::solid(family)
    }

    /// A drill: three of the one solid (its pose by its seed).
    fn drill(&self, family: Family, _turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        (Solid::ALL.into_iter())
            .filter(|solid| solid.family() == family)
            .flat_map(|solid| std::iter::repeat_n((Kind::Solid(solid), None), DRILLED))
            .collect()
    }
}
