//! Value masses (`Kind::ValueMasses`, V大's 黑白灰, S96): a composition's picture painted as its few
//! big masses in three greys, read against the answer its pack holds (`marks.rs`), the answer drawn
//! in its greys for Compare to lay.
//!
//! A lesson holds one picture's value masses (about 6 minutes): a simple or a medium composition
//! in turn at the first level, a medium one at the second, a hard one at the third, each among
//! those whose masses read (`course/picker.rs`).

pub(crate) mod demo;
pub(crate) mod marks;
pub(crate) mod value_masses_read;

use std::sync::Arc;

use crate::course::picker::composition;
use crate::demos::demo::Demo;
use crate::kinds::{Described, Facts, Kind, Needs, Original, SkillLevel, ToolSet, Turn, held};
use crate::marks::asked::{Asked, Compare, Painted};
use crate::marks::mark::SheetMark;
use crate::marks::tile::{TileMark, closed, edge, square};
use crate::packs::pack::{ItemRef, Tier};
use crate::sheets::exercise::Exercise;
use crate::sheets::sheet_picture::SheetPicture;

pub(crate) struct ValueMasses;

const FACTS: [Facts; 1] = [Facts::of("value-masses", ToolSet::GREYS, 360).needs(Needs::Masses)];
/// How many value masses a lesson's place turns through at the first level (as unit 4's drills
/// held them before S122).
const TURNED: usize = 3;
/// The tile's tints: the middle and the dark, shares of the tile's colour.
const MIDDLE: f32 = 0.3;
const DARK: f32 = 0.75;

impl Described for ValueMasses {
    fn code(&self) -> u8 {
        17
    }

    fn facts(&self) -> &'static [Facts] {
        &FACTS
    }

    fn variant(&self, kind: Kind) -> Option<u8> {
        match kind {
            Kind::ValueMasses(tier) => Some(tier.code()),
            _ => None,
        }
    }

    fn of_variant(&self, variant: u8) -> Option<Kind> {
        Tier::of_code(variant).map(Kind::ValueMasses)
    }

    fn generate(&self, exercise: &Exercise) -> Arc<dyn Original> {
        held(Valued(exercise.item()))
    }

    fn how(&self, _kind: Kind, _cells: usize) -> Option<Demo> {
        Some(demo::value_masses())
    }

    fn lesson(&self, level: SkillLevel, turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        composition((level, TURNED), turn, Kind::ValueMasses)
    }

    fn window(&self) -> usize {
        3
    }

    /// A picture's edge, its ground under the ridge middle, a crown and a roof dark, a wall light.
    fn glyph(&self) -> Vec<TileMark> {
        let ground = vec![
            [9.0, 19.0],
            [22.0, 12.5],
            [45.0, 18.0],
            [45.0, 32.0],
            [9.0, 32.0],
        ];
        let crown: Vec<[f32; 2]> = (0..8)
            .map(|k| {
                let turn = std::f32::consts::TAU * k as f32 / 8.0;
                [16.0 + 4.0 * turn.cos(), 20.0 + 4.6 * turn.sin()]
            })
            .collect();
        let roof = vec![[29.8, 21.0], [34.5, 16.5], [39.2, 21.0]];
        let wall = square([31.0, 21.0], [38.0, 26.5]);
        vec![
            edge(),
            TileMark::Tint(ground.clone(), MIDDLE),
            TileMark::Tint(crown.clone(), DARK),
            TileMark::Tint(roof.clone(), DARK),
            // The wall lies light over the ground: the paper, its outline drawn.
            TileMark::Paper(wall.to_vec()),
            TileMark::Line(vec![ground[0], ground[1], ground[2]]),
            TileMark::Line(closed(&crown)),
            TileMark::Line(closed(&roof)),
            TileMark::Line(closed(&wall)),
        ]
    }
}

/// Value masses' original: the composition's picture whose masses are painted.
#[derive(Debug)]
pub(crate) struct Valued(ItemRef);

impl Original for Valued {
    fn givens(&self) -> Vec<SheetMark> {
        crate::marks::drawing::pictured()
    }

    fn compare(&self, asked: &Asked<'_>) -> Option<Compare> {
        marks::compare(asked)
    }

    /// The answer its picture holds in its greys, a pixel a cell (S96).
    fn painted(&self, picture: Option<&SheetPicture>) -> Option<Painted> {
        let masses = picture?.masses.as_ref()?;
        Some((masses.answer.size, masses.answer.picture()))
    }

    fn picture(&self) -> Option<&ItemRef> {
        Some(&self.0)
    }
}
