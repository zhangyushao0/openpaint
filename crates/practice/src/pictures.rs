//! The pictures exercises copy (S80): every shape item of the packs kept, the official packs' by
//! tier and the user's own apart, every composition item (S95), and how a session picks one. A
//! shape is picked by the session's seed among those of its tier not shown in an earlier session,
//! all of them again once each was; a tier with none falls back to the nearest tier that has
//! some, the easier first. The user's own pictures take one place in every stage-2 session, once
//! the session's tiers reach theirs (S83: the tier the user chose for each). A composition is
//! picked alike among those not shown in the last ten sessions, one of those only when none other
//! is left, never one the session holds already while another is left.

use std::collections::HashSet;

use crate::pack::{ItemRef, Pack, PictureKind, Tier};
use crate::random::Random;

/// Every picture a session can pick from.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Pictures {
    /// The official packs' shape items with their tiers, by pack and the pack's order.
    official: Vec<(Tier, ItemRef)>,
    /// The user's own with the tiers chosen for them, oldest first.
    personal: Vec<(Tier, ItemRef)>,
    /// The packs' compositions with their tiers (S95).
    compositions: Vec<(Tier, ItemRef)>,
}

impl Pictures {
    /// The shape and composition pictures of `packs`.
    #[must_use]
    pub fn of(packs: &[Pack]) -> Self {
        let mut pictures = Self::default();
        for pack in packs {
            for item in &pack.items {
                let named = ItemRef {
                    pack: pack.id.clone(),
                    item: item.id.clone(),
                };
                match item.kind {
                    PictureKind::Composition => pictures.compositions.push((item.tier, named)),
                    PictureKind::Shape if pack.personal() => {
                        pictures.personal.push((item.tier, named));
                    }
                    PictureKind::Shape => pictures.official.push((item.tier, named)),
                }
            }
        }
        pictures
    }

    /// Whether there is no shape picture at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.official.is_empty() && self.personal.is_empty()
    }

    /// Whether there is a composition to divide or copy whole.
    #[must_use]
    pub fn has_compositions(&self) -> bool {
        !self.compositions.is_empty()
    }

    /// Whether the user has pictures of their own of `reach` or an easier tier.
    #[must_use]
    pub fn has_personal(&self, reach: Tier) -> bool {
        self.personal.iter().any(|(tier, _)| *tier <= reach)
    }

    /// An official picture of `tier`, or of the nearest tier with any (the easier first), else
    /// one of the user's own; `None` without pictures.
    pub(crate) fn pick(
        &self,
        tier: Tier,
        random: &mut Random,
        used: &HashSet<ItemRef>,
    ) -> Option<(Tier, ItemRef)> {
        nearest(&self.official, tier, |of| chosen(of, random, used))
            .or_else(|| self.pick_personal(Tier::Hard, random, used))
    }

    /// A composition of `tier` or of the nearest tier with any, picked by `random` among those
    /// neither `shown` in the session nor `recent`, then among those not `shown`, then any.
    pub(crate) fn pick_composition(
        &self,
        tier: Tier,
        random: &mut Random,
        (shown, recent): (&HashSet<ItemRef>, &HashSet<ItemRef>),
    ) -> Option<(Tier, ItemRef)> {
        nearest(&self.compositions, tier, |of| {
            let fresh: Vec<&ItemRef> = (of.iter().copied())
                .filter(|item| !shown.contains(*item) && !recent.contains(*item))
                .collect();
            let from = if fresh.is_empty() { of } else { &fresh };
            chosen(from, random, shown)
        })
    }

    /// One of the user's own pictures of `reach` or an easier tier, with its tier.
    pub(crate) fn pick_personal(
        &self,
        reach: Tier,
        random: &mut Random,
        used: &HashSet<ItemRef>,
    ) -> Option<(Tier, ItemRef)> {
        let own: Vec<&ItemRef> = (self.personal.iter())
            .filter(|(tier, _)| *tier <= reach)
            .map(|(_, item)| item)
            .collect();
        let item = chosen(&own, random, used)?;
        let tier = (self.personal.iter()).find_map(|(tier, held)| (*held == item).then_some(*tier));
        tier.map(|tier| (tier, item))
    }
}

/// The first of `items`' tiers nearest `tier` (the easier first) where `choose` picks one of its
/// items, with that tier.
fn nearest(
    items: &[(Tier, ItemRef)],
    tier: Tier,
    mut choose: impl FnMut(&[&ItemRef]) -> Option<ItemRef>,
) -> Option<(Tier, ItemRef)> {
    let mut tiers: Vec<Tier> = Tier::ALL.to_vec();
    tiers.sort_by_key(|t| (t.code().abs_diff(tier.code()), t.code()));
    tiers.into_iter().find_map(|near| {
        let of: Vec<&ItemRef> = (items.iter())
            .filter(|(t, _)| *t == near)
            .map(|(_, item)| item)
            .collect();
        choose(&of).map(|item| (near, item))
    })
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
