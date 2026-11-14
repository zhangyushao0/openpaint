//! Whole copies (`Kind::WholeCopy`, V大's 整图复制, S95, S96): a composition's picture divided, then
//! copied whole as a line drawing or in colour blocks over its division dimmed; compared by its
//! picture laid over the copy as strongly as the user sets, its division hidden, its division's
//! counts kept (`divisions/`).
//!
//! A lesson holds one whole copy (about 8 minutes): a simple or a medium composition in turn at
//! the first level, a medium one at the second, a hard one at the third (`course/picker.rs`).

pub(crate) mod demo;

use std::sync::Arc;

use crate::course::picker::composition;
use crate::demos::demo::Demo;
use crate::kinds::divisions::marks::division;
use crate::kinds::{
    Described, Facts, Kind, Needs, Original, Picking, SkillLevel, ToolSet, Turn, held,
};
use crate::marks::asked::{Asked, Compare};
use crate::marks::drawing::picture_laid;
use crate::marks::mark::SheetMark;
use crate::marks::tile::{TileMark, closed, curve, edge, square};
use crate::packs::pack::{ItemRef, Tier};
use crate::sheets::exercise::Exercise;

pub(crate) struct WholeCopies;

const FACTS: [Facts; 1] = [Facts::of("whole-copy", ToolSet::COPYING, 480)
    .picks(Picking::BLOCKS)
    .needs(Needs::Compositions)];

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

    fn generate(&self, exercise: &Exercise) -> Arc<dyn Original> {
        held(Whole(exercise.item()))
    }

    fn how(&self, _kind: Kind, _cells: usize) -> Option<Demo> {
        Some(demo::whole_copy())
    }

    fn lesson(&self, level: SkillLevel, turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        composition((level, 1), turn, Kind::WholeCopy)
    }

    fn window(&self) -> usize {
        3
    }

    /// A picture's edge, a ridge, a field's edge, a tree and a house.
    fn glyph(&self) -> Vec<TileMark> {
        vec![
            edge(),
            TileMark::Line(vec![[9.0, 19.0], [22.0, 12.5], [45.0, 18.0]]),
            TileMark::Line(vec![[9.0, 26.5], [45.0, 25.5]]),
            TileMark::Line(closed(&square([31.0, 20.5], [38.0, 25.8]))),
            TileMark::Line(vec![[29.8, 20.5], [34.5, 16.5], [39.2, 20.5]]),
            TileMark::Line(curve(|t| {
                let turn = std::f32::consts::TAU * t;
                [16.0 + 3.6 * turn.cos(), 19.5 + 4.2 * turn.sin()]
            })),
            TileMark::Line(vec![[16.0, 23.7], [16.0, 26.3]]),
        ]
    }
}

/// A whole copy's original: the composition's picture it copies.
#[derive(Debug)]
pub(crate) struct Whole(ItemRef);

impl Original for Whole {
    fn givens(&self) -> Vec<SheetMark> {
        crate::marks::drawing::pictured()
    }

    /// Once anything lies in its frame: its picture laid over it, keeping its division's numbers.
    fn compare(&self, asked: &Asked<'_>) -> Option<Compare> {
        if !asked.held() {
            return None;
        }
        let metrics = division(asked).map(|check| check.metrics);
        picture_laid(asked, metrics.unwrap_or_default())
    }

    fn picture(&self) -> Option<&ItemRef> {
        Some(&self.0)
    }
}
