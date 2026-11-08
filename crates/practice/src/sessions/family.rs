//! What an exercise asks for, whatever its variant (`Kind`): its family, as the user meets it (a
//! tile, a How, a line in the summary). What each family is, its key, work and frames, its kind's
//! description says (`Facts`, `kinds/`).

use crate::kinds::{Described, Facts, KINDS, Work};

/// An exercise's family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Family {
    Line,
    Combined,
    Shape,
    Box,
    FreeBoxes,
    Ellipse,
    /// Estimates on the flat: a point, a proportion, an angle.
    Point,
    Proportion,
    Angle,
    /// Points in space, a family each of V大's five tricks: each has its own instructions.
    Centre,
    Halves,
    Thirds,
    Step,
    Mirror,
    /// Solids found in a box, a family each: each has its own instructions.
    Cylinder,
    Cone,
    Sphere,
    /// Objects simplified into boxes (S94).
    Object,
    /// A picture divided in three levels, and one copied whole over its division (S95).
    Division,
    WholeCopy,
    Colour,
    Areas,
    Scale,
    Study,
    /// A picture's value masses painted in three greys (S96).
    ValueMasses,
    /// A crop painted again shifted toward a tendency (S107).
    KeyShift,
}

impl Family {
    /// In a warm-up's order.
    pub const ALL: [Self; 26] = [
        Self::Line,
        Self::Combined,
        Self::Shape,
        Self::Box,
        Self::FreeBoxes,
        Self::Ellipse,
        Self::Point,
        Self::Proportion,
        Self::Angle,
        Self::Centre,
        Self::Halves,
        Self::Thirds,
        Self::Step,
        Self::Mirror,
        Self::Cylinder,
        Self::Cone,
        Self::Sphere,
        Self::Object,
        Self::Division,
        Self::WholeCopy,
        Self::Colour,
        Self::Areas,
        Self::Scale,
        Self::Study,
        Self::ValueMasses,
        Self::KeyShift,
    ];

    /// The description of the kind it belongs to.
    pub(crate) fn described(self) -> &'static dyn Described {
        (KINDS.iter().copied())
            .find(|kind| kind.facts().iter().any(|facts| facts.family == self))
            .unwrap_or(KINDS[0])
    }

    /// What its kind's description says of it.
    #[must_use]
    pub fn facts(self) -> &'static Facts {
        let facts = self.described().facts();
        (facts.iter())
            .find(|facts| facts.family == self)
            .unwrap_or(&facts[0])
    }

    /// The family keyed `key` (`Facts::key`), if any.
    #[must_use]
    pub fn of_key(key: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|family| family.facts().key == key)
    }

    /// Whether its exercises are drawn with the pens as lines (lines, combined lines, single
    /// shapes, boxes, ellipses, estimates, points in space, solids, objects, divisions and whole
    /// copies), not coloured.
    #[must_use]
    pub fn drawn(self) -> bool {
        matches!(
            self.facts().work,
            Work::Lines | Work::Geometry | Work::Divided
        )
    }

    /// Whether its copy is checked as geometry: what it holds found in the lines and set against
    /// an exact answer or rule, never aligned.
    #[must_use]
    pub fn geometric(self) -> bool {
        self.facts().work == Work::Geometry
    }

    /// Whether it is drawn with a division's pens in V大's three colours (S95).
    #[must_use]
    pub fn divided(self) -> bool {
        self.facts().work == Work::Divided
    }

    /// Whether its exercises are painted with brushes and read from the copy's pixels.
    #[must_use]
    pub fn painted(self) -> bool {
        self.facts().work == Work::Painted
    }

    /// Whether its answer is built from given parts in the one frame drawn in (S97, S98).
    #[must_use]
    pub fn constructed(self) -> bool {
        self.facts().constructed
    }

    /// Whether its exercises copy a composition's picture (S95, S96).
    #[must_use]
    pub fn composed(self) -> bool {
        self.facts().composed
    }
}
