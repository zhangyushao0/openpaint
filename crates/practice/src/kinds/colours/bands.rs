//! A colour's bands (D108): how far off a colour may be, in Oklab, before its verdict and its
//! reasons say so; colours and colour combinations cut their results by them, the colour table
//! scales its bars from the just-noticeable step.

/// The Oklab difference an error falls to nothing at: five just-noticeable steps.
pub const DELTA_SPAN: f64 = 0.10;
/// The Oklab difference that is just noticeable.
pub const NOTICEABLE: f64 = 0.02;
/// The Oklab lightness, chroma or hue part an error falls to nothing at: two and a half
/// just-noticeable steps in that part alone.
pub const PART_SPAN: f64 = 0.05;
