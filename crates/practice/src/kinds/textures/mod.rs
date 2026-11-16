//! Texture (`Kind::Texture`, S154j, 纹理, unit 7): telling a surface's texture by the shadows its
//! small forms throw (implicit), not by outlining every form (explicit); at 稳 letting the side
//! toward the light go sparse (Drawabox's texture analysis and its density gradient). A strip of
//! ground covered with small forms (`field.rs`), seen and lit exactly (`rays.rs`), its picture and
//! the answer's shadows (`shadows.rs`); the copy is painted with the darkest pen alone and read
//! against them (`read.rs`), the marks help and the eye judges (D123, `marks.rs`).
//!
//! A lesson holds one strip (about 5 minutes), the sun from the left and the right in turn: at 入门
//! round pebbles of one size; at 熟练 plates, ridges and grooves; at 稳 a dense mix, the half toward
//! the light kept to its biggest shadows.

pub(crate) mod demo;
pub(crate) mod field;
pub(crate) mod marks;
pub(crate) mod rays;
pub(crate) mod read;
pub(crate) mod shadows;

#[cfg(test)]
mod tests;

use std::sync::Arc;

use crate::course::picker::laid;
use crate::demos::demo::Demo;
use crate::kinds::objects::OBJECT_SIDE;
use crate::kinds::{
    Blank, Described, Facts, Kind, Original, SkillLevel, ToolSet, Turn, held, place,
};
use crate::marks::asked::{Asked, Compare, Painted, Reading};
use crate::marks::mark::SheetMark;
use crate::marks::tile::{TileMark, arc, curve};
use crate::packs::pack::ItemRef;
use crate::sheets::exercise::Exercise;
use crate::sheets::sheet_picture::SheetPicture;
use field::{Bumps, Texture};
use marks::TextureReading;
use shadows::TextureTask;

pub(crate) struct Textures;

/// The strip's shape, its frames': twice as wide as high.
const SHAPE: [u16; 2] = [2, 1];
/// Its topics: every shadow told (入门, 熟练), and the side toward the light sparse (稳).
const FACTS: [Facts; 2] = [
    Facts::of("texture", ToolSet::SHADOWS, 300).shaped(SHAPE),
    Facts::of("texture-sparse", ToolSet::SHADOWS, 300).shaped(SHAPE),
];

impl Described for Textures {
    fn code(&self) -> u8 {
        30
    }

    fn facts(&self) -> &'static [Facts] {
        &FACTS
    }

    fn key(&self) -> &'static str {
        "texture"
    }

    fn variant(&self, kind: Kind) -> Option<u8> {
        match kind {
            Kind::Texture(texture) => Some(place(&Texture::ALL, &texture)),
            _ => None,
        }
    }

    fn of_variant(&self, variant: u8) -> Option<Kind> {
        (Texture::ALL.get(usize::from(variant))).map(|texture| Kind::Texture(*texture))
    }

    fn topic(&self, kind: Kind) -> &'static Facts {
        match kind {
            Kind::Texture(Texture {
                bumps: Bumps::Dense,
                ..
            }) => &FACTS[1],
            _ => &FACTS[0],
        }
    }

    fn generate(&self, exercise: &Exercise) -> Arc<dyn Original> {
        let Kind::Texture(texture) = exercise.kind else {
            return held(Blank);
        };
        held(TextureTask::generate(texture, exercise.seed))
    }

    fn how(&self, kind: Kind, _cells: usize) -> Option<Demo> {
        let Kind::Texture(texture) = kind else {
            return None;
        };
        Some(demo::texture(texture))
    }

    fn lesson(&self, level: SkillLevel, turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        let bumps = match level {
            SkillLevel::First => Bumps::Pebbles,
            SkillLevel::Second => Bumps::Mixed,
            SkillLevel::Third => Bumps::Dense,
        };
        let from_right = turn.turn % 2 == 1;
        laid([Kind::Texture(Texture { bumps, from_right })])
    }

    fn window(&self) -> usize {
        3
    }

    /// A strip of pebbles, each its shadow beside it, no outline round the lit side.
    fn glyph(&self) -> Vec<TileMark> {
        let mut marks = Vec::new();
        for (centre, radius) in [
            ([12.0, 14.0], 4.5),
            ([28.0, 24.0], 5.5),
            ([42.0, 13.0], 4.0),
        ] {
            let [x, y]: [f32; 2] = centre;
            let shadow: Vec<[f32; 2]> = curve(|t| {
                let turn = std::f32::consts::TAU * t;
                [
                    x + radius * 0.9 + radius * turn.cos(),
                    y + radius * 0.35 * turn.sin(),
                ]
            });
            let turned: Vec<[f32; 2]> = curve(|t| {
                let turn = (-80.0 + 160.0 * t).to_radians();
                [x + radius * turn.cos(), y + radius * turn.sin()]
            });
            marks.push(TileMark::Tint(shadow, 0.6));
            marks.push(TileMark::Tint(turned, 0.6));
            marks.push(arc(centre, [radius; 2], (-90.0, 90.0)));
        }
        marks
    }
}

/// A strip to tell by its shadows.
impl Original for TextureTask {
    fn givens(&self) -> Vec<SheetMark> {
        marks::givens()
    }

    /// Once anything lies in the copy's frame, read from its pixels on the compare worker.
    fn compare(&self, asked: &Asked<'_>) -> Option<Compare> {
        let reading = || TextureReading {
            task: Arc::new(self.clone()),
        };
        (asked.held()).then(|| Compare::Read(Reading::of(reading())))
    }

    /// The strip's picture, `OBJECT_SIDE` wide and half as high.
    fn painted(&self, _picture: Option<&SheetPicture>) -> Option<Painted> {
        Some(([OBJECT_SIDE, OBJECT_SIDE / 2], self.picture(OBJECT_SIDE)))
    }
}
