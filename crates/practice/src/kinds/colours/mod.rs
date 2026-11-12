//! Single colours to match (`Kind::Colour`): their tones and scores, the colour radar's judgement
//! (D108), their How, tile and lessons; compared by the colours laid (`marks.rs`).
//!
//! A lesson (a colour about 1.75 minutes) holds three: the first level stage 1's first three
//! (muted, near-grey and middle, each of another lightness), the second muted and near-grey ones,
//! the third near-greys alone, where the hue hides (V大 stresses the low saturations); each
//! level's lightnesses in turn by the lesson.

pub(crate) mod colour_match;
pub(crate) mod demo;
pub(crate) mod judgement;
pub(crate) mod marks;

use crate::course::picker::{in_turn, laid, rotated};
use crate::demos::demo::Demo;
use crate::kinds::{Described, Facts, Kept, Kind, Original, Picks, SkillLevel, Tools, Turn};
use crate::marks::asked::{Asked, Compare};
use crate::marks::mark::SheetMark;
use crate::marks::tile::{TileMark, swatch};
use crate::packs::pack::ItemRef;
use crate::sheets::exercise::Exercise;
use crate::sheets::family::Family;
use colour_match::{Lightness, Purity, Tone};

pub(crate) struct Colours;

const FACTS: [Facts; 1] = [Facts::of(Family::Colour, "colour", Tools::Colour, 105)
    .picks(Picks::Swatch)
    .kept(Kept::Pick)];
/// A lesson's colours.
const COUNT: usize = 3;
/// The first level's: stage 1's first three.
const FIRST: [(Purity, Lightness); 3] = [
    (Purity::Muted, Lightness::Middle),
    (Purity::Grey, Lightness::Light),
    (Purity::Middle, Lightness::Dark),
];
/// The second level's, three in turn from the lesson's place.
const SECOND: [(Purity, Lightness); 6] = [
    (Purity::Muted, Lightness::Dark),
    (Purity::Grey, Lightness::Middle),
    (Purity::Muted, Lightness::Light),
    (Purity::Grey, Lightness::Dark),
    (Purity::Muted, Lightness::Middle),
    (Purity::Grey, Lightness::Light),
];
/// The third level's lightnesses, in turn.
const THIRD: [Lightness; 3] = [Lightness::Dark, Lightness::Light, Lightness::Middle];

impl Described for Colours {
    fn code(&self) -> u8 {
        2
    }

    fn facts(&self) -> &'static [Facts] {
        &FACTS
    }

    fn variant(&self, kind: Kind) -> Option<u8> {
        match kind {
            Kind::Colour(tone) => Some(tone.code()),
            _ => None,
        }
    }

    fn of_variant(&self, variant: u8) -> Option<Kind> {
        Tone::of_code(variant).map(Kind::Colour)
    }

    fn original(&self, exercise: &Exercise) -> Original {
        let Kind::Colour(tone) = exercise.kind else {
            return exercise.picture();
        };
        Original::Colour(tone.colour(exercise.seed))
    }

    fn givens(&self, _exercise: &Exercise, original: &Original) -> Vec<SheetMark> {
        original.swatch().map(marks::givens).unwrap_or_default()
    }

    fn how(&self, _family: Family) -> Option<Demo> {
        Some(demo::colour())
    }

    fn lesson(&self, level: SkillLevel, turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        let tones = match level {
            SkillLevel::First => FIRST.to_vec(),
            SkillLevel::Second => in_turn(&SECOND, turn.turn, COUNT),
            SkillLevel::Third => (rotated(&THIRD, turn.turn, COUNT).into_iter())
                .map(|lightness| (Purity::Grey, lightness))
                .collect(),
        };
        let colour = |(purity, lightness)| Kind::Colour(Tone { purity, lightness });
        laid(tones.into_iter().map(colour))
    }

    fn compare(&self, asked: &Asked<'_>) -> Option<Compare> {
        marks::compare(asked)
    }

    fn tile(&self, _family: Family) -> Vec<TileMark> {
        swatch()
    }
}
