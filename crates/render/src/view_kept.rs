//! The view tiers the GPU memory budget holds (S58, DECISIONS D85): a tier the view leaves is
//! kept whole for a return to its scale; the budget lets tiers go by recency, the least recently
//! shown first, never the tier shown now; leaving the app lets every tier go.

use crate::view_atlas::Tier;
use crate::view_tier::ViewCache;

/// One tier held, for the budget: its scale, its bytes, when the screen last showed it (the
/// frame clock's seconds) and whether it is the tier shown now.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeldTier {
    pub scale: u32,
    pub bytes: u64,
    pub shown_at: f64,
    pub shown: bool,
}

impl ViewCache {
    /// The previous tier, if any, kept whole among the tiers the view left.
    pub(crate) fn keep_previous(&mut self) {
        if let Some(previous) = self.previous.take() {
            self.kept.push(previous);
        }
    }

    /// Lets the least recently shown tier go but the one shown: a kept one, else the previous.
    /// Returns its bytes, 0 when only the tier shown is left.
    pub(crate) fn let_go_oldest(&mut self) -> u64 {
        let oldest = (self.kept.iter().enumerate())
            .min_by(|a, b| a.1.shown_at.total_cmp(&b.1.shown_at))
            .map(|(at, _)| at);
        match oldest {
            Some(at) => self.kept.swap_remove(at).bytes(),
            None => self.previous.take().as_ref().map_or(0, Tier::bytes),
        }
    }

    /// The tiers hold at most `bytes` from now on (S49): when they hold more, the least recently
    /// shown go, never the tier shown; a new tier must fit what is left. Returns the bytes given
    /// back.
    pub fn set_room(&mut self, bytes: u64) -> u64 {
        self.room = bytes;
        let mut freed = 0;
        while self.bytes() > self.room {
            let gone = self.let_go_oldest();
            if gone == 0 {
                break;
            }
            freed += gone;
        }
        freed
    }

    /// Lets the tier of `scale` go unless it is the one shown now (the budget's step, S58).
    /// Returns its bytes.
    pub fn let_go(&mut self, scale: u32) -> u64 {
        if let Some(at) = self.kept.iter().position(|tier| tier.scale() == scale) {
            return self.kept.swap_remove(at).bytes();
        }
        if self
            .previous
            .as_ref()
            .is_some_and(|tier| tier.scale() == scale)
        {
            return self.previous.take().as_ref().map_or(0, Tier::bytes);
        }
        0
    }

    /// Every tier held, the one shown first.
    #[must_use]
    pub fn held(&self) -> Vec<HeldTier> {
        let held = |tier: &Tier, shown| HeldTier {
            scale: tier.scale(),
            bytes: tier.bytes(),
            shown_at: tier.shown_at,
            shown,
        };
        let current = self.current.iter().map(|tier| held(tier, true));
        let others = self.previous.iter().chain(&self.kept);
        current
            .chain(others.map(|tier| held(tier, false)))
            .collect()
    }

    /// Lets every tier go, the one shown too (the app left the screen, S58: Android's
    /// `onTrimMemory`); the view makes them again when it shows. Returns their bytes.
    pub fn release(&mut self) -> u64 {
        let bytes = self.bytes();
        self.current = None;
        self.previous = None;
        self.kept.clear();
        bytes
    }
}
