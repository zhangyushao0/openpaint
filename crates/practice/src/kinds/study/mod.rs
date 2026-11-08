//! Long colour copies (`Kind::Study`, V大's 色彩临摹, S83): a chosen picture copied in colour beside
//! a large frame in the full interface, a project of its own beside the sessions (a record of stage
//! `PROJECT`), never one of their exercises nor drilled; no How of its own (its card names V大's
//! four brushes, S100).

use crate::kinds::{Described, Facts, Kind, Original, Work};
use crate::marks::mark::SheetMark;
use crate::sessions::family::Family;
use crate::sessions::plan::Exercise;

pub(crate) struct Study;

const FACTS: [Facts; 1] = [Facts::of(Family::Study, "study", Work::Painted)];

impl Described for Study {
    fn code(&self) -> u8 {
        7
    }

    fn facts(&self) -> &'static [Facts] {
        &FACTS
    }

    fn variant(&self, kind: Kind) -> Option<u8> {
        matches!(kind, Kind::Study).then_some(0)
    }

    fn of_variant(&self, variant: u8) -> Option<Kind> {
        (variant == 0).then_some(Kind::Study)
    }

    fn givens(&self, _exercise: &Exercise, _original: &Original) -> Vec<SheetMark> {
        crate::marks::drawing::pictured()
    }

    fn original(&self, exercise: &Exercise) -> Original {
        exercise.picture()
    }
}
