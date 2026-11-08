//! Points in space (`Kind::Space`, S90, V大's 点定位): a box's face and the five tricks, and their
//! check. Their How is the estimates' (`demo_points.rs`).

pub(crate) mod marks;
pub(crate) mod space;
pub(crate) mod space_check;

use crate::demos::demo::Demo;
use crate::kinds::estimates::demo_points;
use crate::kinds::{Described, Facts, Kind, Original, Turn, Work, place};
use crate::marks::compared::{Compared, DrawnCopy};
use crate::marks::mark::SheetMark;
use crate::packs::pack::ItemRef;
use crate::sessions::family::Family;
use crate::sessions::plan::Exercise;
use space::{SpaceTask, Trick};
use std::sync::Arc;

pub(crate) struct PointsInSpace;

const FACTS: [Facts; 5] = [
    Facts::of(Family::Centre, "centre", Work::Geometry).constructed(),
    Facts::of(Family::Halves, "halves", Work::Geometry).constructed(),
    Facts::of(Family::Thirds, "thirds", Work::Geometry).constructed(),
    Facts::of(Family::Step, "next-face", Work::Geometry).constructed(),
    Facts::of(Family::Mirror, "mirror", Work::Geometry).constructed(),
];
/// How many of a trick a drill holds.
const DRILLED: usize = 3;

impl Described for PointsInSpace {
    fn code(&self) -> u8 {
        12
    }

    fn facts(&self) -> &'static [Facts] {
        &FACTS
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
        demo_points::trick(family)
    }

    /// A drill: three of the one trick.
    fn drill(&self, family: Family, _turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        (Trick::ALL.into_iter())
            .filter(|trick| trick.family() == family)
            .flat_map(|trick| std::iter::repeat_n((Kind::Space(trick), None), DRILLED))
            .collect()
    }
}
