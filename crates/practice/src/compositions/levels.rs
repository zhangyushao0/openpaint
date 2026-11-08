//! A picture's division in V大's three levels (S95): the main lines, then the second and third, each
//! drawn with its pen in his red, green and blue. Shared by a division's check, a whole copy's,
//! the records' numbers per level and the landscapes' known divisions (`masses.rs`).

use crate::maths::colour::Rgb8;

/// A division's levels, in V大's order and colours.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Level {
    /// 主分割: the few most obvious lines, red.
    Main,
    /// 次分割: found from the main lines, green.
    Second,
    /// 三级: shorter still, blue.
    Third,
}

impl Level {
    pub const ALL: [Self; 3] = [Self::Main, Self::Second, Self::Third];

    /// Its pen's colour, as a stroke of it holds it and Compare draws it: V大's red, green, blue.
    #[must_use]
    pub const fn colour(self) -> Rgb8 {
        match self {
            Self::Main => [214, 40, 36],
            Self::Second => [30, 150, 70],
            Self::Third => [40, 100, 220],
        }
    }

    /// Its place in `ALL`.
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::Main => 0,
            Self::Second => 1,
            Self::Third => 2,
        }
    }
}

/// A level's lines as the card counts them: how many, and how far they run on the mean as a share
/// of the picture's longer side (none without a line).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LevelRow {
    pub lines: usize,
    pub length: Option<f64>,
}
