//! The first lesson's drawing (`Kind::Free`, S122, S151): its first exercise, one empty frame in
//! which the learner draws something in front of them, meeting the pen (about half a minute). It
//! has no original (`Blank`: it gives nothing), compares nothing (`Facts::compared` false: no
//! Compare, Next alone) and ends with no result; it is no skill (`Skill::of`), so no level, dot or
//! redo counts it and it lays no lesson. The practice book keeps it as the "before" of the first
//! drawing from life (`Kept::Drawing`, `observation/`). Its one frame is the copy's
//! (constructed), its tile a loop of scribble; it has no How of its own (the first lesson's).

use std::sync::Arc;

use crate::kinds::{Blank, Described, Facts, Kept, Kind, Original, ToolSet, held};
use crate::marks::tile::{TileMark, curve};
use crate::sheets::exercise::Exercise;

pub(crate) struct Free;

const FACTS: [Facts; 1] = [Facts::of("free", ToolSet::DRAWING, 30)
    .constructed()
    .uncompared()
    .kept(Kept::Drawing)];

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
    fn generate(&self, _exercise: &Exercise) -> Arc<dyn Original> {
        held(Blank)
    }

    /// A loop of scribble.
    fn glyph(&self) -> Vec<TileMark> {
        let turn = |t: f32| std::f32::consts::TAU * 2.0 * t;
        vec![TileMark::Line(curve(|t| {
            [
                14.0 + 26.0 * t + 6.0 * turn(t).cos(),
                20.0 + 8.0 * turn(t).sin(),
            ]
        }))]
    }
}
