//! A kind's tile picture (S91, S109b): a line drawing of a fixed example of a skill in a small
//! frame, `TILE` units wide and high, made in its kind's folder (`Described::glyph`) rather than by
//! its generator (at this size a generated original reads as noise, a shape's needs a pack's
//! picture) and drawn by the screen without knowing the kind. In single precision, as the screen
//! draws it. The shapes several kinds share are here: a frame's corners, a curve, the box every
//! perspective tile draws, an ellipse's arc, a picture's edge.

/// A tile's frame, in its picture's units.
pub const TILE: [f32; 2] = [54.0, 40.0];

/// What a tile's picture is made of, in `TILE`'s units, drawn in the tile's ink or pencil.
#[derive(Debug, Clone, PartialEq)]
pub enum TileMark {
    Line(Vec<[f32; 2]>),
    /// A convex area tinted by a share of the colour.
    Tint(Vec<[f32; 2]>, f32),
    /// A convex area of paper over what lies under it (a light mass, S96).
    Paper(Vec<[f32; 2]>),
    Dot([f32; 2]),
    Ring([f32; 2]),
}

/// The line through `points`.
pub(crate) fn line(points: &[[f32; 2]]) -> TileMark {
    TileMark::Line(points.to_vec())
}

/// The corners of the rectangle from `[left, top]` to `[right, bottom]`, clockwise.
pub(crate) const fn square([left, top]: [f32; 2], [right, bottom]: [f32; 2]) -> [[f32; 2]; 4] {
    [[left, top], [right, top], [right, bottom], [left, bottom]]
}

/// `at` sampled along t from 0 to 1.
pub(crate) fn curve(at: impl Fn(f32) -> [f32; 2]) -> Vec<[f32; 2]> {
    (0..=24).map(|step| at(step as f32 / 24.0)).collect()
}

/// `points` with the first again at the end.
pub(crate) fn closed(points: &[[f32; 2]]) -> Vec<[f32; 2]> {
    points.iter().chain(points.first()).copied().collect()
}

/// The arc of the ellipse about `centre` with `axes` from `from` to `to` degrees (y down).
pub(crate) fn arc(centre: [f32; 2], axes: [f32; 2], (from, to): (f32, f32)) -> TileMark {
    TileMark::Line(curve(|t| {
        let turn = (from + (to - from) * t).to_radians();
        [
            centre[0] + axes[0] * turn.cos(),
            centre[1] + axes[1] * turn.sin(),
        ]
    }))
}

/// The box every perspective tile draws, moved by `shift` and scaled by `scale` about its middle:
/// seen from above, its front corner nearest, nine edges.
pub(crate) fn boxed(shift: [f32; 2], scale: f32) -> Vec<TileMark> {
    let middle = [27.0, 20.0];
    let place = |[x, y]: [f32; 2]| {
        [
            middle[0] + (x - middle[0]) * scale + shift[0],
            middle[1] + (y - middle[1]) * scale + shift[1],
        ]
    };
    let [front_top, front_bottom, left_top, left_bottom] =
        [[27.0, 16.0], [27.0, 33.0], [12.0, 12.0], [12.0, 27.0]].map(place);
    let [right_top, right_bottom, back_top] = [[43.0, 11.0], [43.0, 26.0], [28.0, 6.0]].map(place);
    [
        [front_top, front_bottom],
        [left_top, left_bottom],
        [right_top, right_bottom],
        [front_top, left_top],
        [front_top, right_top],
        [left_top, back_top],
        [right_top, back_top],
        [front_bottom, left_bottom],
        [front_bottom, right_bottom],
    ]
    .map(|edge| TileMark::Line(edge.to_vec()))
    .to_vec()
}

/// The box's front face on the left, which the tricks' tiles shade: top back, top front, bottom
/// front, bottom back.
pub(crate) const FACE: [[f32; 2]; 4] = [[12.0, 12.0], [27.0, 16.0], [27.0, 33.0], [12.0, 27.0]];

/// A picture's edge in the tile: the compositions' tiles draw within it.
pub(crate) const EDGE: [[f32; 2]; 2] = [[9.0, 8.0], [45.0, 32.0]];

/// A colour square, tinted and outlined: a colour's tile, a colour copy's.
pub(crate) fn swatch() -> Vec<TileMark> {
    let swatch = square([16.0, 9.0], [38.0, 31.0]);
    vec![
        TileMark::Tint(swatch.to_vec(), 0.35),
        line(&closed(&swatch)),
    ]
}

/// A picture's edge as a closed line.
pub(crate) fn edge() -> TileMark {
    TileMark::Line(closed(&square(EDGE[0], EDGE[1])))
}
