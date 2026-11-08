//! An exercise as Compare found it, whatever its kind (S109, D111): each kind's check or reading
//! implements `Compared`, giving the marks over the pair and the card's items for what the user
//! set (`Seen`), and the numbers a first Compare keeps. The app holds it as it comes and asks it
//! each frame; the screen draws what it gives.

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
    /// The fit the original lies by over the copy (`Fit::SAME` in place); none while the aligned
    /// one is being found.
    pub fit: Option<Fit>,
    /// The point the eyedropper compared: the original's colour there and the copy's.
    pub probe: Option<(Rgb8, Rgb8)>,
}

/// Lines, each a run of points in frame units.
pub type Lines = Vec<Vec<Point>>;

/// A drawn copy's lines in its frame (frame units, the sketch pen's left out): every one, and,
/// once it was compared, those before its first Compare and the corrections after it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DrawnCopy {
    pub all: Vec<Vec<Point>>,
    pub corrected: Option<(Lines, Lines)>,
}

impl DrawnCopy {
    /// Its lines before its first Compare and the corrections after it (every line and none
    /// before one), as lines to read.
    pub(crate) fn split(&self) -> (Vec<&[Point]>, Vec<&[Point]>) {
        match &self.corrected {
            Some((first, corrections)) => (read(first), read(corrections)),
            None => (read(&self.all), Vec::new()),
        }
    }
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
}

/// `pieces` as lines to read.
fn read(pieces: &[Vec<Point>]) -> Vec<&[Point]> {
    pieces.iter().map(Vec::as_slice).collect()
}

impl Comparison {
    /// It as the kind's own comparison `T`, if it is one (the tests read a kind's check so).
    #[must_use]
    pub fn downcast<T: Compared>(&self) -> Option<&T> {
        let any: &dyn std::any::Any = &*self.0;
        any.downcast_ref::<T>()
    }
}
