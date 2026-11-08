//! Free boxes (`Kind::FreeBoxes`, S84): boxes drawn freely on a horizon, checked by the rules
//! alone. Their page is `geometry/perspective.rs`; its How `demo.rs`.

pub(crate) mod demo;
pub(crate) mod marks;
pub(crate) mod perspective_free;

use crate::demos::demo::Demo;
use crate::geometry::perspective::{Horizon, PerspectiveBox};
use crate::kinds::{Described, Facts, Kind, Original, Turn, Work, place};
use crate::marks::compared::{Compared, DrawnCopy};
use crate::marks::mark::SheetMark;
use crate::marks::tile::{TileMark, boxed};
use crate::packs::pack::ItemRef;
use crate::sessions::drill::turned;
use crate::sessions::family::Family;
use crate::sessions::plan::Exercise;
use crate::sessions::session::HORIZONS;
use std::sync::Arc;

pub(crate) struct FreeBoxes;

const FACTS: [Facts; 1] =
    [Facts::of(Family::FreeBoxes, "free-boxes", Work::Geometry).constructed()];

impl Described for FreeBoxes {
    fn code(&self) -> u8 {
        9
    }

    fn facts(&self) -> &'static [Facts] {
        &FACTS
    }

    fn variant(&self, kind: Kind) -> Option<u8> {
        match kind {
            Kind::FreeBoxes(horizon) => Some(place(&Horizon::ALL, &horizon)),
            _ => None,
        }
    }

    fn of_variant(&self, variant: u8) -> Option<Kind> {
        (Horizon::ALL.get(usize::from(variant))).map(|h| Kind::FreeBoxes(*h))
    }

    fn original(&self, exercise: &Exercise) -> Original {
        let Kind::FreeBoxes(horizon) = exercise.kind else {
            return exercise.picture();
        };
        Original::FreeBoxes(PerspectiveBox::on_horizon(horizon, exercise.seed))
    }

    fn givens(&self, _exercise: &Exercise, original: &Original) -> Vec<SheetMark> {
        match original {
            Original::FreeBoxes(page) => marks::givens(page),
            _ => Vec::new(),
        }
    }

    fn check(&self, original: &Original, copy: &DrawnCopy) -> Option<Arc<dyn Compared>> {
        match original {
            Original::FreeBoxes(page) => Some(marks::FreeCompared::of(page, copy)),
            _ => None,
        }
    }

    fn how(&self, _family: Family) -> Option<Demo> {
        Some(demo::free_boxes())
    }

    /// A drill: one page, its horizon in turn.
    fn drill(&self, _family: Family, turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        turned((&HORIZONS, 1), turn, Kind::FreeBoxes)
    }

    /// Two small boxes on a horizon.
    fn tile(&self, _family: Family) -> Vec<TileMark> {
        let mut marks = vec![TileMark::Line(vec![[4.0, 12.0], [50.0, 12.0]])];
        marks.extend(boxed([-12.0, 6.0], 0.45));
        marks.extend(boxed([12.0, 5.0], 0.4));
        marks
    }
}
