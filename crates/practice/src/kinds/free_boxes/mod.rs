//! Free boxes (`Kind::FreeBoxes`, S84): boxes drawn freely on a horizon, checked by the rules
//! alone and judged by eye (S114). Their page is `geometry/perspective.rs`; its How `demo.rs`.
//!
//! A lesson (a page of three or four boxes, about 3 minutes): the first level one page, its
//! horizon in the middle, high, then low by the lesson (stage 2's drills), the second one page
//! high or low in turn, the boxes all seen from above or below, the third two pages, one of
//! each. A page has no harder variant than its horizon.

pub(crate) mod demo;
pub(crate) mod marks;
pub(crate) mod perspective_free;

use crate::course::picker::{laid, rotated, turned};
use crate::demos::demo::Demo;
use crate::geometry::perspective::{Horizon, PerspectiveBox};
use crate::kinds::{
    Blank, Described, Facts, Kind, Original, SkillLevel, ToolSet, Turn, held, place,
};
use crate::marks::compared::{Compared, DrawnCopy};
use crate::marks::mark::SheetMark;
use crate::marks::tile::{TileMark, boxed};
use crate::packs::pack::ItemRef;
use crate::sheets::exercise::Exercise;
use std::sync::Arc;

pub(crate) struct FreeBoxes;

const FACTS: [Facts; 1] = [Facts::of("free-boxes", ToolSet::DRAWING, 180).constructed()];
/// The first level's horizons in turn: in the middle first; the others' far from it.
const HORIZONS: [Horizon; 3] = [Horizon::Middle, Horizon::High, Horizon::Low];
const FAR: [Horizon; 2] = [Horizon::High, Horizon::Low];

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

    fn generate(&self, exercise: &Exercise) -> Arc<dyn Original> {
        let Kind::FreeBoxes(horizon) = exercise.kind else {
            return held(Blank);
        };
        held(FreePage(PerspectiveBox::on_horizon(horizon, exercise.seed)))
    }

    fn how(&self, _kind: Kind, _cells: usize) -> Option<Demo> {
        Some(demo::free_boxes())
    }

    fn lesson(&self, level: SkillLevel, turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        match level {
            SkillLevel::First => turned((&HORIZONS, 1), turn, Kind::FreeBoxes),
            SkillLevel::Second => turned((&FAR, 1), turn, Kind::FreeBoxes),
            SkillLevel::Third => laid(
                rotated(&FAR, turn.turn, FAR.len())
                    .into_iter()
                    .map(Kind::FreeBoxes),
            ),
        }
    }

    fn window(&self) -> usize {
        3
    }

    /// Two small boxes on a horizon.
    fn glyph(&self) -> Vec<TileMark> {
        let mut marks = vec![TileMark::Line(vec![[4.0, 12.0], [50.0, 12.0]])];
        marks.extend(boxed([-12.0, 6.0], 0.45));
        marks.extend(boxed([12.0, 5.0], 0.4));
        marks
    }
}

/// A free-boxes page: its horizon is the example box's.
#[derive(Debug)]
pub struct FreePage(pub PerspectiveBox);

impl Original for FreePage {
    fn givens(&self) -> Vec<SheetMark> {
        marks::givens(&self.0)
    }

    fn check(&self, copy: &DrawnCopy) -> Option<Arc<dyn Compared>> {
        Some(marks::FreeCompared::of(&self.0, copy))
    }
}
