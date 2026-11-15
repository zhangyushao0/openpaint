//! Abstract analysis (`Kind::Analysis`, S112, V大's W5-6 抽象对比分析: the content forgotten for a
//! while, only the shapes, the colours and how they stand against each other looked at): a
//! composition's picture in one frame of its shape, lying on a locked layer of the sheet under the
//! marks (`Facts::under`); its four steps on the card, as V大 asks them: the place that draws the
//! eye most ringed with 强 (the levels' red pen), by what contrast it draws it (six chips, several
//! on), the second places ringed with 次 and the weak hatched with 弱, and how strength is spread
//! (four pictures, one on). Compare scores nothing: its views veil the picture where a contrast is
//! weak (`contrast.rs`, from its pixels on the compare worker), the marks over the veil
//! (`marks.rs`); the eye judges once a 强 mark lies in the frame.
//!
//! A lesson holds one picture (about 5 minutes), the composition's tier its level: simple,
//! medium, hard (the nearest tier held where none).

pub(crate) mod contrast;
pub(crate) mod demo;
pub(crate) mod marks;

use std::sync::Arc;

use crate::course::picker::Picker;
use crate::demos::demo::Demo;
use crate::kinds::{
    Described, Facts, Kind, Needs, Original, Picking, SkillLevel, ToolSet, Turn, held,
};
use crate::marks::asked::{Asked, Compare};
use crate::marks::card::CardItem;
use crate::marks::mark::{Figure, In, SheetMark};
use crate::marks::tile::{TileMark, curve, edge};
use crate::packs::pack::{ItemRef, Tier};
use crate::sheets::exercise::Exercise;

pub(crate) struct Analyses;

const FACTS: [Facts; 1] = [Facts::of("abstract-analysis", ToolSet::MARKING, 300)
    .constructed()
    .under()
    .picks(Picking::LOCKED)
    .needs(Needs::Compositions)];

impl Described for Analyses {
    fn code(&self) -> u8 {
        21
    }

    fn facts(&self) -> &'static [Facts] {
        &FACTS
    }

    fn variant(&self, kind: Kind) -> Option<u8> {
        match kind {
            Kind::Analysis(tier) => Some(tier.code()),
            _ => None,
        }
    }

    fn of_variant(&self, variant: u8) -> Option<Kind> {
        Tier::of_code(variant).map(Kind::Analysis)
    }

    fn generate(&self, exercise: &Exercise) -> Arc<dyn Original> {
        held(Analysed(exercise.item()))
    }

    fn how(&self, _kind: Kind, _cells: usize) -> Option<Demo> {
        Some(demo::analysis())
    }

    /// One composition of the level's tier: simple, medium, hard.
    fn lesson(&self, level: SkillLevel, turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        let tier = match level {
            SkillLevel::First => Tier::Simple,
            SkillLevel::Second => Tier::Medium,
            SkillLevel::Third => Tier::Hard,
        };
        let picked = Picker::new(turn, Tier::Medium).composition(tier, Kind::Analysis);
        picked.into_iter().collect()
    }

    fn window(&self) -> usize {
        3
    }

    /// A picture's edge, a ring round its strongest place and a few strokes of hatching.
    fn glyph(&self) -> Vec<TileMark> {
        let ring = curve(|t| {
            let turn = std::f32::consts::TAU * t;
            [33.0 + 7.0 * turn.cos(), 17.0 + 6.0 * turn.sin()]
        });
        vec![
            edge(),
            TileMark::Line(ring),
            TileMark::Line(vec![[11.0, 30.0], [17.0, 24.0]]),
            TileMark::Line(vec![[14.0, 31.0], [21.0, 24.0]]),
            TileMark::Line(vec![[18.0, 31.0], [24.0, 25.0]]),
        ]
    }
}

/// An abstract analysis's original: the composition's picture it analyses.
#[derive(Debug)]
pub(crate) struct Analysed(ItemRef);

impl Original for Analysed {
    /// The picture's credit under its frame; the picture itself lies on the sheet's layer.
    fn givens(&self) -> Vec<SheetMark> {
        vec![SheetMark::free(In::Copy, Figure::Pictured)]
    }

    fn task(&self) -> Vec<CardItem> {
        marks::task()
    }

    fn compare(&self, asked: &Asked<'_>) -> Option<Compare> {
        marks::compare(asked)
    }

    fn picture(&self) -> Option<&ItemRef> {
        Some(&self.0)
    }
}
