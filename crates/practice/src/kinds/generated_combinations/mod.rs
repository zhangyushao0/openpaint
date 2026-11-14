//! The generated colour combinations (`Kind::Areas`, S82, painted 1.5.0 to 1.18.1): a kind kept for
//! its records alone, never planned or drilled since S101 (the author, 2026-11-07); its picture
//! made again from its seed (`colour_areas/areas_generated.rs`). It goes by the colour
//! combinations' topic (`combinations/`): a record of one shows and compares as a combination
//! does.

use std::sync::Arc;

use crate::colour_areas::areas::Areas;
use crate::colour_areas::areas_generated::{Arrangement, GeneratedAreas};
use crate::kinds::{Blank, Described, Facts, Kind, Original, held, place};
use crate::marks::asked::{Asked, Compare, Painted};
use crate::marks::mark::SheetMark;
use crate::maths::colour::Rgb8;
use crate::maths::path::Point;
use crate::sheets::exercise::Exercise;
use crate::sheets::sheet_picture::SheetPicture;

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

    fn topic(&self, _kind: Kind) -> &'static Facts {
        &crate::kinds::combinations::FACTS[0]
    }

    fn generate(&self, exercise: &Exercise) -> Arc<dyn Original> {
        let Kind::Areas(arrangement) = exercise.kind else {
            return held(Blank);
        };
        held(GeneratedAreas::generate(arrangement, exercise.seed))
    }
}

/// A generated colour combination's picture and areas (a record of 1.18.1 and before).
impl Original for GeneratedAreas {
    /// Its picture drawn again, as the combinations' marks lay it.
    fn givens(&self) -> Vec<SheetMark> {
        crate::kinds::combinations::marks::generated_givens()
    }

    fn compare(&self, asked: &Asked<'_>) -> Option<Compare> {
        crate::kinds::combinations::reading::compare(asked)
    }

    /// Its picture made again from its areas, `AREAS_SIDE` square.
    fn painted(&self, _picture: Option<&SheetPicture>) -> Option<Painted> {
        Some(([AREAS_SIDE; 2], GeneratedAreas::picture(self, AREAS_SIDE)))
    }

    /// Its square frame whole.
    fn areas(&self) -> Option<(&Areas, Point)> {
        Some((&self.areas, [1.0, 1.0]))
    }

    fn colour_at(&self, at: Point) -> Option<Rgb8> {
        Some(GeneratedAreas::colour_at(self, at, 0.0))
    }
}
