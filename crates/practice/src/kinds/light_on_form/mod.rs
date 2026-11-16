//! Light on form (`Kind::Light`, S153, 明暗, unit 6): from a known light, where a form's light
//! family ends and its shadow family begins (the terminator and the cast shadow), then the tones
//! inside each family in order; a form seen as planes turned to or from the light. A form stands
//! on the ground in a sun the pin beside it gives (`scene.rs`); the model paints each point's family
//! and tone (`answer.rs`); the copy is painted in the level's greys and read pixel by pixel against
//! the families, which the app judges (D123), the inner tones hatched as help (`read.rs`).
//!
//! A lesson holds two forms (about 2.5 minutes each): at 入门 a box, a cylinder and a sphere in
//! turn, lit from three-quarter front, in the light and the dark; at 熟练 the same and a cone, lit
//! from the side or from behind in turn, the middle grey too; at 稳 a round form and, every second
//! lesson, a carved object (else another round form), the darkest too: the core, the reflected
//! light and the contact.

pub(crate) mod answer;
pub(crate) mod construction;
pub(crate) mod demo;
pub(crate) mod marks;
pub(crate) mod outline;
pub(crate) mod read;
pub(crate) mod scene;
pub(crate) mod survey;
pub(crate) mod trace;

#[cfg(test)]
mod tests;

use std::sync::Arc;

use crate::course::picker::{in_turn, laid};
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
use marks::LightReading;
use scene::{LightTask, Lit, Tones};

pub(crate) struct LightOnForm;

/// Its topics, one a level's greys: the families alone, the halftone too, the darkest too.
const FACTS: [Facts; 3] = [
    Facts::of("light-families", ToolSet::TWO_GREYS, 150).constructed(),
    Facts::of("light-halftone", ToolSet::GREYS, 150).constructed(),
    Facts::of("light-core", ToolSet::FOUR_GREYS, 150).constructed(),
];
/// The variants each level takes (`Lit::ALL`'s): 入门's forms; 熟练's from the side and from
/// behind; 稳's round forms and its carved object.
const FIRST: [Lit; 3] = [Lit::ALL[0], Lit::ALL[1], Lit::ALL[2]];
const SIDE: [Lit; 4] = [Lit::ALL[3], Lit::ALL[4], Lit::ALL[5], Lit::ALL[6]];
const BEHIND: [Lit; 4] = [Lit::ALL[7], Lit::ALL[8], Lit::ALL[9], Lit::ALL[10]];
const ROUND: [Lit; 3] = [Lit::ALL[11], Lit::ALL[12], Lit::ALL[13]];
const OBJECT: Lit = Lit::ALL[14];
/// Forms a lesson.
const COUNT: usize = 2;

impl Described for LightOnForm {
    fn code(&self) -> u8 {
        26
    }

    fn facts(&self) -> &'static [Facts] {
        &FACTS
    }

    fn key(&self) -> &'static str {
        "light-on-form"
    }

    fn variant(&self, kind: Kind) -> Option<u8> {
        match kind {
            Kind::Light(lit) => Some(place(&Lit::ALL, &lit)),
            _ => None,
        }
    }

    fn of_variant(&self, variant: u8) -> Option<Kind> {
        (Lit::ALL.get(usize::from(variant))).map(|lit| Kind::Light(*lit))
    }

    fn topic(&self, kind: Kind) -> &'static Facts {
        match kind {
            Kind::Light(Lit {
                tones: Tones::Three,
                ..
            }) => &FACTS[1],
            Kind::Light(Lit {
                tones: Tones::Four, ..
            }) => &FACTS[2],
            _ => &FACTS[0],
        }
    }

    fn generate(&self, exercise: &Exercise) -> Arc<dyn Original> {
        let Kind::Light(lit) = exercise.kind else {
            return held(Blank);
        };
        held(LightTask::generate(lit, exercise.seed))
    }

    fn how(&self, kind: Kind, _cells: usize) -> Option<Demo> {
        let Kind::Light(lit) = kind else {
            return None;
        };
        Some(demo::light_on_form(lit))
    }

    fn lesson(&self, level: SkillLevel, turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        let lits = match level {
            SkillLevel::First => in_turn(&FIRST, turn.turn, COUNT),
            SkillLevel::Second => {
                let lit = if turn.turn.is_multiple_of(2) {
                    &SIDE
                } else {
                    &BEHIND
                };
                in_turn(lit, turn.turn / 2, COUNT)
            }
            SkillLevel::Third => {
                let round = ROUND[turn.turn % ROUND.len()];
                let beside = if turn.turn % 2 == 1 {
                    OBJECT
                } else {
                    ROUND[(turn.turn + 1) % ROUND.len()]
                };
                vec![round, beside]
            }
        };
        laid(lits.into_iter().map(Kind::Light))
    }

    /// A ball on the ground, its side turned away and its cast shadow tinted, a pin beside it.
    fn glyph(&self) -> Vec<TileMark> {
        let (centre, radius) = ([22.0, 18.0], 9.5);
        let turned: Vec<[f32; 2]> = curve(|t| {
            let turn = (-40.0 + 180.0 * t).to_radians();
            [
                centre[0] + radius * turn.cos(),
                centre[1] + radius * turn.sin(),
            ]
        });
        let cast: Vec<[f32; 2]> = curve(|t| {
            let turn = std::f32::consts::TAU * t;
            [31.0 + 12.0 * turn.cos(), 29.5 + 3.0 * turn.sin()]
        });
        vec![
            TileMark::Tint(cast, 0.6),
            TileMark::Tint(turned, 0.6),
            arc(centre, [radius; 2], (0.0, 360.0)),
            TileMark::Line(vec![[44.0, 30.0], [44.0, 19.0]]),
            TileMark::Dot([44.0, 19.0]),
            TileMark::Line(vec![[44.0, 30.0], [50.0, 33.5]]),
        ]
    }
}

/// A form in the light to paint in its families.
impl Original for LightTask {
    fn givens(&self) -> Vec<SheetMark> {
        marks::givens(self)
    }

    /// Once anything lies in the copy's frame, read from its pixels against the model.
    fn compare(&self, asked: &Asked<'_>) -> Option<Compare> {
        let reading = || LightReading {
            task: Arc::new(self.clone()),
        };
        (asked.held()).then(|| Compare::Read(Reading::of(reading())))
    }

    /// The answer in the level's greys, `OBJECT_SIDE` square.
    fn painted(&self, _picture: Option<&SheetPicture>) -> Option<Painted> {
        Some(([OBJECT_SIDE; 2], self.picture(OBJECT_SIDE)))
    }
}
