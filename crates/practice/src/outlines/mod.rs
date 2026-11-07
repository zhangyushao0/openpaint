//! A picture's silhouette outline (S80, D103, D106): its ground, a watershed, a matte cleaned,
//! the contours traced.

pub(crate) mod outline;
pub(crate) mod outline_flood;
pub(crate) mod outline_ground;
pub(crate) mod outline_mask;
pub(crate) mod outline_matte;
pub(crate) mod outline_mesh;
pub(crate) mod outline_sides;
pub(crate) mod outline_trace;
