//! The generated colour combinations (`Kind::Areas`, S82, painted 1.5.0 to 1.18.1): a kind kept for
//! its records alone, never planned or drilled since S101 (the author, 2026-11-07); its picture
//! made again from its seed (`colour_areas/areas_generated.rs`). It goes by the colour
//! combinations' family (`combinations/`): a record of one shows as a combination does.

use crate::colour_areas::areas_generated::{Arrangement, GeneratedAreas};
use crate::kinds::{Described, Facts, Kind, Original, place};
use crate::marks::mark::SheetMark;
use crate::sessions::family::Family;
use crate::sessions::plan::Exercise;

pub(crate) struct Generated;

impl Described for Generated {
    fn code(&self) -> u8 {
        5
    }

    fn facts(&self) -> &'static [Facts] {
        &[]
    }

    fn variant(&self, kind: Kind) -> Option<u8> {
        match kind {
            Kind::Areas(arrangement) => Some(place(&Arrangement::ALL, &arrangement)),
            _ => None,
        }
    }

    fn of_variant(&self, variant: u8) -> Option<Kind> {
        (Arrangement::ALL.get(usize::from(variant))).map(|a| Kind::Areas(*a))
    }

    fn family(&self, _kind: Kind) -> Family {
        Family::Areas
    }

    /// Its picture drawn again, as the combinations' marks lay it.
    fn givens(&self, _exercise: &Exercise, _original: &Original) -> Vec<SheetMark> {
        crate::kinds::combinations::marks::generated_givens()
    }

    fn original(&self, exercise: &Exercise) -> Original {
        let Kind::Areas(arrangement) = exercise.kind else {
            return exercise.picture();
        };
        Original::Areas(GeneratedAreas::generate(arrangement, exercise.seed))
    }
}
