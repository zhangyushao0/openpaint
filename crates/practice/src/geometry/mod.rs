//! The course's shared geometry: boxes and squares in perspective seen by one eye, exact ellipses,
//! straight edges and marks read from a hand's lines, and only what is off (S98), on which the
//! geometric kinds (`kinds/`) build their answers and checks.

pub(crate) mod edges;
pub(crate) mod ellipse;
pub(crate) mod marks;
pub(crate) mod off;
pub(crate) mod perspective;
pub(crate) mod perspective_eye;
