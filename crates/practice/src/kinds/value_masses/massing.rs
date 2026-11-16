//! What varies a value-masses exercise (S96, S156): its composition's tier, and whether its masses
//! are painted in three values or in two (Notan: every mass decided light or dark, no middle; at
//! 熟练 and 稳, every second lesson). Stored by its place in `ALL` from 1, so a three-value one keeps
//! its tier's code as before S156 and the two-value ones follow.

use crate::kinds::place;
use crate::packs::pack::Tier;

/// A value-masses exercise's tier and values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Massing {
    /// Three greys, dark, middle and light (V大's 黑白灰, S96).
    Three(Tier),
    /// Two, dark and light (Notan, S156).
    Two(Tier),
}

impl Massing {
    /// Every variant as stored, by its place from 1: the three-value ones in their tiers' order
    /// (their tiers' codes), the two-value ones after them.
    pub const ALL: [Self; 8] = [
        Self::Three(Tier::Entry),
        Self::Three(Tier::Simple),
        Self::Three(Tier::Medium),
        Self::Three(Tier::Hard),
        Self::Two(Tier::Entry),
        Self::Two(Tier::Simple),
        Self::Two(Tier::Medium),
        Self::Two(Tier::Hard),
    ];

    /// Its composition's tier.
    #[must_use]
    pub const fn tier(self) -> Tier {
        match self {
            Self::Three(tier) | Self::Two(tier) => tier,
        }
    }

    /// Whether it is painted in two values (Notan).
    #[must_use]
    pub const fn two(self) -> bool {
        matches!(self, Self::Two(_))
    }

    /// As stored: its place in `ALL` from 1.
    #[must_use]
    pub fn code(self) -> u8 {
        place(&Self::ALL, &self) + 1
    }

    /// The variant stored as `code`; none for a code it never stored.
    #[must_use]
    pub fn of_code(code: u8) -> Option<Self> {
        let at = usize::from(code.checked_sub(1)?);
        Self::ALL.get(at).copied()
    }
}
