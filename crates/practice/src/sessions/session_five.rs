//! Stage 5's sessions, 变调 / Key shifts (S107, V大's weeks 5 and 6): a warm-up of three from the
//! kinds met (at most one long, as stages 3 and 4 take it), then three key shifts, each a crop of
//! the packs (none twice while others are left, as combinations pick them) with its tendency, the
//! leans in turn across sessions (`Lean::ALL`: the warmth every exercise, the purity every second)
//! and each hue never near the crop's own main hue. Abstract analysis (S108) and structure
//! rotation (S109) join the mix after the key shifts. Without a crop there is no key shift: the
//! practice options refuse the stage's row until a pack holding crops is kept.
//!
//! A key shift taken at about 6 minutes (the tendency looked at, the base laid, the greys, the
//! opposites and the lightest each against it, compared, an area repainted): the mix about 18
//! minutes, the warm-up about 5, a session about 23 until abstract analysis and structure
//! rotation join it.

use crate::kinds::key_shifts::key_shift::{Lean, Tendency};
use crate::maths::random::{Random, derived};
use crate::packs::pack::{ItemRef, Tier};
use crate::sessions::plan::Kind;
use crate::sessions::session::{Before, Picker, Plan, seeded, warm_up_of};
use crate::sessions::session_three::QUICK_THREE;

/// Key shifts a session holds.
const KEY_SHIFTS: usize = 3;

/// Stage 5's session `number` seeded `seed` after `before`: its warm-up, then three key shifts
/// whose leans go on from the sessions before (none without a crop).
#[must_use]
pub fn stage_five(number: u32, seed: u64, before: &Before<'_>) -> Plan {
    let turn = number.saturating_sub(1) as usize;
    let mut picker = Picker::new(seed, before, Tier::Hard);
    let warm_up = picker.warm_up(warm_up_of(number, seed, before.met, &QUICK_THREE));
    let warmed = u32::try_from(warm_up.len()).unwrap_or(0);
    let shifts = key_shifts(&mut picker, (KEY_SHIFTS, turn), (seed, before));
    Plan {
        exercises: seeded(seed, warm_up.into_iter().chain(shifts), before.scales),
        warm_up: warmed,
    }
}

/// `count` key shifts, the `turn`th sheet's of their kind (the leans in turn from its place on),
/// their crops picked by `picker` and their tendencies' hues by the seed's own sequence among
/// those away from each crop's main hue; none without crops.
pub(crate) fn key_shifts(
    picker: &mut Picker<'_>,
    (count, turn): (usize, usize),
    (seed, before): (u64, &Before<'_>),
) -> Vec<(Kind, Option<ItemRef>)> {
    let mut random = Random::new(derived(seed, u64::MAX - 3));
    (0..count)
        .filter_map(|k| {
            let lean = Lean::ALL[(count * turn + k) % Lean::ALL.len()];
            let (_, item) = picker.picked_crop()?;
            let main = before.pictures.crop_hue(&item);
            let tendency = Tendency::chosen(lean, main, &mut random);
            Some((Kind::KeyShift(tendency), Some(item)))
        })
        .collect()
}
