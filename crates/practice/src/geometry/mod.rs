//! The course's shared geometry: boxes and squares in perspective seen by one eye, exact ellipses
//! and the fit of a drawn one, the sides of a frame an ellipse misses, straight edges and marks
//! read from a hand's lines, where an edge heads, and only what is off (S98), on which the
//! geometric kinds (`kinds/`) build their answers and checks.

pub(crate) mod edges;
pub(crate) mod ellipse;
pub(crate) mod ellipse_fit;
pub(crate) mod ellipse_sides;
pub(crate) mod heading;
pub(crate) mod marks;
pub(crate) mod off;
pub(crate) mod perspective;
pub(crate) mod perspective_eye;
