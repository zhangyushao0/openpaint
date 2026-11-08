//! Colour combinations (`Kind::Crop`, V大's 临摹色彩组合, S101): a pack's crop, its few colour areas
//! copied by eye and read against the areas the pack holds (`colour_areas/`, `reading.rs`). Its How
//! plays on a generated example (`demo.rs`).

pub(crate) mod demo;
pub(crate) mod marks;
pub(crate) mod reading;

use crate::demos::demo::Demo;
use crate::kinds::{Described, Facts, Kind, Needs, Original, Picks, Turn, Work};
use crate::marks::asked::{Asked, Compare};
use crate::marks::mark::SheetMark;
use crate::marks::tile::{TileMark, closed};
use crate::packs::pack::{ItemRef, Tier};
use crate::records::record::AREAS_ROWS;
use crate::sessions::family::Family;
use crate::sessions::plan::Exercise;
use crate::sessions::session::Picker;

pub(crate) struct Crops;

const FACTS: [Facts; 1] = [Facts::of(Family::Areas, "combination", Work::Painted)
    .picks(Picks::Probed)
    .needs(Needs::Crops)
    .rows(&AREAS_ROWS)];
/// How many combinations a drill holds.
const DRILLED: usize = 2;

impl Described for Crops {
    fn code(&self) -> u8 {
        18
    }

    fn facts(&self) -> &'static [Facts] {
        &FACTS
    }

    fn variant(&self, kind: Kind) -> Option<u8> {
        match kind {
            Kind::Crop(tier) => Some(tier.code()),
            _ => None,
        }
    }

    fn of_variant(&self, variant: u8) -> Option<Kind> {
        Tier::of_code(variant).map(Kind::Crop)
    }

    fn givens(&self, _exercise: &Exercise, _original: &Original) -> Vec<SheetMark> {
        crate::marks::drawing::pictured()
    }

    fn original(&self, exercise: &Exercise) -> Original {
        exercise.picture()
    }

    fn how(&self, _family: Family) -> Option<Demo> {
        Some(demo::areas())
    }

    /// A drill: two crops not shown before while any are left.
    fn drill(&self, _family: Family, turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        let mut picker = Picker::new(turn.seed, turn.before, Tier::Simple);
        (0..DRILLED).filter_map(|_| picker.crop()).collect()
    }

    fn compare(&self, asked: &Asked<'_>) -> Option<Compare> {
        reading::compare(asked)
    }

    /// Three bands in a frame, darker downwards.
    fn tile(&self, _family: Family) -> Vec<TileMark> {
        let (left, right) = (9.0, 45.0);
        let edges = [(8.0, 8.0), (15.0, 19.0), (25.0, 22.0), (32.0, 32.0)];
        let mut marks: Vec<TileMark> = (edges.windows(2).enumerate())
            .map(|(band, pair)| {
                let ((a, b), (c, d)) = (pair[0], pair[1]);
                let quad = vec![[left, a], [right, b], [right, d], [left, c]];
                TileMark::Tint(quad, 0.12 + 0.2 * band as f32)
            })
            .collect();
        let outline = [[left, 8.0], [right, 8.0], [right, 32.0], [left, 32.0]];
        marks.push(TileMark::Line(closed(&outline)));
        for (a, b) in &edges[1..3] {
            marks.push(TileMark::Line(vec![[left, *a], [right, *b]]));
        }
        marks
    }
}
