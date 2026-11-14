//! How a lesson lays its exercises (S122): its variants in turn by the lesson's place among its
//! skill's (`in_turn`, `turned`), and its pictures picked by the seed (`Picker`): shapes others
//! than those shown before while there are, the user's own where asked; compositions others than
//! the last ten lessons' while there are, none twice in a lesson while another is left (S95);
//! crops none shown before while any is left (S101). The pictures themselves are the packs'
//! (`packs/pictures.rs`).

use std::collections::HashSet;

use crate::kinds::Kind;
use crate::kinds::{SkillLevel, Turn};
use crate::maths::random::{Random, derived};
use crate::packs::pack::{ItemRef, Tier};

/// `count` of `variants` from the `turn`th lesson's place on, round and round.
pub(crate) fn in_turn<T: Copy>(variants: &[T], turn: usize, count: usize) -> Vec<T> {
    (0..count)
        .map(|k| variants[(count * turn + k) % variants.len()])
        .collect()
}

/// `count` of `variants` from the `turn`th of them on, round and round: the same few each lesson,
/// starting one further on.
pub(crate) fn rotated<T: Copy>(variants: &[T], turn: usize, count: usize) -> Vec<T> {
    (0..count)
        .map(|k| variants[(turn + k) % variants.len()])
        .collect()
}

/// `count` kinds made by `kind` of `variants` in turn, no pictures.
pub(crate) fn turned<T: Copy>(
    (variants, count): (&[T], usize),
    turn: &Turn<'_>,
    kind: impl Fn(T) -> Kind,
) -> Vec<(Kind, Option<ItemRef>)> {
    (in_turn(variants, turn.turn, count).into_iter())
        .map(|variant| (kind(variant), None))
        .collect()
}

/// `kinds` (each with no picture yet) laid as they are.
pub(crate) fn laid(kinds: impl IntoIterator<Item = Kind>) -> Vec<(Kind, Option<ItemRef>)> {
    kinds.into_iter().map(|kind| (kind, None)).collect()
}

/// The tiers stage 4's drills of compositions took in turn.
const COMPOSITION_TIERS: [Tier; 2] = [Tier::Simple, Tier::Medium];

/// A lesson of one composition for the kind `kind` makes (a division, a whole copy, value
/// masses), picked by `turn`'s seed (value masses' among those whose masses read), at `level`: the
/// first the simple and medium tiers in turn as the kind's drills of `drilled` took them (its
/// first), the second a medium picture, the third a hard one (the nearest tier the packs hold
/// where they hold none of it); none without such a composition.
pub(crate) fn composition(
    (level, drilled): (SkillLevel, usize),
    turn: &Turn<'_>,
    kind: fn(Tier) -> Kind,
) -> Vec<(Kind, Option<ItemRef>)> {
    let tier = match level {
        SkillLevel::First => in_turn(&COMPOSITION_TIERS, turn.turn, drilled)[0],
        SkillLevel::Second => Tier::Medium,
        SkillLevel::Third => Tier::Hard,
    };
    Picker::new(turn, Tier::Medium).compositions(&[kind(tier)])
}

/// A lesson's pictures, picked by its seed.
pub(crate) struct Picker<'a> {
    turn: &'a Turn<'a>,
    random: Random,
    /// The crops' own sequence (S101), so the shapes and compositions pick as before.
    crops: Random,
    used: HashSet<ItemRef>,
    /// The hardest tier the user's own pictures may be of.
    reach: Tier,
    /// The lesson's own pictures so far, and those of the last ten lessons.
    shown: HashSet<ItemRef>,
    recent: HashSet<ItemRef>,
}

impl<'a> Picker<'a> {
    pub(crate) fn new(turn: &'a Turn<'a>, reach: Tier) -> Self {
        Self {
            turn,
            random: Random::new(derived(turn.seed, u64::MAX - 1)),
            crops: Random::new(derived(turn.seed, u64::MAX - 2)),
            used: turn.used.iter().cloned().collect(),
            reach,
            shown: HashSet::new(),
            recent: turn.recent.iter().cloned().collect(),
        }
    }

    /// `kinds` (divisions, whole copies and value masses) with compositions of their tiers (value
    /// masses' among those whose masses read); none without such a composition.
    pub(crate) fn compositions(&mut self, kinds: &[Kind]) -> Vec<(Kind, Option<ItemRef>)> {
        let mut picked = Vec::new();
        for kind in kinds {
            let (Kind::Division(tier) | Kind::WholeCopy(tier) | Kind::ValueMasses(tier)) = *kind
            else {
                continue;
            };
            let massed = matches!(kind, Kind::ValueMasses(_));
            let shown = (&self.shown, &self.recent);
            let pictures = self.turn.pictures;
            let Some((tier, item)) =
                pictures.pick_composition((tier, massed), &mut self.random, shown)
            else {
                continue;
            };
            self.shown.insert(item.clone());
            let kind = match kind {
                Kind::Division(_) => Kind::Division(tier),
                Kind::ValueMasses(_) => Kind::ValueMasses(tier),
                _ => Kind::WholeCopy(tier),
            };
            picked.push((kind, Some(item)));
        }
        picked
    }

    /// A colour combination with its crop (S101).
    pub(crate) fn crop(&mut self) -> Option<(Kind, Option<ItemRef>)> {
        let (tier, item) = self.picked_crop()?;
        Some((Kind::Crop(tier), Some(item)))
    }

    /// A crop with its tier (a combination's, a key shift's): one no earlier lesson showed while
    /// any is left; none without crops.
    pub(crate) fn picked_crop(&mut self) -> Option<(Tier, ItemRef)> {
        let (tier, item) = self.turn.pictures.pick_crop(&mut self.crops, &self.used)?;
        self.used.insert(item.clone());
        Some((tier, item))
    }

    /// A shape of `tier` with its picture, the user's own of the reach or an easier tier when
    /// `own` and they have one; none when no picture is left.
    pub(crate) fn shape(&mut self, tier: Tier, own: bool) -> Option<(Kind, Option<ItemRef>)> {
        let pictures = self.turn.pictures;
        let personal =
            own.then(|| pictures.pick_personal(self.reach, &mut self.random, &self.used));
        let (tier, item) = match personal.flatten() {
            Some(found) => found,
            None => pictures.pick(tier, &mut self.random, &self.used)?,
        };
        self.used.insert(item.clone());
        Some((Kind::Shape(tier), Some(item)))
    }

    /// Shapes of `tiers` with their pictures, the last the user's own when they have one of the
    /// reach or an easier tier.
    pub(crate) fn shapes(&mut self, tiers: &[Tier]) -> Vec<(Kind, Option<ItemRef>)> {
        let own = self.turn.pictures.has_personal(self.reach);
        (tiers.iter().enumerate())
            .filter_map(|(at, tier)| self.shape(*tier, own && at + 1 == tiers.len()))
            .collect()
    }

    /// Whether the user has pictures of their own of the reach or an easier tier.
    pub(crate) fn has_own(&self) -> bool {
        self.turn.pictures.has_personal(self.reach)
    }
}
