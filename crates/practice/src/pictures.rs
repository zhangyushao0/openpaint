//! The pictures single shapes are copied from (S80): every shape item of the packs kept, the
//! official packs' by tier and the user's own apart, and how a session picks one. A picture is
//! picked by the session's seed among those of its tier not shown in an earlier session, all of
//! them again once each was; a tier with none falls back to the nearest tier that has some, the
//! easier first. The user's own pictures take one place in every stage-2 session.

use std::collections::HashSet;

use crate::pack::{ItemRef, Pack, PictureKind, Tier};
use crate::random::Random;

/// Every picture a session can pick from.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Pictures {
    /// The official packs' shape items with their tiers, by pack and the pack's order.
    official: Vec<(Tier, ItemRef)>,
    /// The user's own, oldest first.
    personal: Vec<ItemRef>,
}

impl Pictures {
    /// The shape pictures of `packs`.
    #[must_use]
    pub fn of(packs: &[Pack]) -> Self {
        let mut pictures = Self::default();
        for pack in packs {
            for item in pack.items.iter().filter(|i| i.kind == PictureKind::Shape) {
                let named = ItemRef {
                    pack: pack.id.clone(),
                    item: item.id.clone(),
                };
                if pack.personal() {
                    pictures.personal.push(named);
                } else {
                    pictures.official.push((item.tier, named));
                }
            }
        }
        pictures
    }

    /// Whether there is no picture at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.official.is_empty() && self.personal.is_empty()
    }

    /// Whether the user has pictures of their own.
    #[must_use]
    pub fn has_personal(&self) -> bool {
        !self.personal.is_empty()
    }

    /// An official picture of `tier`, or of the nearest tier with any (the easier first), else
    /// one of the user's own; `None` without pictures.
    pub(crate) fn pick(
        &self,
        tier: Tier,
        random: &mut Random,
        used: &HashSet<ItemRef>,
    ) -> Option<(Tier, ItemRef)> {
        let mut tiers: Vec<Tier> = Tier::ALL.to_vec();
        tiers.sort_by_key(|t| (t.code().abs_diff(tier.code()), t.code()));
        for near in tiers {
            let of: Vec<&ItemRef> = (self.official.iter())
                .filter(|(t, _)| *t == near)
                .map(|(_, item)| item)
                .collect();
            if let Some(item) = chosen(&of, random, used) {
                return Some((near, item));
            }
        }
        self.pick_personal(random, used)
            .map(|item| (crate::pack::PERSONAL_TIER, item))
    }

    /// One of the user's own pictures, else an official one of their tier.
    pub(crate) fn pick_personal(
        &self,
        random: &mut Random,
        used: &HashSet<ItemRef>,
    ) -> Option<ItemRef> {
        let own: Vec<&ItemRef> = self.personal.iter().collect();
        chosen(&own, random, used)
    }
}

/// One of `items` by `random`, among those not `used` while any is not.
fn chosen(items: &[&ItemRef], random: &mut Random, used: &HashSet<ItemRef>) -> Option<ItemRef> {
    let fresh: Vec<&ItemRef> = items
        .iter()
        .copied()
        .filter(|i| !used.contains(*i))
        .collect();
    let from = if fresh.is_empty() { items } else { &fresh };
    if from.is_empty() {
        return None;
    }
    let at = ((random.unit() * from.len() as f64) as usize).min(from.len() - 1);
    Some(from[at].clone())
}
