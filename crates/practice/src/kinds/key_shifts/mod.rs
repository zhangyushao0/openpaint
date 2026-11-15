//! Key shifts (`Kind::KeyShift`, S107, V大's 变调): tendencies, a shift by rule, the check, how a
//! copy is read (`reading.rs`), the How, the tile and the lessons.
//!
//! A lesson holds one key shift (about 6 minutes), a crop none shown before while any is left,
//! its tendency's hue never near the crop's own main hue: the first level a warm tendency, pure
//! and grey in turn (the First-prefix rule); the second and the third every lean in turn
//! (`Lean::ALL`: warm and cool each lesson, pure and grey every second, so no colour habit
//! forms), their crops picked as the first level's.

pub(crate) mod demo;
pub(crate) mod key_shift;
pub(crate) mod key_shift_check;
pub(crate) mod key_shift_rule;
pub(crate) mod marks;
pub(crate) mod reading;

use std::sync::Arc;

use crate::course::picker::{Picker, in_turn};
use crate::demos::demo::Demo;
use crate::kinds::{
    Blank, Described, Facts, Kind, Needs, Original, Picking, SkillLevel, ToolSet, Turn, held,
};
use crate::marks::asked::{Asked, Compare};
use crate::marks::mark::SheetMark;
use crate::marks::tile::{TileMark, closed, square};
use crate::maths::random::{Random, derived};
use crate::packs::pack::{ItemRef, Tier};
use crate::sheets::exercise::Exercise;
use key_shift::{Lean, Tendency};

pub(crate) struct KeyShifts;

const FACTS: [Facts; 1] = [Facts::of("key-shift", ToolSet::PAINTING, 360)
    .picks(Picking::LOCKED)
    .needs(Needs::Crops)];
/// How many key shifts a lesson's place turns through at the first level (as unit 5's drills
/// held them before S122), the leans in turn across them.
const TURNED: usize = 2;

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

    fn generate(&self, exercise: &Exercise) -> Arc<dyn Original> {
        let Kind::KeyShift(tendency) = exercise.kind else {
            return held(Blank);
        };
        held(Shift {
            picture: exercise.item(),
            tendency,
        })
    }

    fn how(&self, _kind: Kind, _cells: usize) -> Option<Demo> {
        Some(demo::key_shift())
    }

    fn lesson(&self, level: SkillLevel, turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        let lean = match level {
            SkillLevel::First => in_turn(&Lean::ALL, turn.turn, TURNED)[0],
            SkillLevel::Second | SkillLevel::Third => Lean::ALL[turn.turn % Lean::ALL.len()],
        };
        key_shift(lean, turn).into_iter().collect()
    }

    fn window(&self) -> usize {
        3
    }

    /// A crop's two bands, an arrow, the tendency's disc it shifts toward.
    fn glyph(&self) -> Vec<TileMark> {
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

/// A key shift's original: the crop it shifts and the tendency it shifts toward.
#[derive(Debug)]
pub(crate) struct Shift {
    picture: ItemRef,
    tendency: Tendency,
}

impl Original for Shift {
    fn givens(&self) -> Vec<SheetMark> {
        marks::givens(self.tendency)
    }

    fn compare(&self, asked: &Asked<'_>) -> Option<Compare> {
        reading::compare(asked)
    }

    fn picture(&self) -> Option<&ItemRef> {
        Some(&self.picture)
    }
}

/// A key shift of `lean` on a crop `turn`'s picker picks, its tendency's hue picked by the seed's
/// own sequence among those away from the crop's main hue; none without crops.
fn key_shift(lean: Lean, turn: &Turn<'_>) -> Option<(Kind, Option<ItemRef>)> {
    let mut random = Random::new(derived(turn.seed, u64::MAX - 3));
    let (_, item) = Picker::new(turn, Tier::Simple).picked_crop()?;
    let main = turn.pictures.crop_hue(&item);
    let tendency = Tendency::chosen(lean, main, &mut random);
    Some((Kind::KeyShift(tendency), Some(item)))
}
