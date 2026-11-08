//! Single colours to match (`Kind::Colour`): their tones and scores, the colour radar's judgement
//! the summary shows (D108), their How and tile; compared by the colours laid (`marks.rs`).

pub(crate) mod colour_match;
pub(crate) mod demo;
pub(crate) mod judgement;
pub(crate) mod marks;

use crate::demos::demo::Demo;
use crate::kinds::{Described, Facts, Kept, Kind, Original, Picks, Turn, Work};
use crate::marks::asked::{Asked, Compare};
use crate::marks::mark::SheetMark;
use crate::marks::tile::{TileMark, swatch};
use crate::packs::pack::ItemRef;
use crate::records::metric::Metrics;
use crate::records::record::COLOUR_ROWS;
use crate::sessions::family::Family;
use crate::sessions::plan::Exercise;
use crate::sessions::session::COLOURS;
use colour_match::Tone;
use judgement::Judgement;

pub(crate) struct Colours;

const FACTS: [Facts; 1] = [Facts::of(Family::Colour, "colour", Work::Colour)
    .picks(Picks::Swatch)
    .kept(Kept::Pick)
    .rows(&COLOUR_ROWS)];

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

    /// A drill: stage 1's six colours.
    fn drill(&self, _family: Family, _turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        (COLOURS.into_iter())
            .map(|(purity, lightness)| (Kind::Colour(Tone { purity, lightness }), None))
            .collect()
    }

    fn compare(&self, asked: &Asked<'_>) -> Option<Compare> {
        marks::compare(asked)
    }

    fn tile(&self, _family: Family) -> Vec<TileMark> {
        swatch()
    }

    /// The colour radar's judgement of the colours among `results`.
    fn judgement(&self, results: &[(Kind, &Metrics)]) -> Option<Judgement> {
        let colours: Vec<(Tone, &[(crate::records::metric::Metric, f64)])> = (results.iter())
            .filter_map(|(kind, metrics)| match kind {
                Kind::Colour(tone) => Some((*tone, metrics.as_slice())),
                _ => None,
            })
            .collect();
        Some(judgement::judgement(&colours))
    }
}
