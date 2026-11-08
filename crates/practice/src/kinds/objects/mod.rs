//! Objects simplified into boxes (`Kind::Object`, S94): an object's design, body, exact answer,
//! construction, rules and picture, and its How. An object is V大's box simplified, so it builds
//! on the solids (`solids/`): its answer is a solid's kind (`SolidAnswer`) and its check theirs
//! (`SolidCheck::of_object`).

pub(crate) mod demo_objects;
pub(crate) mod marks;
pub(crate) mod object;
pub(crate) mod object_answer;
pub(crate) mod object_body;
pub(crate) mod object_changes;
pub(crate) mod object_construction;
pub(crate) mod object_design;
pub(crate) mod object_hull;
pub(crate) mod object_picture;
pub(crate) mod object_rays;
pub(crate) mod object_rules;

use crate::demos::demo::Demo;
use crate::kinds::{Described, Facts, Kind, Original, Turn, Work, place};
use crate::marks::compared::{Compared, DrawnCopy};
use crate::marks::mark::SheetMark;
use crate::packs::pack::ItemRef;
use crate::sessions::drill::turned;
use crate::sessions::family::Family;
use crate::sessions::plan::Exercise;
use object::ObjectTask;
use object_design::Carve;
use std::sync::Arc;

pub(crate) struct Objects;

const FACTS: [Facts; 1] = [Facts::of(Family::Object, "objects", Work::Geometry)];
/// How many objects a drill holds.
const DRILLED: usize = 3;

impl Described for Objects {
    fn code(&self) -> u8 {
        14
    }

    fn facts(&self) -> &'static [Facts] {
        &FACTS
    }

    fn variant(&self, kind: Kind) -> Option<u8> {
        match kind {
            Kind::Object(carve) => Some(place(&Carve::ALL, &carve)),
            _ => None,
        }
    }

    fn of_variant(&self, variant: u8) -> Option<Kind> {
        Carve::ALL
            .get(usize::from(variant))
            .map(|c| Kind::Object(*c))
    }

    fn original(&self, exercise: &Exercise) -> Original {
        let Kind::Object(carve) = exercise.kind else {
            return exercise.picture();
        };
        Original::Object(Box::new(ObjectTask::generate(carve, exercise.seed)))
    }

    fn givens(&self, _exercise: &Exercise, original: &Original) -> Vec<SheetMark> {
        match original {
            Original::Object(task) => marks::givens(task),
            _ => Vec::new(),
        }
    }

    fn check(&self, original: &Original, copy: &DrawnCopy) -> Option<Arc<dyn Compared>> {
        match original {
            Original::Object(task) => Some(marks::ObjectCompared::of(task, copy)),
            _ => None,
        }
    }

    fn how(&self, _family: Family) -> Option<Demo> {
        demo_objects::object()
    }

    /// A drill: three objects, their carves in turn.
    fn drill(&self, _family: Family, turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        turned((&Carve::ALL, DRILLED), turn, Kind::Object)
    }
}
