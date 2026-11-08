//! Value masses (`Kind::ValueMasses`, V大's 黑白灰, S96): a composition's picture painted as its few
//! big masses in three greys, read against the answer its pack holds (`marks.rs`), the answer drawn
//! in its greys for Compare to lay.

pub(crate) mod demo;
pub(crate) mod marks;
pub(crate) mod value_masses_read;

use crate::demos::demo::Demo;
use crate::kinds::{Described, Facts, Kind, Needs, Original, Tools, Turn, Work};
use crate::marks::asked::{Asked, Compare, Painted};
use crate::marks::mark::SheetMark;
use crate::marks::tile::{TileMark, closed, edge, square};
use crate::packs::pack::{ItemRef, Tier};
use crate::sessions::drill::compositions;
use crate::sessions::family::Family;
use crate::sessions::plan::Exercise;
use crate::sessions::sheet_picture::SheetPicture;

pub(crate) struct ValueMasses;

const FACTS: [Facts; 1] = [
    Facts::of(Family::ValueMasses, "value-masses", Work::Painted)
        .composed()
        .tools(Tools::Greys)
        .needs(Needs::Masses),
];
/// How many value masses a drill holds.
const DRILLED: usize = 3;
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

    fn givens(&self, _exercise: &Exercise, _original: &Original) -> Vec<SheetMark> {
        crate::marks::drawing::pictured()
    }

    fn original(&self, exercise: &Exercise) -> Original {
        exercise.picture()
    }

    fn how(&self, _family: Family) -> Option<Demo> {
        Some(demo::value_masses())
    }

    /// A drill: three compositions whose masses read, the simple and medium tiers in turn.
    fn drill(&self, _family: Family, turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        compositions(DRILLED, turn, Kind::ValueMasses)
    }

    fn compare(&self, asked: &Asked<'_>) -> Option<Compare> {
        marks::compare(asked)
    }

    /// The answer its picture holds in its greys, a pixel a cell (S96).
    fn painted(&self, _original: &Original, picture: Option<&SheetPicture>) -> Option<Painted> {
        let masses = picture?.masses.as_ref()?;
        Some((masses.answer.size, masses.answer.picture()))
    }

    /// A picture's edge, its ground under the ridge middle, a crown and a roof dark, a wall light.
    fn tile(&self, _family: Family) -> Vec<TileMark> {
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
