//! Whole copies (`Kind::WholeCopy`, S95, V大's 整图复制): a composition divided, then copied whole as
//! a line drawing or in colour blocks (S96) over its division dimmed; its division read as a
//! division's (`divisions/`).

use crate::demos::demo::Demo;
use crate::kinds::divisions::demo_division;
use crate::kinds::{Described, Facts, Kind, Original, Turn, Work};
use crate::marks::mark::SheetMark;
use crate::packs::pack::{ItemRef, Tier};
use crate::sessions::drill::compositions;
use crate::sessions::family::Family;
use crate::sessions::plan::Exercise;

pub(crate) struct WholeCopies;

const FACTS: [Facts; 1] = [Facts::of(Family::WholeCopy, "whole-copy", Work::Divided).composed()];

impl Described for WholeCopies {
    fn code(&self) -> u8 {
        16
    }

    fn facts(&self) -> &'static [Facts] {
        &FACTS
    }

    fn variant(&self, kind: Kind) -> Option<u8> {
        match kind {
            Kind::WholeCopy(tier) => Some(tier.code()),
            _ => None,
        }
    }

    fn of_variant(&self, variant: u8) -> Option<Kind> {
        Tier::of_code(variant).map(Kind::WholeCopy)
    }

    fn givens(&self, _exercise: &Exercise, _original: &Original) -> Vec<SheetMark> {
        crate::marks::drawing::pictured()
    }

    fn original(&self, exercise: &Exercise) -> Original {
        exercise.picture()
    }

    fn how(&self, _family: Family) -> Option<Demo> {
        Some(demo_division::whole_copy())
    }

    /// A drill: one composition, the simple and medium tiers in turn.
    fn drill(&self, _family: Family, turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        compositions(1, turn, Kind::WholeCopy)
    }
}
