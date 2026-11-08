//! Key shifts (`Kind::KeyShift`, S107, V大's 变调): tendencies, a shift by rule, the check, how a
//! copy is read (`reading.rs`), the How and the tile.

pub(crate) mod demo;
pub(crate) mod key_shift;
pub(crate) mod key_shift_check;
pub(crate) mod key_shift_rule;
pub(crate) mod marks;
pub(crate) mod reading;

use crate::demos::demo::Demo;
use crate::kinds::{Described, Facts, Kind, Needs, Original, Picks, Turn, Work};
use crate::marks::asked::{Asked, Compare};
use crate::marks::mark::SheetMark;
use crate::marks::tile::{TileMark, closed, square};
use crate::packs::pack::{ItemRef, Tier};
use crate::sessions::family::Family;
use crate::sessions::plan::Exercise;
use crate::sessions::session::Picker;
use crate::sessions::session_five::key_shifts;
use key_shift::Tendency;

pub(crate) struct KeyShifts;

const FACTS: [Facts; 1] = [Facts::of(Family::KeyShift, "key-shift", Work::Painted)
    .picks(Picks::Locked)
    .needs(Needs::Crops)];
/// How many key shifts a drill holds.
const DRILLED: usize = 2;

impl Described for KeyShifts {
    fn code(&self) -> u8 {
        19
    }

    fn facts(&self) -> &'static [Facts] {
        &FACTS
    }

    fn variant(&self, kind: Kind) -> Option<u8> {
        match kind {
            Kind::KeyShift(tendency) => Some(tendency.code()),
            _ => None,
        }
    }

    fn of_variant(&self, variant: u8) -> Option<Kind> {
        Tendency::of_code(variant).map(Kind::KeyShift)
    }

    fn original(&self, exercise: &Exercise) -> Original {
        exercise.picture()
    }

    fn givens(&self, exercise: &Exercise, _original: &Original) -> Vec<SheetMark> {
        match exercise.kind {
            Kind::KeyShift(tendency) => marks::givens(tendency),
            _ => Vec::new(),
        }
    }

    fn how(&self, _family: Family) -> Option<Demo> {
        Some(demo::key_shift())
    }

    /// A drill: two crops, their leans in turn.
    fn drill(&self, _family: Family, turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        let mut picker = Picker::new(turn.seed, turn.before, Tier::Simple);
        key_shifts(&mut picker, (DRILLED, turn.turn), (turn.seed, turn.before))
    }

    fn compare(&self, asked: &Asked<'_>) -> Option<Compare> {
        reading::compare(asked)
    }

    /// A crop's two bands, an arrow, the tendency's disc it shifts toward.
    fn tile(&self, _family: Family) -> Vec<TileMark> {
        let crop = square([7.0, 10.0], [27.0, 30.0]);
        let disc: Vec<[f32; 2]> = (0..12)
            .map(|k| {
                let turn = std::f32::consts::TAU * k as f32 / 12.0;
                [42.0 + 5.0 * turn.cos(), 20.0 + 5.0 * turn.sin()]
            })
            .collect();
        vec![
            TileMark::Tint(
                vec![[7.0, 10.0], [27.0, 10.0], [27.0, 17.0], [7.0, 21.0]],
                0.15,
            ),
            TileMark::Tint(
                vec![[7.0, 21.0], [27.0, 17.0], [27.0, 30.0], [7.0, 30.0]],
                0.45,
            ),
            TileMark::Line(closed(&crop)),
            TileMark::Line(vec![[7.0, 21.0], [27.0, 17.0]]),
            TileMark::Line(vec![[30.0, 20.0], [35.0, 20.0]]),
            TileMark::Line(vec![[33.0, 18.0], [35.0, 20.0], [33.0, 22.0]]),
            TileMark::Tint(disc.clone(), 0.6),
            TileMark::Line(closed(&disc)),
        ]
    }
}
