//! What an exercise asks for, whatever its variant (`plan.rs`'s `Kind`): its instructions, its
//! tools, its line in the summary.

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

    /// Whether its exercises are drawn with the pens as lines (lines, combined lines, single
    /// shapes, boxes, ellipses, estimates, points in space, solids, objects, divisions and whole
    /// copies), not coloured.
    #[must_use]
    pub const fn drawn(self) -> bool {
        !self.painted() && !matches!(self, Self::Colour)
    }

    /// Whether its copy is checked as geometry (boxes in perspective, ellipses, estimates, points
    /// in space, solids, objects): what it holds found in the lines and set against an exact
    /// answer or rule, never aligned.
    #[must_use]
    pub const fn geometric(self) -> bool {
        self.drawn()
            && !matches!(self, Self::Line | Self::Combined | Self::Shape)
            && !self.divided()
    }

    /// Whether it is drawn with a division's pens in V大's three colours (a division, a whole
    /// copy's division): no answer, the picture laid under the lines (S95).
    #[must_use]
    pub const fn divided(self) -> bool {
        matches!(self, Self::Division | Self::WholeCopy)
    }

    /// Whether its exercises are painted with brushes and read from the copy's pixels (colour
    /// combinations, value scales, colour copies, value masses, key shifts).
    #[must_use]
    pub const fn painted(self) -> bool {
        matches!(
            self,
            Self::Areas | Self::Scale | Self::Study | Self::ValueMasses | Self::KeyShift
        )
    }

    /// Whether its answer is built from given parts in the one frame drawn in (S97): a value
    /// scale, a box, free boxes, an ellipse; V大's five tricks and the three solids (S98). The
    /// original's frame beside it is left empty, room for what Compare draws beyond the frame (the
    /// sheet keeps its place, so strokes stay put). An estimate and an object keep their original
    /// beside them: it is copied by eye.
    #[must_use]
    pub const fn constructed(self) -> bool {
        matches!(
            self,
            Self::Scale
                | Self::Box
                | Self::FreeBoxes
                | Self::Ellipse
                | Self::Centre
                | Self::Halves
                | Self::Thirds
                | Self::Step
                | Self::Mirror
                | Self::Cylinder
                | Self::Cone
                | Self::Sphere
        )
    }

    /// Whether its exercises copy a composition's picture (S95, S96): a division, a whole copy,
    /// value masses.
    #[must_use]
    pub const fn composed(self) -> bool {
        matches!(self, Self::Division | Self::WholeCopy | Self::ValueMasses)
    }
}
