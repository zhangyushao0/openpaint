//! Stage 4's sessions, 概括 / Simplifying (S94, V大's weeks 3 and 4 assignment: objects
//! simplified into geometric solids by 点定位): a warm-up of three from the kinds met (at most one
//! long, as stage 3's), then three objects (their carves in turn: a block taken out, a cut, a top
//! drawn in, a round part), a solid in its box in turn (a cylinder, a cone, a sphere), a single
//! shape of the medium tier, then of the hard one, in turn (the user's own of that tier or an
//! easier one when they have one) and a colour combination, its arrangement in turn.
//!
//! An object taken at about 4.5 minutes (its box completed and its points found with Sketch, its
//! visible edges with Draw, compared, corrected once), a solid at about 2.5, a medium shape at
//! about 4.5 and a hard one at about 5.5, a combination at about 4: the mix about 24.5 to 25.5
//! minutes, the warm-up about 4.5, so a session about 29 to 30 (about 25 without a pack). Objects
//! and solids, taking minutes each, stay out of the warm-up's quick places.

use crate::areas::Arrangement;
use crate::object_design::Carve;
use crate::pack::Tier;
use crate::plan::Kind;
use crate::session::{Before, Picker, Plan, seeded, warm_up_of};
use crate::session_three::QUICK_THREE;
use crate::solid::Solid;

/// Objects a session holds; its shape's tier in turn.
const OBJECTS: usize = 3;
const TIERS: [Tier; 2] = [Tier::Medium, Tier::Hard];

/// Stage 4's session `number` seeded `seed` after `before`: its warm-up, the objects, the solid,
/// the shape (none without a picture) and the colour combination.
#[must_use]
pub fn stage_four(number: u32, seed: u64, before: &Before<'_>) -> Plan {
    let turn = number.saturating_sub(1) as usize;
    let tier = TIERS[turn % TIERS.len()];
    let mut picker = Picker::new(seed, before, tier);
    let warm_up = picker.warm_up(warm_up_of(number, seed, before.met, &QUICK_THREE));
    let shape = picker.shapes(&[tier]);
    let objects =
        (0..OBJECTS).map(|k| Kind::Object(Carve::ALL[(OBJECTS * turn + k) % Carve::ALL.len()]));
    let solid = Kind::Solid(Solid::ALL[turn % Solid::ALL.len()]);
    let combination = Kind::Areas(Arrangement::ALL[turn % Arrangement::ALL.len()]);
    let warmed = u32::try_from(warm_up.len()).unwrap_or(0);
    let all = (warm_up.into_iter())
        .chain(objects.chain([solid]).map(|kind| (kind, None)))
        .chain(shape)
        .chain([(combination, None)]);
    Plan {
        exercises: seeded(seed, all),
        warm_up: warmed,
    }
}
