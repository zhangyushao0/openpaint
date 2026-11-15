//! A free-boxes page (S84, S142): where its horizon lies and which way its eye looks, and where a
//! box goes against the eye's level (a page's example, a solid's or an object's box, a point in
//! space's).

use crate::maths::random::Random;

/// Where a free-boxes page's horizon lies (V大: a low horizon sees the boxes from below, a high
/// one from above), and on a three-point page (S142) the eye looking down from a high horizon or
/// up from a low one, its third point given by two faint guides or found from the boxes drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Horizon {
    Low,
    Middle,
    High,
    HighDown,
    LowUp,
    HighDownBare,
    LowUpBare,
}

/// Which way a page's eye looks: level (its boxes two-point), down or up (three-point).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Look {
    Level,
    Down,
    Up,
}

impl Horizon {
    pub const ALL: [Self; 7] = [
        Self::Low,
        Self::Middle,
        Self::High,
        Self::HighDown,
        Self::LowUp,
        Self::HighDownBare,
        Self::LowUpBare,
    ];

    /// Its height in the frame, down from the top.
    #[must_use]
    pub const fn height(self) -> f64 {
        match self {
            Self::Low | Self::LowUp | Self::LowUpBare => 0.72,
            Self::Middle => 0.5,
            Self::High | Self::HighDown | Self::HighDownBare => 0.28,
        }
    }

    /// Which way the page's eye looks.
    #[must_use]
    pub const fn look(self) -> Look {
        match self {
            Self::Low | Self::Middle | Self::High => Look::Level,
            Self::HighDown | Self::HighDownBare => Look::Down,
            Self::LowUp | Self::LowUpBare => Look::Up,
        }
    }

    /// Whether the page gives its third point by two faint guides.
    #[must_use]
    pub const fn guided(self) -> bool {
        matches!(self, Self::HighDown | Self::LowUp)
    }
}

/// Where a box lies against the eye's level: below it (its top seen), across it, above it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Place {
    Below,
    Across,
    Above,
}

/// Where a box goes against the eye's level, from `roll` (0 to 1) and the page's horizon.
pub(crate) fn place(roll: f64, page: Option<Horizon>) -> Place {
    let [below, across] = match page {
        None => [0.45, 0.75],
        Some(Horizon::High | Horizon::HighDown | Horizon::HighDownBare) => [0.7, 1.0],
        Some(Horizon::Low | Horizon::LowUp | Horizon::LowUpBare) => [0.0, 0.3],
        Some(Horizon::Middle) => [0.4, 0.6],
    };
    if roll < below {
        Place::Below
    } else if roll < across {
        Place::Across
    } else {
        Place::Above
    }
}

/// A box's middle's height against the eye, for a box `height` high placed so.
pub(crate) fn level(place: Place, height: f64, random: &mut Random) -> f64 {
    match place {
        Place::Below => -(height / 2.0 + random.range(0.1, 1.0)),
        Place::Across => random.range(-0.3, 0.3) * height,
        Place::Above => height / 2.0 + random.range(0.1, 0.8),
    }
}
