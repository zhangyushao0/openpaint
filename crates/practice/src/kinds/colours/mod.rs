//! Single colours to match (`Kind::Colour`): their tones and scores, the colour radar's judgement
//! (D108), their How, tile and lessons; compared by the colours laid (`marks.rs`).
//!
//! A lesson (a colour about 1.75 minutes) holds three: the first level its first three
//! (muted, near-grey and middle, each of another lightness), the second muted and near-grey ones,
//! the third near-greys alone, where the hue hides (V大 stresses the low saturations); each
//! level's lightnesses in turn by the lesson.

pub(crate) mod colour_match;
pub(crate) mod demo;
pub(crate) mod judgement;
pub(crate) mod marks;

use std::sync::Arc;

use crate::course::picker::{in_turn, laid, rotated};
use crate::demos::demo::Demo;
use crate::kinds::{
    Blank, Counted, Described, Facts, Kept, Kind, Original, Picking, SkillLevel, ToolSet, Turn,
    held,
};
use crate::marks::asked::{Asked, Compare};
use crate::marks::mark::SheetMark;
use crate::marks::tile::{TileMark, swatch};
use crate::maths::colour::Rgb8;
use crate::packs::pack::ItemRef;
use crate::progress::scaled;
use crate::records::metric::{Metric, Metrics, value};
use crate::sheets::exercise::Exercise;
use colour_match::{Lightness, Purity, Tone};
use judgement::{NOTICEABLE, PART_SPAN};

pub(crate) struct Colours;

const FACTS: [Facts; 1] = [Facts::of("colour", ToolSet::COLOUR, 105)
    .picks(Picking::SWATCH)
    .counted(Counted::Colour)
    .kept(Kept::Pick)];
/// A lesson's colours.
const COUNT: usize = 3;
/// The first level's: unit 1's first three before S122.
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

    fn generate(&self, exercise: &Exercise) -> Arc<dyn Original> {
        let Kind::Colour(tone) = exercise.kind else {
            return held(Blank);
        };
        held(Swatch(tone.colour(exercise.seed)))
    }

    fn how(&self, _kind: Kind, _cells: usize) -> Option<Demo> {
        Some(demo::colour())
    }

    /// The difference at the first Compare, cut at a noticeable step and a part's span.
    fn measured(&self, _exercise: &Exercise, metrics: &Metrics) -> Option<f64> {
        let off = value(metrics, Metric::FinalDelta)?;
        Some(scaled(off, [NOTICEABLE, PART_SPAN]))
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

    fn glyph(&self) -> Vec<TileMark> {
        swatch()
    }
}

/// A colour's original: its swatch.
#[derive(Debug)]
pub(crate) struct Swatch(Rgb8);

impl Original for Swatch {
    fn givens(&self) -> Vec<SheetMark> {
        marks::givens(self.0)
    }

    fn compare(&self, asked: &Asked<'_>) -> Option<Compare> {
        marks::compare(asked)
    }

    fn swatch(&self) -> Option<Rgb8> {
        Some(self.0)
    }
}
