//! Divisions (`Kind::Division`, V大's 画面分割, S95): a composition's picture divided by its most
//! obvious lines in three levels; no answer: its lines read by their pens' levels, joined, counted
//! and measured (`division.rs`, `marks.rs`), the picture laid under them.

pub(crate) mod demo;
pub(crate) mod division;
pub(crate) mod marks;

use crate::demos::demo::Demo;
use crate::kinds::{Described, Facts, Kind, Needs, Original, Turn, Work};
use crate::marks::asked::{Asked, Compare};
use crate::marks::mark::SheetMark;
use crate::marks::tile::{TileMark, edge};
use crate::packs::pack::{ItemRef, Tier};
use crate::sessions::drill::compositions;
use crate::sessions::family::Family;
use crate::sessions::plan::Exercise;

pub(crate) struct Divisions;

const FACTS: [Facts; 1] = [Facts::of(Family::Division, "division", Work::Divided)
    .composed()
    .needs(Needs::Compositions)];
/// How many divisions a drill holds.
const DRILLED: usize = 3;

impl Described for Divisions {
    fn code(&self) -> u8 {
        15
    }

    fn facts(&self) -> &'static [Facts] {
        &FACTS
    }

    fn variant(&self, kind: Kind) -> Option<u8> {
        match kind {
            Kind::Division(tier) => Some(tier.code()),
            _ => None,
        }
    }

    fn of_variant(&self, variant: u8) -> Option<Kind> {
        Tier::of_code(variant).map(Kind::Division)
    }

    fn givens(&self, _exercise: &Exercise, _original: &Original) -> Vec<SheetMark> {
        crate::marks::drawing::pictured()
    }

    fn original(&self, exercise: &Exercise) -> Original {
        exercise.picture()
    }

    fn how(&self, _family: Family) -> Option<Demo> {
        Some(demo::division())
    }

    /// A drill: three compositions, the simple and medium tiers in turn.
    fn drill(&self, _family: Family, turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        compositions(DRILLED, turn, Kind::Division)
    }

    fn compare(&self, asked: &Asked<'_>) -> Option<Compare> {
        marks::compare(asked)
    }

    /// A picture's edge, a long line right through it, a bent one meeting its sides and a part's
    /// small frame standing on it.
    fn tile(&self, _family: Family) -> Vec<TileMark> {
        vec![
            edge(),
            TileMark::Line(vec![[9.0, 18.0], [45.0, 14.5]]),
            TileMark::Line(vec![[9.0, 26.0], [27.0, 23.0], [45.0, 26.5]]),
            TileMark::Line(vec![[33.0, 24.4], [33.0, 17.0], [40.0, 17.0], [40.0, 25.3]]),
        ]
    }
}
