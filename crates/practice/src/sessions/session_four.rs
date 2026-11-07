//! Stage 4's sessions, 概括 / Simplifying (S94, V大's weeks 3 and 4 assignment: objects
//! simplified into geometric solids by 点定位; S95, its 画面分割 and 整图复制; S96, its colour
//! blocks grouped by value): a warm-up of three from the kinds met (at most one long, as stage
//! 3's), then three objects (their carves in turn: a block taken out, a cut, a top drawn in, a
//! round part), a division and then value masses (in the first, third, fifth session, where a
//! picture's masses read) or a whole copy (in the others) of the composition pack's pictures, the
//! simple and the medium tier in turn (one of each, the division's first in odd sessions), none
//! twice in a session and none of the last ten sessions' while others are left. Without a
//! composition, S94's mix: the objects, a solid in its box in turn (a cylinder, a cone, a sphere),
//! a single shape of the medium tier, then of the hard one, in turn (the user's own of that tier
//! or an easier one when they have one) and a colour combination of a crop (S101; none without
//! one).
//!
//! An object taken at about 4.5 minutes (its box completed and its points found with Sketch, its
//! visible edges with Draw, compared, corrected once), a division at about 5 (half of it on its
//! few main lines, V大's rule, then the second and third levels, compared), a whole copy at about
//! 8 (its division, then the line drawing or the colour blocks over it at low completion, compared
//! under the slider), value masses at about 6 (squinted at, the dark masses, the light, the middle,
//! compared, a mass repainted): the mix about 24.5 to 26.5 minutes, the warm-up about 4.5, so a
//! session about 29 to 31. S94's mix: a solid at about 2.5, a medium shape at about 4.5 and a hard
//! one at about 5.5, a combination at about 4, a session about 29 to 30 (about 25 without a pack).
//! Objects, solids, divisions, whole copies and value masses, taking minutes each, stay out of the
//! warm-up's quick places.

use crate::kinds::objects::object_design::Carve;
use crate::kinds::solids::solid::Solid;
use crate::packs::pack::Tier;
use crate::sessions::plan::Kind;
use crate::sessions::session::{Before, Picker, Plan, seeded, warm_up_of};
use crate::sessions::session_three::QUICK_THREE;

/// Objects a session holds; its shape's tier in turn; its compositions' tiers in turn.
const OBJECTS: usize = 3;
const TIERS: [Tier; 2] = [Tier::Medium, Tier::Hard];
pub(crate) const COMPOSITION_TIERS: [Tier; 2] = [Tier::Simple, Tier::Medium];

/// Stage 4's session `number` seeded `seed` after `before`: its warm-up and the objects, then a
/// division and value masses or a whole copy in turn where a composition is kept, else the solid,
/// the shape (none without a picture) and the colour combination.
#[must_use]
pub fn stage_four(number: u32, seed: u64, before: &Before<'_>) -> Plan {
    let turn = number.saturating_sub(1) as usize;
    let tier = TIERS[turn % TIERS.len()];
    let mut picker = Picker::new(seed, before, tier);
    let warm_up = picker.warm_up(warm_up_of(number, seed, before.met, &QUICK_THREE));
    let objects =
        (0..OBJECTS).map(|k| Kind::Object(Carve::ALL[(OBJECTS * turn + k) % Carve::ALL.len()]));
    let objects = objects.map(|kind| (kind, None));
    let warmed = u32::try_from(warm_up.len()).unwrap_or(0);
    let rest = if before.pictures.has_compositions() {
        let [first, second] = [turn, turn + 1].map(|k| COMPOSITION_TIERS[k % 2]);
        // Value masses in the first, third, fifth session where a picture's masses read (S96),
        // else a whole copy.
        let masses = turn.is_multiple_of(2) && before.pictures.has_masses();
        let last = if masses {
            Kind::ValueMasses(second)
        } else {
            Kind::WholeCopy(second)
        };
        picker.compositions(&[Kind::Division(first), last])
    } else {
        let shape = picker.shapes(&[tier]);
        let solid = Kind::Solid(Solid::ALL[turn % Solid::ALL.len()]);
        let combination = picker.crop();
        (std::iter::once((solid, None)))
            .chain(shape)
            .chain(combination)
            .collect()
    };
    let all = (warm_up.into_iter()).chain(objects).chain(rest);
    Plan {
        exercises: seeded(seed, all, before.scales),
        warm_up: warmed,
    }
}
