//! A blind pair (比一比, S147): two earlier copies of one skill shown side by side without their
//! dates, the learner saying which is better. Kept as asked and answered (practice-store's space
//! `b`); its days, levels and kinds are read from the sheets, never stored twice. A pair whose
//! sheet is gone is left out (sheet ids are never reused).
//!
//! Which copy shows on the left is fixed by the pair's own ids (`older_left`), so a card shown
//! again shows the same; the answer is the side, which `better` turns into which copy.

use crate::progress::Better;
use crate::records::lessons::OnSheet;
use crate::records::sheet::{Laid, Sheet};

/// A copy: exercise `exercise` (its index among its sheet's) of sheet `sheet`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CopyRef {
    pub sheet: u64,
    pub exercise: u16,
}

/// What the learner answered of a pair: the left copy better, the two alike, the right one better.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Answer {
    Left,
    Alike,
    Right,
}

/// A blind pair asked and answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlindPair {
    /// When it was answered, seconds since 1970 UTC.
    pub asked: u64,
    /// The lesson whose end asked it: its sheet's id and its index among the sheet's lessons.
    pub lesson: (u64, u16),
    pub older: CopyRef,
    pub newer: CopyRef,
    /// The older copy showed on the left.
    pub older_left: bool,
    pub answer: Answer,
}

impl Answer {
    /// Its stored code: 1 left better, 2 alike, 3 right better.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::Left => 1,
            Self::Alike => 2,
            Self::Right => 3,
        }
    }

    #[must_use]
    pub const fn of_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::Left),
            2 => Some(Self::Alike),
            3 => Some(Self::Right),
            _ => None,
        }
    }
}

impl CopyRef {
    /// The copy as `sheets` hold it, with its sheet.
    #[must_use]
    pub fn find<'a>(&self, sheets: &'a [Sheet]) -> Option<(&'a Sheet, &'a Laid)> {
        let sheet = sheets.iter().find(|sheet| sheet.id == self.sheet)?;
        Some((sheet, sheet.laid(usize::from(self.exercise))?))
    }

    /// Exercise `index` of `on`'s sheet.
    #[must_use]
    pub fn of(on: &OnSheet<'_>, index: usize) -> Self {
        Self {
            sheet: on.sheet.id,
            exercise: u16::try_from(index).unwrap_or(u16::MAX),
        }
    }
}

impl BlindPair {
    /// Which copy the answer called better.
    pub(crate) const fn better(&self) -> Better {
        match (self.answer, self.older_left) {
            (Answer::Alike, _) => Better::Alike,
            (Answer::Left, true) | (Answer::Right, false) => Better::Older,
            (Answer::Left, false) | (Answer::Right, true) => Better::Newer,
        }
    }

    /// Whether `copy` is one of its two.
    #[must_use]
    pub fn holds(&self, copy: CopyRef) -> bool {
        self.older == copy || self.newer == copy
    }
}

/// Whether the older of `older` and `newer` shows on the left: a bit of their ids mixed, the same
/// every time they are shown together.
#[must_use]
pub fn older_left(older: CopyRef, newer: CopyRef) -> bool {
    let mut mixed: u64 = 0x9E37_79B9_7F4A_7C15;
    for part in [
        older.sheet,
        u64::from(older.exercise),
        newer.sheet,
        u64::from(newer.exercise),
    ] {
        mixed ^= part;
        mixed = mixed.wrapping_mul(0xBF58_476D_1CE4_E5B9);
        mixed ^= mixed >> 31;
    }
    mixed & 1 == 0
}
