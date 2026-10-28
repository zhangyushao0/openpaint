//! Stage 3's sessions (V大's week 1 二维抽象网格 as quick estimates, then week 3's 点定位): a
//! warm-up of three from the kinds met (at most one long, as stage 2's), then six estimates on the
//! flat (a point, a proportion and an angle, twice), three points in space (V大's five tricks in
//! turn by the session's number: the centre, the halves, the thirds, the next face, a mirror),
//! two single shapes of the medium and hard tiers (the last the user's own of those tiers or an
//! easier one when they have one) and a colour combination, its arrangement in turn.
//!
//! An estimate taken at about 0.6 minutes (looked at, marked, compared, corrected once), a point
//! in space at about 2 (constructed with Sketch, marked, compared, corrected), a medium shape at
//! about 4.5 and a hard one at about 5.5, a combination at about 4: the mix about 23.6 minutes,
//! the warm-up about 5, so a session about 28 (about 19 without a pack, which holds no shapes).

use crate::areas::Arrangement;
use crate::flat::Estimate;
use crate::pack::Tier;
use crate::plan::{Family, Kind};
use crate::session::{Before, Picker, Plan, seeded, warm_up_of};
use crate::space::Trick;

/// The quick families a stage-3 warm-up takes in turn: the stage's own first, then stage 1's and
/// stage 2's.
const QUICK_THREE: [Family; 13] = [
    Family::Point,
    Family::Proportion,
    Family::Angle,
    Family::Centre,
    Family::Halves,
    Family::Thirds,
    Family::Step,
    Family::Mirror,
    Family::Line,
    Family::Combined,
    Family::Colour,
    Family::Box,
    Family::Ellipse,
];
/// The estimates in order, each kind twice.
const ESTIMATES: [Estimate; 6] = [
    Estimate::Point,
    Estimate::Proportion,
    Estimate::Angle,
    Estimate::Point,
    Estimate::Proportion,
    Estimate::Angle,
];
/// Points in space a session holds, and its shapes' tiers.
const TRICKS: usize = 3;
const TIERS: [Tier; 2] = [Tier::Medium, Tier::Hard];

/// Stage 3's session `number` seeded `seed` after `before`: its warm-up, the estimates, the
/// points in space, the shapes (none without a picture) and the colour combination.
#[must_use]
pub fn stage_three(number: u32, seed: u64, before: &Before<'_>) -> Plan {
    let mut picker = Picker::new(seed, before, TIERS[TIERS.len() - 1]);
    let warm_up = picker.warm_up(warm_up_of(number, seed, before.met, &QUICK_THREE));
    let shapes = picker.shapes(&TIERS);
    let turn = number.saturating_sub(1) as usize;
    let tricks = (0..TRICKS).map(|k| Trick::ALL[(TRICKS * turn + k) % Trick::ALL.len()]);
    let combination = Kind::Areas(Arrangement::ALL[turn % Arrangement::ALL.len()]);
    let warmed = u32::try_from(warm_up.len()).unwrap_or(0);
    let flat = (ESTIMATES.into_iter().map(Kind::Flat)).chain(tricks.map(Kind::Space));
    let all = (warm_up.into_iter())
        .chain(flat.map(|kind| (kind, None)))
        .chain(shapes)
        .chain([(combination, None)]);
    Plan {
        exercises: seeded(seed, all),
        warm_up: warmed,
    }
}
