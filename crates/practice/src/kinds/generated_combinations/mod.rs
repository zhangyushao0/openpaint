//! The generated colour combinations (`Kind::Areas`, S82, painted 1.5.0 to 1.18.1): a kind kept for
//! its records alone, never planned or drilled since S101 (the author, 2026-11-07); its picture
//! made again from its seed (`colour_areas/areas_generated.rs`). It goes by the colour
//! combinations' family (`combinations/`): a record of one shows and compares as a combination
//! does.

use crate::colour_areas::areas_generated::{Arrangement, GeneratedAreas};
use crate::kinds::{Described, Facts, Kind, Original, place};
use crate::marks::asked::{Asked, Compare, Painted};
use crate::marks::mark::SheetMark;
use crate::sessions::family::Family;
use crate::sessions::plan::Exercise;
use crate::sessions::sheet_picture::SheetPicture;

/// The side its picture is drawn at, in pixels: one to a frame's canvas pixel.
pub const AREAS_SIDE: u32 = 400;

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

    fn compare(&self, asked: &Asked<'_>) -> Option<Compare> {
        crate::kinds::combinations::reading::compare(asked)
    }

    /// Its picture made again from its areas, `AREAS_SIDE` square.
    fn painted(&self, original: &Original, _picture: Option<&SheetPicture>) -> Option<Painted> {
        let Original::Areas(generated) = original else {
            return None;
        };
        Some(([AREAS_SIDE; 2], generated.picture(AREAS_SIDE)))
    }
}
