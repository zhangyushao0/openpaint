//! A free scribble (`Kind::Free`, S122): the first lesson's first exercise, one empty frame the
//! hand draws in as it likes to meet the pen (about half a minute). It has no original, gives
//! nothing, compares nothing (`Facts::compared` false: no Compare, Next alone) and ends with no
//! result; it is no skill (`Skill::of`), so no level, dot or redo counts it and it lays no lesson.
//! Its one frame is the copy's (constructed), its tile a loop of scribble; it has no How (nothing
//! is copied).

use crate::kinds::{Described, Facts, Kind, Original, Tools};
use crate::marks::tile::{TileMark, curve};
use crate::sheets::exercise::Exercise;
use crate::sheets::family::Family;

pub(crate) struct Free;

const FACTS: [Facts; 1] = [Facts::of(Family::Free, "free", Tools::Drawing, 30)
    .constructed()
    .uncompared()];

impl Described for Free {
    fn code(&self) -> u8 {
        20
    }

    fn facts(&self) -> &'static [Facts] {
        &FACTS
    }

    fn variant(&self, kind: Kind) -> Option<u8> {
        matches!(kind, Kind::Free).then_some(0)
    }

    fn of_variant(&self, variant: u8) -> Option<Kind> {
        (variant == 0).then_some(Kind::Free)
    }

    /// Nothing to copy.
    fn original(&self, _exercise: &Exercise) -> Original {
        Original::Lines(Vec::new())
    }

    /// A loop of scribble.
    fn tile(&self, _family: Family) -> Vec<TileMark> {
        let turn = |t: f32| std::f32::consts::TAU * 2.0 * t;
        vec![TileMark::Line(curve(|t| {
            [
                14.0 + 26.0 * t + 6.0 * turn(t).cos(),
                20.0 + 8.0 * turn(t).sin(),
            ]
        }))]
    }
}
