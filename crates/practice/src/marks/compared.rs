//! An exercise as Compare found it, whatever its kind (S109, D111): each kind's check or reading
//! implements `Compared`, giving the marks over the pair and the card's items for what the user
//! set (`Seen`), the numbers a first Compare keeps, and its verdict (S119: `Verdict`, the app's
//! `Outcome` where an exact answer judges, the eye's where only it can, or none yet while the copy
//! is incomplete). The app holds it as it comes and asks it each frame; the screen draws what it
//! gives.

use std::fmt::Debug;

use crate::fits::align::Fit;
use crate::marks::card::CardItem;
use crate::marks::mark::SheetMark;
use crate::maths::colour::Rgb8;
use crate::maths::path::Point;
use crate::records::metric::Metrics;

/// What the user set over a comparison.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Seen<'a> {
    /// The way to see a geometric answer shows (S97, S98).
    pub construction: bool,
    /// How strongly the original shows over the copy, 0 to 1.
    pub shown: f32,
    /// The area the card compares and its outline (frame units).
    pub chosen: usize,
    pub outline: &'a [Vec<Point>],
    /// The frames show in grey (S107).
    pub grey: bool,
    /// The view of the card's views shown (`CardItem::Views`, S112).
    pub view: usize,
    /// The fit the original lies by over the copy (`Fit::SAME` in place); none while the aligned
    /// one is being found.
    pub fit: Option<Fit>,
    /// The point the eyedropper compared: the original's colour there and the copy's.
    pub probe: Option<(Rgb8, Rgb8)>,
}

/// A drawn copy's lines in its frame (frame units, the sketch pen's left out), read only where one
/// stroke is asked (an estimate's, a point in space's; D123).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DrawnCopy {
    pub all: Vec<Vec<Point>>,
}

/// An exercise compared, held by the app as it comes: equal only to itself.
#[derive(Debug, Clone)]
pub struct Comparison(pub std::sync::Arc<dyn Compared>);

impl PartialEq for Comparison {
    fn eq(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Comparison {
    /// `compared`, held.
    pub fn of(compared: impl Compared + 'static) -> Self {
        Self(std::sync::Arc::new(compared))
    }
}

impl std::ops::Deref for Comparison {
    type Target = dyn Compared;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

/// What an exercise ends with (S119): copied clean, near, or to do again.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Outcome {
    /// Compare marks nothing (a colour or an area within what the eye barely notices).
    Clean,
    /// Off a little: one part marked, or within the kind's near band.
    Near,
    /// Off more: draw or paint it again.
    Redo,
}

impl Outcome {
    /// Its stored code, frozen (S120): 1 clean, 2 near, 3 redo.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::Clean => 1,
            Self::Near => 2,
            Self::Redo => 3,
        }
    }

    /// The outcome stored as `code`; none for a code no outcome holds.
    #[must_use]
    pub const fn of_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::Clean),
            2 => Some(Self::Near),
            3 => Some(Self::Redo),
            _ => None,
        }
    }

    /// Of a copy off by `off`: clean at most `clean`, near at most `near`, else redo.
    #[must_use]
    pub fn within(off: f64, [clean, near]: [f64; 2]) -> Self {
        if off <= clean {
            Self::Clean
        } else if off <= near {
            Self::Near
        } else {
            Self::Redo
        }
    }

    /// Of a copy Compare marked `marked` times: clean at none, near at most `near`, else redo.
    #[must_use]
    pub const fn counting(marked: usize, near: usize) -> Self {
        if marked == 0 {
            Self::Clean
        } else if marked <= near {
            Self::Near
        } else {
            Self::Redo
        }
    }
}

/// Who judges a comparison (S119): the app where the kind holds an exact answer, the user's eye
/// where only the eye can (the overlay is the feedback, D99), or nobody yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Verdict {
    /// The app judged the copy against its answer.
    App(Outcome),
    /// The user chooses the outcome: lines, shapes, free boxes, divisions, whole and colour copies,
    /// a painted kind whose answer no pack holds now.
    Eye,
    /// No verdict is honest yet: the copy does not cover what the exercise asks, or its pixels
    /// are still being read.
    Incomplete,
    /// No result at all, asked or kept: a drawing from life, which the eye looks at against its
    /// photo (S151).
    Nobody,
}

/// An exercise compared.
pub trait Compared: std::any::Any + Debug + Send + Sync {
    /// What lies over the pair, in order.
    fn marks(&self, seen: &Seen<'_>) -> Vec<SheetMark>;
    /// What the card shows under the title.
    fn card(&self, seen: &Seen<'_>) -> Vec<CardItem>;
    /// The numbers a first Compare keeps.
    fn metrics(&self) -> Metrics {
        Metrics::new()
    }
    /// Who judges it and, where the app does, its outcome (S119); every kind says, none by
    /// default.
    fn verdict(&self) -> Verdict;
    /// The area its card opens chosen on once read, outlined on both frames (S139): the one its
    /// reason names, findable without counting chips; none (the area chosen stays) by default.
    fn named_area(&self) -> Option<usize> {
        None
    }
}

impl Comparison {
    /// It as the kind's own comparison `T`, if it is one (the tests read a kind's check so).
    #[must_use]
    pub fn downcast<T: Compared>(&self) -> Option<&T> {
        let any: &dyn std::any::Any = &*self.0;
        any.downcast_ref::<T>()
    }
}
