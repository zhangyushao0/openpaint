//! Key shifts (`Kind::KeyShift`, S107, V大's 变调): tendencies, a shift by rule, the check, the How.

pub(crate) mod demo_key;
pub(crate) mod key_shift;
pub(crate) mod key_shift_check;
pub(crate) mod key_shift_rule;
pub(crate) mod marks;

use crate::demos::demo::Demo;
use crate::kinds::{Described, Facts, Kind, Original, Turn, Work};
use crate::marks::mark::SheetMark;
use crate::packs::pack::{ItemRef, Tier};
use crate::sessions::family::Family;
use crate::sessions::plan::Exercise;
use crate::sessions::session::Picker;
use crate::sessions::session_five::key_shifts;
use key_shift::Tendency;

pub(crate) struct KeyShifts;

const FACTS: [Facts; 1] = [Facts::of(Family::KeyShift, "key-shift", Work::Painted)];
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
        Some(demo_key::key_shift())
    }

    /// A drill: two crops, their leans in turn.
    fn drill(&self, _family: Family, turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        let mut picker = Picker::new(turn.seed, turn.before, Tier::Simple);
        key_shifts(&mut picker, (DRILLED, turn.turn), (turn.seed, turn.before))
    }
}
