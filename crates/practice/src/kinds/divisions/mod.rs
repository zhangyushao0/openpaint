//! Divisions (`Kind::Division`, V大's 画面分割, S95): a composition's picture divided by its most
//! obvious lines in three levels; no answer: its lines read by their pens' levels, joined, counted
//! and measured (`division.rs`, `marks.rs`), the picture laid under them.
//!
//! A lesson holds one division (about 5 minutes): a simple or a medium composition in turn at the
//! first level, a medium one at the second (four to six masses), a hard one at the third (many
//! masses, close values; `course/picker.rs`).

pub(crate) mod demo;
pub(crate) mod division;
pub(crate) mod marks;

use std::sync::Arc;

use crate::course::picker::composition;
use crate::demos::demo::Demo;
use crate::kinds::{Described, Facts, Kind, Needs, Original, SkillLevel, ToolSet, Turn, held};
use crate::marks::asked::{Asked, Compare};
use crate::marks::mark::SheetMark;
use crate::marks::tile::{TileMark, edge};
use crate::packs::pack::{ItemRef, Tier};
use crate::sheets::exercise::Exercise;

pub(crate) struct Divisions;

const FACTS: [Facts; 1] =
    [Facts::of("division", ToolSet::DIVIDING, 300).needs(Needs::Compositions)];
/// How many divisions a lesson's place turns through at the first level (as unit 4's drills
/// held them before S122).
const TURNED: usize = 3;

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

    fn generate(&self, exercise: &Exercise) -> Arc<dyn Original> {
        held(Divided(exercise.item()))
    }

    fn how(&self, _kind: Kind, _cells: usize) -> Option<Demo> {
        Some(demo::division())
    }

    fn lesson(&self, level: SkillLevel, turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        composition((level, TURNED), turn, Kind::Division)
    }

    fn window(&self) -> usize {
        3
    }

    /// A picture's edge, a long line right through it, a bent one meeting its sides and a part's
    /// small frame standing on it.
    fn glyph(&self) -> Vec<TileMark> {
        vec![
            edge(),
            TileMark::Line(vec![[9.0, 18.0], [45.0, 14.5]]),
            TileMark::Line(vec![[9.0, 26.0], [27.0, 23.0], [45.0, 26.5]]),
            TileMark::Line(vec![[33.0, 24.4], [33.0, 17.0], [40.0, 17.0], [40.0, 25.3]]),
        ]
    }
}

/// A division's original: the composition's picture it divides.
#[derive(Debug)]
pub(crate) struct Divided(ItemRef);

impl Original for Divided {
    fn givens(&self) -> Vec<SheetMark> {
        crate::marks::drawing::pictured()
    }

    fn compare(&self, asked: &Asked<'_>) -> Option<Compare> {
        marks::compare(asked)
    }

    fn picture(&self) -> Option<&ItemRef> {
        Some(&self.0)
    }
}
