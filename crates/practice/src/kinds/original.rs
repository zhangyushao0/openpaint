//! What an exercise gives (`Original`, S109): the variants its kinds' generators make, and what the
//! flow reads of any of them without asking which it is (S109b): its lines, its swatch, its
//! picture, its value scale's cells, the colour it gives at a point.

use crate::colour_areas::areas::Areas;
use crate::colour_areas::areas_generated::GeneratedAreas;
use crate::geometry::ellipse::EllipseGuide;
use crate::geometry::perspective::PerspectiveBox;
use crate::kinds::estimates::flat::Flat;
use crate::kinds::objects::object::ObjectTask;
use crate::kinds::points_in_space::space::SpaceTask;
use crate::kinds::scales::scale::ValueScale;
use crate::kinds::solids::solid::SolidTask;
use crate::maths::colour::Rgb8;
use crate::maths::path::Point;
use crate::packs::pack::ItemRef;

/// What an exercise's original is.
#[derive(Debug, Clone, PartialEq)]
pub enum Original {
    /// One line or a group of them, each in frame units.
    Lines(Vec<Vec<Point>>),
    /// A swatch.
    Colour(Rgb8),
    /// A pack's picture (an empty reference: none was kept); a crop's with its areas (S101).
    Picture(ItemRef),
    /// A generated colour combination's picture and areas (a record of 1.18.1 and before).
    Areas(GeneratedAreas),
    /// A value scale's ends and cells.
    Scale(ValueScale),
    /// A box to complete: the exact box, its front corner's three edges given.
    Box(PerspectiveBox),
    /// A free-boxes page: its horizon is the example box's.
    FreeBoxes(PerspectiveBox),
    /// An ellipse's guide and the exact ellipse.
    Ellipse(EllipseGuide),
    /// An estimate's givens: its answer follows from them.
    Flat(Flat),
    /// A box with a face to fix a point in space on.
    Space(Box<SpaceTask>),
    /// A box with a solid to find in it.
    Solid(Box<SolidTask>),
    /// An object to simplify into its box.
    Object(Box<ObjectTask>),
}

impl Original {
    /// A value scale's count of cells; none for another original.
    #[must_use]
    pub fn cells(&self) -> Option<usize> {
        self.scale().map(|scale| scale.cells)
    }

    /// A value scale's ends and cells, which Fill cell and the pen take one at a time; none for
    /// another original.
    #[must_use]
    pub const fn scale(&self) -> Option<&ValueScale> {
        match self {
            Self::Scale(scale) => Some(scale),
            _ => None,
        }
    }

    /// Its lines, each in frame units (a line's, a group's); none for another original.
    #[must_use]
    pub fn lines(&self) -> Option<&[Vec<Point>]> {
        match self {
            Self::Lines(lines) => Some(lines),
            _ => None,
        }
    }

    /// Its swatch (a colour's); none for another original.
    #[must_use]
    pub const fn swatch(&self) -> Option<Rgb8> {
        match self {
            Self::Colour(colour) => Some(*colour),
            _ => None,
        }
    }

    /// The pack's picture it copies (an empty reference: none was kept); none for an original
    /// made from its seed.
    #[must_use]
    pub const fn picture(&self) -> Option<&ItemRef> {
        match self {
            Self::Picture(item) => Some(item),
            _ => None,
        }
    }

    /// The colour areas it was made with and the share of its frame they fill (a generated
    /// combination's, its square frame whole); none for another original, whose areas, if any,
    /// its pack's picture holds.
    #[must_use]
    pub fn areas(&self) -> Option<(&Areas, Point)> {
        match self {
            Self::Areas(generated) => Some((&generated.areas, [1.0, 1.0])),
            _ => None,
        }
    }

    /// The colour it gives at the frame point `at`: a generated combination's there, or in a value
    /// scale's frame its given end's where `at` lies in one; none elsewhere and for another
    /// original (a pack's picture's colour is its pixels').
    #[must_use]
    pub fn colour_at(&self, at: Point) -> Option<Rgb8> {
        match self {
            Self::Areas(generated) => Some(generated.colour_at(at, 0.0)),
            Self::Scale(scale) => {
                let end = (scale.cell_at(at)).filter(|cell| *cell == 0 || cell + 1 == scale.cells);
                end.map(|cell| scale.ends()[usize::from(cell > 0)])
            }
            _ => None,
        }
    }
}
