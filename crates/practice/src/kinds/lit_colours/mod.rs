//! Colour under light (`Kind::LitColour`, S154j, 光色 / 条件色, unit 7): predicting what a light's
//! colour and its surroundings do to a local colour: the light family takes the light's colour,
//! the shadow family the sky's (or a bounce's) and goes darker, and the object's own colour stays
//! in both (Gurney's local colour modified by the light; Bucci's warm against cool light; 色彩's
//! 固有色, 光源色, 环境色). Light on form's scene in colour (`scene.rs`), its exact families as
//! regions (`regions.rs`), a model's render as one answer (`render.rs`), rules read directly per
//! region (`check.rs`) and made wrong on purpose for the contact sheet (`wrong.rs`); the copy is
//! read on the compare worker and the marks are help: the learner's eye judges (D123,
//! `marks.rs`).
//!
//! A lesson holds one scene (about 6 minutes): at 入门 a box, a cylinder and a sphere in turn, white
//! or light grey on a grey ground, a warm sun and a blue sky in turn with a cool light and a warm
//! sky; at 熟练 the same and a cone, a coloured object of any of the wheel's twelve hues, drawn
//! afresh each scene so no habit forms; at 稳 a ball under a coloured lamp of any hue on a coloured
//! ground whose light bounces into its shadow.

pub(crate) mod check;
pub(crate) mod colours;
pub(crate) mod demo;
pub(crate) mod marks;
pub(crate) mod regions;
pub(crate) mod render;
pub(crate) mod scene;
pub(crate) mod wrong;

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
use crate::marks::tile::{TileMark, arc, closed, curve};
use crate::packs::pack::ItemRef;
use crate::sheets::exercise::Exercise;
use crate::sheets::sheet_picture::SheetPicture;
use marks::LitReading;
use scene::{ColourTask, LitColour, Palette};

pub(crate) struct LitColours;

/// Its topics: the light and the sky on a form (入门, 熟练), and the ground's bounce too (稳).
const FACTS: [Facts; 2] = [
    Facts::of("lit-colour", ToolSet::PAINTING, 360).constructed(),
    Facts::of("lit-colour-bounce", ToolSet::PAINTING, 360).constructed(),
];
/// The variants each level takes in turn (`LitColour::ALL`'s): 入门's forms under the warm sun and
/// under the cool light; 熟练's coloured forms; 稳's ball under a lamp.
const FIRST: [LitColour; 6] = [
    LitColour::ALL[0],
    LitColour::ALL[4],
    LitColour::ALL[2],
    LitColour::ALL[3],
    LitColour::ALL[1],
    LitColour::ALL[5],
];
const SECOND: [LitColour; 4] = [
    LitColour::ALL[6],
    LitColour::ALL[7],
    LitColour::ALL[8],
    LitColour::ALL[9],
];
const THIRD: LitColour = LitColour::ALL[10];

impl Described for LitColours {
    fn code(&self) -> u8 {
        29
    }

    fn facts(&self) -> &'static [Facts] {
        &FACTS
    }

    fn key(&self) -> &'static str {
        "colour-under-light"
    }

    fn variant(&self, kind: Kind) -> Option<u8> {
        match kind {
            Kind::LitColour(lit) => Some(place(&LitColour::ALL, &lit)),
            _ => None,
        }
    }

    fn of_variant(&self, variant: u8) -> Option<Kind> {
        (LitColour::ALL.get(usize::from(variant))).map(|lit| Kind::LitColour(*lit))
    }

    fn topic(&self, kind: Kind) -> &'static Facts {
        match kind {
            Kind::LitColour(LitColour {
                palette: Palette::Bounce,
                ..
            }) => &FACTS[1],
            _ => &FACTS[0],
        }
    }

    fn generate(&self, exercise: &Exercise) -> Arc<dyn Original> {
        let Kind::LitColour(lit) = exercise.kind else {
            return held(Blank);
        };
        held(ColourTask::generate(lit, exercise.seed))
    }

    fn how(&self, kind: Kind, _cells: usize) -> Option<Demo> {
        let Kind::LitColour(lit) = kind else {
            return None;
        };
        Some(demo::colour_under_light(lit))
    }

    fn lesson(&self, level: SkillLevel, turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        let lit = match level {
            SkillLevel::First => FIRST[turn.turn % FIRST.len()],
            SkillLevel::Second => SECOND[turn.turn % SECOND.len()],
            SkillLevel::Third => THIRD,
        };
        laid([Kind::LitColour(lit)])
    }

    fn window(&self) -> usize {
        3
    }

    /// A ball, its shadow side and its cast shadow tinted, three swatches beside it.
    fn glyph(&self) -> Vec<TileMark> {
        let (centre, radius) = ([20.0, 19.0], 10.0);
        let turned: Vec<[f32; 2]> = curve(|t| {
            let turn = (-40.0 + 180.0 * t).to_radians();
            [
                centre[0] + radius * turn.cos(),
                centre[1] + radius * turn.sin(),
            ]
        });
        let cast: Vec<[f32; 2]> = curve(|t| {
            let turn = std::f32::consts::TAU * t;
            [28.0 + 10.0 * turn.cos(), 30.5 + 2.5 * turn.sin()]
        });
        let mut marks = vec![
            TileMark::Tint(cast, 0.3),
            TileMark::Tint(turned, 0.55),
            arc(centre, [radius; 2], (0.0, 360.0)),
        ];
        for (top, tint) in [(8.0, 0.2), (17.0, 0.05), (26.0, 0.4)] {
            let swatch = [
                [42.0, top],
                [47.0, top],
                [47.0, top + 5.0],
                [42.0, top + 5.0],
            ];
            marks.push(TileMark::Tint(swatch.to_vec(), tint));
            marks.push(TileMark::Line(closed(&swatch)));
        }
        marks
    }
}

/// A form in coloured light to paint.
impl Original for ColourTask {
    fn givens(&self) -> Vec<SheetMark> {
        marks::givens(self)
    }

    /// Once anything lies in the copy's frame, read from its pixels on the compare worker.
    fn compare(&self, asked: &Asked<'_>) -> Option<Compare> {
        let reading = || LitReading {
            task: Arc::new(self.clone()),
        };
        (asked.held()).then(|| Compare::Read(Reading::of(reading())))
    }

    /// The model's render, `OBJECT_SIDE` square: one answer.
    fn painted(&self, _picture: Option<&SheetPicture>) -> Option<Painted> {
        Some(([OBJECT_SIDE; 2], self.render(OBJECT_SIDE)))
    }
}
