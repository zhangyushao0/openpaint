//! What an exercise gives (`Original`, S109, S123): the value its kind's generator makes from its
//! seed (or its picture), a type of the kind's own folder behind one trait. It says what it prints
//! in its frames, how its copy is compared, the picture the crate draws for it and its answer as a
//! hand draws it; and it answers what the flow reads of any original without asking which it is:
//! its lines, its swatch, its picture, its value scale's cells, the colour it gives at a point.
//! See `Described` (`kind.rs`) for why a kind is two halves of one description.

use std::any::Any;
use std::fmt::Debug;
use std::sync::Arc;

use crate::colour_areas::areas::Areas;
use crate::kinds::scales::scale::ValueScale;
use crate::marks::asked::{Asked, Compare, Painted};
use crate::marks::card::CardItem;
use crate::marks::compared::{Compared, Comparison, DrawnCopy};
use crate::marks::mark::SheetMark;
use crate::maths::colour::Rgb8;
use crate::maths::path::Point;
use crate::packs::pack::ItemRef;
use crate::sheets::sheet_picture::SheetPicture;

/// An exercise's original: its kind's folder's own type, made by its kind's description
/// (`Described::generate`). Every method has a default: a kind says only what it gives.
pub trait Original: Any + Debug + Send + Sync {
    /// What it gives in its exercise's frames before Compare; a pack's
    /// picture is the sheet's to lay.
    fn givens(&self) -> Vec<SheetMark> {
        Vec::new()
    }
    /// What it prints in its frames before Compare, its pack's `picture` in hand (none: none
    /// held): its `givens`, but where they are drawn from the picture (a contrast thumbnail's
    /// shapes and rings, S154).
    fn printed(&self, _picture: Option<&SheetPicture>) -> Vec<SheetMark> {
        self.givens()
    }
    /// What the card shows while the copy is made, before Compare: the steps it asks and the
    /// choices it takes (abstract analysis's, S112); nothing for most kinds.
    fn task(&self) -> Vec<CardItem> {
        Vec::new()
    }
    /// Its copy, drawn as `copy`, checked as geometry (S84 to S98); none for a kind not checked
    /// so.
    fn check(&self, _copy: &DrawnCopy) -> Option<Arc<dyn Compared>> {
        None
    }
    /// How `asked`'s copy is compared (`asked.original` is it); none while there is nothing to
    /// compare (S109b). By default its drawn lines, once anything lies in its frame, checked as
    /// geometry.
    fn compare(&self, asked: &Asked<'_>) -> Option<Compare> {
        if !asked.held() {
            return None;
        }
        let checked = self.check(&asked.copy.drawn())?;
        Some(Compare::Checked(Comparison(checked)))
    }
    /// The picture the practice crate draws for it (`picture` its pack's, if any): a generated
    /// combination's, an object's, value masses' answer; none for most kinds.
    fn painted(&self, _picture: Option<&SheetPicture>) -> Option<Painted> {
        None
    }
    /// Its answer as a hand would draw it, a stroke at a time in frame units (a cross on a point,
    /// a stroke along each line, once round each ellipse): what `bench practice` draws a little
    /// off; none for a kind without one.
    fn answer(&self) -> Vec<Vec<Point>> {
        Vec::new()
    }
    /// Its lines, each in frame units (a line's, a group's); none for another original.
    fn lines(&self) -> Option<&[Vec<Point>]> {
        None
    }
    /// Its swatch (a colour's); none for another original.
    fn swatch(&self) -> Option<Rgb8> {
        None
    }
    /// The pack's picture it copies (an empty reference: none was kept); none for an original
    /// made from its seed.
    fn picture(&self) -> Option<&ItemRef> {
        None
    }
    /// The colour areas it was made with and the share of its frame they fill (a generated
    /// combination's, its square frame whole); none for another original, whose areas, if any,
    /// its pack's picture holds.
    fn areas(&self) -> Option<(&Areas, Point)> {
        None
    }
    /// A value scale's ends and cells, which Fill cell and the pen take one at a time; none for
    /// another original.
    fn scale(&self) -> Option<&ValueScale> {
        None
    }
    /// The colour it gives at the frame point `at`: a generated combination's there, or in a value
    /// scale's frame its given end's where `at` lies in one; none elsewhere and for another
    /// original (a pack's picture's colour is its pixels').
    fn colour_at(&self, _at: Point) -> Option<Rgb8> {
        None
    }
}

impl dyn Original {
    /// A value scale's count of cells; none for another original.
    #[must_use]
    pub fn cells(&self) -> Option<usize> {
        self.scale().map(|scale| scale.cells)
    }

    /// It as its kind's own type `T`, if it is one (the tests read a generator's exact task so).
    #[must_use]
    pub fn downcast<T: Original>(&self) -> Option<&T> {
        let any: &dyn Any = self;
        any.downcast_ref::<T>()
    }
}

/// An original held by the flow is equal only to itself, as it was made.
impl PartialEq for dyn Original {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self, other)
    }
}

/// What a kind's generator gives for an exercise not of its kind, which its description is never
/// asked for: nothing.
#[derive(Debug)]
pub(crate) struct Blank;

impl Original for Blank {}

/// `original`, held as the flow holds it.
pub(crate) fn held(original: impl Original) -> Arc<dyn Original> {
    Arc::new(original)
}
