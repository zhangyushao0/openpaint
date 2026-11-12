//! What Compare asks of a copy, whatever its kind (S109b, D111): the app reads the copy off its
//! artwork as a kind asks (`Copied`: whether anything lies in its frame, the colours laid, its
//! drawn lines, its division's lines) and hands it with the exercise's original and picture
//! (`Asked`) to the kind's description, which says how it is compared (`Compare`): shown as
//! found, its original fitted onto it for Aligned, checked as geometry, or read from its pixels
//! off the frame loop (`Reading`).

use std::any::Any;
use std::fmt::Debug;
use std::sync::Arc;

use crate::compositions::levels::Level;
use crate::kinds::Original;
use crate::marks::compared::{Comparison, DrawnCopy};
use crate::maths::colour::Rgb8;
use crate::maths::path::Point;
use crate::records::metric::Metrics;
use crate::sheets::exercise::Exercise;
use crate::sheets::sheet_picture::SheetPicture;

/// The copy in an exercise's empty frame as the app reads it off the artwork.
pub trait Copied {
    /// Whether anything was laid in the frame: a paint stroke begun there or a fill.
    fn holds(&self) -> bool;
    /// The colours laid in the frame, in the order they lie.
    fn picks(&self) -> Vec<Rgb8>;
    /// Its drawn lines (the sketch pen's left out), and once compared those before its first
    /// Compare and the corrections after it.
    fn drawn(&self) -> DrawnCopy;
    /// A division's lines drawn with the levels' pens, each with its level (S95).
    fn division(&self) -> Vec<(Level, Vec<Point>)>;
}

/// An exercise asked to be compared: what it gives, its picture as the sheet holds it (none for
/// a kind without one, or one no pack holds now) and its copy.
#[derive(Clone, Copy)]
pub struct Asked<'a> {
    pub exercise: &'a Exercise,
    pub original: &'a Original,
    pub picture: Option<&'a SheetPicture>,
    pub copy: &'a dyn Copied,
}

/// A drawn copy's original to fit onto it for Aligned: lines, or a single shape's outline
/// (`shape`), in frame units.
#[derive(Debug, Clone, PartialEq)]
pub struct ToFit {
    pub original: Vec<Vec<Point>>,
    pub shape: bool,
}

/// How a copy is compared.
#[derive(Debug, Clone, PartialEq)]
pub enum Compare {
    /// Shown as found: a colour's colours, a division's lines, a picture laid over the copy.
    Made(Comparison),
    /// Its original laid over it, fitted onto it when Aligned is chosen (none to fit: a picture
    /// without an outline, in place only); the pen turns blue.
    Fitted(Comparison, Option<ToFit>),
    /// Checked as geometry against its exact answer, a construction to lay over it; the pen
    /// turns blue.
    Checked(Comparison),
    /// Read from its frame's pixels off the frame loop.
    Read(Reading),
}

/// A painted copy as its frame's pixels read: its comparison and the numbers a first Compare keeps.
#[derive(Debug, Clone)]
pub struct Read {
    pub comparison: Comparison,
    pub metrics: Metrics,
}

/// How a painted copy is read, on the compare worker.
pub trait Reads: Any + Debug + Send + Sync {
    /// What shows until the copy is read.
    fn unread(&self) -> Comparison;
    /// The copy whose frame's pixels are `pixels` (8-bit RGBA, `size`, on `paper`), as read.
    fn read(&self, size: [u32; 2], pixels: &[u8], paper: Rgb8) -> Read;
    /// Whether its pair of frames shows in grey on asking once read (a key shift's Grey view,
    /// S107).
    fn greys(&self) -> bool {
        false
    }
}

/// A painted copy's reading, held by the app as it comes: equal only to itself.
#[derive(Debug, Clone)]
pub struct Reading(pub Arc<dyn Reads>);

impl PartialEq for Reading {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Reading {
    /// `reads`, held.
    pub fn of(reads: impl Reads + 'static) -> Self {
        Self(Arc::new(reads))
    }

    /// It as the kind's own reading `T`, if it is one (the tests read it so).
    #[must_use]
    pub fn downcast<T: Reads>(&self) -> Option<&T> {
        let any: &dyn Any = &*self.0;
        any.downcast_ref::<T>()
    }
}

impl std::ops::Deref for Reading {
    type Target = dyn Reads;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

/// A picture the practice crate draws for an exercise (a generated combination's, an object's,
/// value masses' answer in its greys): its size and 8-bit RGBA pixels, straight alpha.
pub type Painted = ([u32; 2], Vec<u8>);

impl Asked<'_> {
    /// Whether anything lies in the copy's frame: most kinds compare nothing before.
    #[must_use]
    pub fn held(&self) -> bool {
        self.copy.holds()
    }

    /// Its picture's corners in its frame (frame units), if the sheet holds its picture.
    #[must_use]
    pub fn corners(&self) -> Option<[Point; 4]> {
        self.picture.map(|picture| picture.fit.corners())
    }
}
