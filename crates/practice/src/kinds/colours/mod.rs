//! Single colours to match (`Kind::Colour`): their tones and scores, the colour radar's
//! judgement, and the How of colours, colour combinations and value scales.

pub(crate) mod colour_match;
pub(crate) mod demo_colour;
pub(crate) mod judgement;
pub(crate) mod marks;

use crate::demos::demo::Demo;
use crate::kinds::{Described, Facts, Kind, Original, Turn, Work};
use crate::marks::mark::SheetMark;
use crate::packs::pack::ItemRef;
use crate::sessions::family::Family;
use crate::sessions::plan::Exercise;
use crate::sessions::session::COLOURS;
use colour_match::Tone;

pub(crate) struct Colours;

const FACTS: [Facts; 1] = [Facts::of(Family::Colour, "colour", Work::Colour)];

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
        match original {
            Original::Colour(colour) => marks::givens(*colour),
            _ => Vec::new(),
        }
    }

    fn how(&self, _family: Family) -> Option<Demo> {
        Some(demo_colour::colour())
    }

    /// A drill: stage 1's six colours.
    fn drill(&self, _family: Family, _turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        (COLOURS.into_iter())
            .map(|(purity, lightness)| (Kind::Colour(Tone { purity, lightness }), None))
            .collect()
    }
}
