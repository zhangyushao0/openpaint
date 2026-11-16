//! Value masses (`Kind::ValueMasses`, V大's 黑白灰, S96): a composition's picture painted as its few
//! big masses in three greys, read against the answer its pack holds (`marks.rs`), the answer drawn
//! in its greys for Compare to lay; or in two (Notan, S156, `notan.rs`), each mass decided light or
//! dark, read against the answer's two merges. What varies an exercise is `massing.rs`.
//!
//! A lesson holds one picture's value masses (about 6 minutes): a simple or a medium composition
//! in turn at the first level, a medium one at the second, a hard one at the third, each among
//! those whose masses read (`course/picker.rs`); at the second and the third every second lesson
//! (the even turns) paints it in two values, its picture picked as the three values' would be.

pub(crate) mod demo;
pub(crate) mod marks;
pub(crate) mod massing;
pub(crate) mod notan;
pub(crate) mod value_masses_read;

use std::sync::Arc;

use crate::course::picker::composition;
use crate::demos::demo::Demo;
use crate::kinds::{Described, Facts, Kind, Needs, Original, SkillLevel, ToolSet, Turn, held};
use crate::marks::asked::{Asked, Compare, Painted};
use crate::marks::mark::SheetMark;
use crate::marks::tile::{TileMark, closed, edge, square};
use crate::packs::pack::ItemRef;
use crate::sheets::exercise::Exercise;
use crate::sheets::sheet_picture::SheetPicture;
use massing::Massing;

pub(crate) struct ValueMasses;

/// Its topics: three values, and two (Notan, S156).
const FACTS: [Facts; 2] = [
    Facts::of("value-masses", ToolSet::GREYS, 360).needs(Needs::Masses),
    Facts::of("notan", ToolSet::TWO_GREYS, 300).needs(Needs::Masses),
];
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
            Kind::ValueMasses(massing) => Some(massing.code()),
            _ => None,
        }
    }

    fn of_variant(&self, variant: u8) -> Option<Kind> {
        Massing::of_code(variant).map(Kind::ValueMasses)
    }

    fn topic(&self, kind: Kind) -> &'static Facts {
        match kind {
            Kind::ValueMasses(Massing::Two(_)) => &FACTS[1],
            _ => &FACTS[0],
        }
    }

    fn generate(&self, exercise: &Exercise) -> Arc<dyn Original> {
        let two = matches!(exercise.kind, Kind::ValueMasses(Massing::Two(_)));
        held(Valued {
            item: exercise.item(),
            two,
        })
    }

    fn how(&self, kind: Kind, _cells: usize) -> Option<Demo> {
        Some(match kind {
            Kind::ValueMasses(Massing::Two(_)) => demo::notan(),
            _ => demo::value_masses(),
        })
    }

    /// The first level as before S156; the second and the third in two values at even turns.
    fn lesson(&self, level: SkillLevel, turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        let two = level != SkillLevel::First && turn.turn.is_multiple_of(2);
        if two {
            composition((level, TURNED), turn, |tier| {
                Kind::ValueMasses(Massing::Two(tier))
            })
        } else {
            composition((level, TURNED), turn, |tier| {
                Kind::ValueMasses(Massing::Three(tier))
            })
        }
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

/// Value masses' original: the composition's picture whose masses are painted, in two values or
/// three.
#[derive(Debug)]
pub(crate) struct Valued {
    item: ItemRef,
    two: bool,
}

impl Original for Valued {
    fn givens(&self) -> Vec<SheetMark> {
        crate::marks::drawing::pictured()
    }

    fn compare(&self, asked: &Asked<'_>) -> Option<Compare> {
        if self.two {
            notan::compare(asked)
        } else {
            marks::compare(asked)
        }
    }

    /// The answer its picture holds in its greys, a pixel a cell (S96); none in two values, whose
    /// answer is the merge the copy matches better, which its reading brings.
    fn painted(&self, picture: Option<&SheetPicture>) -> Option<Painted> {
        let masses = picture?.masses.as_ref().filter(|_| !self.two)?;
        Some((masses.answer.size, masses.answer.picture()))
    }

    fn picture(&self) -> Option<&ItemRef> {
        Some(&self.item)
    }
}
