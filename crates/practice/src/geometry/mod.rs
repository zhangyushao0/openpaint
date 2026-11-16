//! The course's shared geometry: boxes and squares in perspective seen by one eye, exact ellipses,
//! straight edges and marks read where one stroke is asked, where an edge heads, and only what is
//! off (S98), on which the geometric kinds (`kinds/`) build their answers (D123: a drawn copy's
//! free strokes are never read against an answer).

pub(crate) mod edges;
pub(crate) mod ellipse;
pub(crate) mod heading;
pub(crate) mod marks;
pub(crate) mod off;
pub(crate) mod perspective;
pub(crate) mod perspective_eye;
