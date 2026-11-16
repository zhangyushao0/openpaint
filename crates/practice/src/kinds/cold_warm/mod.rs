//! Cold-warm copies (`Kind::ColdWarm`, S154, V大's 整图变调 of weeks 7 and 8): a long colour
//! copy of a whole picture painted shifted toward two tendencies at once, its lights toward a warm
//! and its shadows toward a cool about opposite (`leans.rs`), its rich local colours still told
//! apart after the shift. A project beside the lessons, as a long colour copy is: one pair of
//! frames of its picture's shape on a sheet of its own (`Purpose::Copy`, kept until removed),
//! painted in the full interface; the two tendencies given beside the copy's frame as swatches,
//! named and picked whole (`Facts::beside` leaves the sheet room for them). Compared as a long
//! copy with a Grey view and a disc (`marks.rs`); no result, no lesson, no dot. Begun from New
//! copy… or unit 7's path page (`Sheet::cold_warm`).

pub(crate) mod leans;
pub(crate) mod marks;

use std::sync::Arc;

use crate::kinds::{Blank, Described, Facts, Kind, Original, Picking, ToolSet, held};
use crate::marks::asked::{Asked, Compare, Reading};
use crate::marks::drawing::pictured;
use crate::marks::mark::{BESIDE_SIDE, Figure, In, SheetMark};
use crate::marks::tile::{TileMark, closed, square};
use crate::packs::pack::ItemRef;
use crate::records::sheet::Sheet;
use crate::sheets::exercise::Exercise;
use leans::Leans;
use marks::ShiftReading;

pub(crate) struct ColdWarm;

/// A cold-warm copy runs as long as a colour copy; no lesson counts it.
const FACTS: [Facts; 1] = [Facts::of("cold-warm", ToolSet::FULL, 36_000)
    .picks(Picking::PROBED)
    .beside()];
/// The two swatches beside the copy's frame: their middles' distance from its right side, the
/// lights' height and the gap down to the shadows' (frame units, room for a name under each).
const OUT: f64 = 0.02 + BESIDE_SIDE / 2.0;
const LIGHTS_AT: f64 = BESIDE_SIDE / 2.0;
const DOWN: f64 = 2.2 * BESIDE_SIDE;

impl Described for ColdWarm {
    fn code(&self) -> u8 {
        32
    }

    fn facts(&self) -> &'static [Facts] {
        &FACTS
    }

    fn variant(&self, kind: Kind) -> Option<u8> {
        match kind {
            Kind::ColdWarm(leans) => Some(leans.code()),
            _ => None,
        }
    }

    fn of_variant(&self, variant: u8) -> Option<Kind> {
        Leans::of_code(variant).map(Kind::ColdWarm)
    }

    fn generate(&self, exercise: &Exercise) -> Arc<dyn Original> {
        let Kind::ColdWarm(leans) = exercise.kind else {
            return held(Blank);
        };
        held(Shifted {
            picture: exercise.item(),
            leans,
        })
    }

    /// Two swatches, a warm over a cool.
    fn glyph(&self) -> Vec<TileMark> {
        let warm = square([12.0, 9.0], [26.0, 31.0]);
        let cool = square([30.0, 9.0], [44.0, 31.0]);
        vec![
            TileMark::Tint(warm.to_vec(), 0.25),
            TileMark::Tint(cool.to_vec(), 0.6),
            TileMark::Line(closed(&warm)),
            TileMark::Line(closed(&cool)),
        ]
    }
}

/// A cold-warm copy's original: the picture it copies and the pair it leans toward.
#[derive(Debug)]
pub(crate) struct Shifted {
    picture: ItemRef,
    leans: Leans,
}

impl Original for Shifted {
    /// The picture in its frame; beside the copy's the lights' warm over the shadows' cool.
    fn givens(&self) -> Vec<SheetMark> {
        let swatch = |y: f64, colour, name| Figure::Beside {
            at: [OUT, y],
            colour,
            name: Some(name),
        };
        let mut marks = pictured();
        marks.push(SheetMark::free(
            In::Copy,
            swatch(LIGHTS_AT, self.leans.lights(), "lights"),
        ));
        marks.push(SheetMark::free(
            In::Copy,
            swatch(LIGHTS_AT + DOWN, self.leans.shadows(), "shadows"),
        ));
        marks
    }

    /// Once anything lies in its frame and its picture is held: laid over it, read for its Grey
    /// view.
    fn compare(&self, asked: &Asked<'_>) -> Option<Compare> {
        if !asked.held() {
            return None;
        }
        let corners = asked.corners()?;
        Some(Compare::Read(Reading::of(ShiftReading {
            corners,
            leans: self.leans,
        })))
    }

    fn picture(&self) -> Option<&ItemRef> {
        Some(&self.picture)
    }
}

impl Sheet {
    /// A cold-warm copy of `picture` (S154), the `id`th of all sheets, seeded `seed`, begun at
    /// `started`: a project as a colour copy is, its tendencies the pair after those of the
    /// cold-warm copies among `sheets` (`Leans::nth`).
    #[must_use]
    pub fn cold_warm(id: u64, seeded: (u64, u64), picture: ItemRef, sheets: &[Self]) -> Self {
        let shifted = |sheet: &&Self| {
            (sheet.exercises().next())
                .is_some_and(|laid| matches!(laid.exercise.kind, Kind::ColdWarm(_)))
        };
        let earlier = sheets
            .iter()
            .filter(|sheet| sheet.is_copy())
            .filter(shifted);
        Self::project(
            Kind::ColdWarm(Leans::nth(earlier.count())),
            id,
            seeded,
            picture,
        )
    }
}
