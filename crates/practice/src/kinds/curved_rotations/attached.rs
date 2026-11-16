//! What stands on a curved object's side (S154f, V大's weeks 7 and 8: an attachment made as a
//! solid of its own whose vanishing points are the main box's; a handle's profile drawn on a face
//! and pulled through along the lines to the vanishing point): a handle's bars as objects' hulls,
//! its edges, the lines where its bars meet the body, and, for V大's construction, the box each
//! stands in from the main box's side, a spout's sections where it leaves the side and at its tip.

use crate::geometry::ellipse::{EllipseGuide, square_on};
use crate::kinds::curved_rotations::answer::Line;
use crate::kinds::curved_rotations::body::Frame;
use crate::kinds::curved_rotations::design::{Attachment, BAR, DEEP, Design};
use crate::kinds::objects::object_body::{Hull, Placed};
use crate::kinds::objects::object_design::Block;
use crate::kinds::objects::object_hull::hull;
use crate::maths::path::Point;
use crate::maths::vector::{Vector, add, norm, sub, times};

/// A handle's planes are numbered from here, planes in one place one surface.
const BARS: u32 = 500;

/// A handle's three bars as hulls: from the axis out on the side `-x` at the shares `along` of
/// the length, their outer face `reach` beyond the box's side.
pub(crate) fn handle_bars(frame: &Frame, [low, high]: [f64; 2], reach: f64) -> Vec<Hull> {
    let out = times(frame.side, -1.0);
    let reach = (0.5 + reach) * frame.width;
    let (bar, deep) = (BAR * frame.width, DEEP * frame.width);
    let [low, high] = [low, high].map(|s| s * frame.length);
    // Each bar: from, to across the axis; from, to along it.
    let blocks = [
        ([0.0, reach], [high - bar, high]),
        ([reach - bar, reach], [low + bar, high - bar]),
        ([0.0, reach], [low, low + bar]),
    ];
    let mut places: Vec<(Vector, f64)> = Vec::new();
    (blocks.iter())
        .filter_map(|([x0, x1], [s0, s1])| {
            let origin = add(
                add(frame.foot, times(frame.axis, *s0)),
                add(times(out, *x0), times(frame.across, -deep / 2.0)),
            );
            let axes = [
                times(out, x1 - x0),
                times(frame.axis, s1 - s0),
                times(frame.across, deep),
            ];
            hull(&Block::WHOLE, &[], Placed { origin, axes })
        })
        .map(|mut piece| {
            piece.surfaces = (piece.planes.iter())
                .map(|plane| {
                    let same = |(normal, offset): &(Vector, f64)| {
                        norm(sub(*normal, plane.normal)) <= 1e-9
                            && (offset - plane.offset).abs() <= 1e-9
                    };
                    let at = places.iter().position(same).unwrap_or_else(|| {
                        places.push((plane.normal, plane.offset));
                        places.len() - 1
                    });
                    BARS + at as u32
                })
                .collect();
            piece
        })
        .collect()
}

/// A handle's lines on the side `-x`, its bars' foot and top at the shares `along` of the length,
/// its outer face `reach` beyond the box's side: its profile's edges on both its faces (but those
/// along the axis, inside the body), the edges pulled through at its corners, and where each bar
/// meets the body.
pub(crate) fn handle_lines(frame: &Frame, [low, high]: [f64; 2], reach: f64) -> Vec<Line> {
    let thick = BAR * frame.width / frame.length;
    let (outer, inner) = (-(0.5 + reach), -(0.5 + reach) + BAR);
    // The profile, in the body's terms along the axis and towards the side.
    let profile = [
        [high, 0.0],
        [high, outer],
        [low, outer],
        [low, 0.0],
        [low + thick, 0.0],
        [low + thick, inner],
        [high - thick, inner],
        [high - thick, 0.0],
    ];
    let at = |[s, x]: [f64; 2], y: f64| frame.at([s, x, y]);
    let half = DEEP / 2.0;
    let mut lines = Vec::new();
    for (a, b) in [(0, 1), (1, 2), (2, 3), (4, 5), (5, 6), (6, 7)] {
        for y in [-half, half] {
            lines.push(Line::Straight([at(profile[a], y), at(profile[b], y)]));
        }
    }
    for corner in [1, 2, 5, 6] {
        lines.push(Line::Straight([
            at(profile[corner], -half),
            at(profile[corner], half),
        ]));
    }
    for [s0, s1] in [[high - thick, high], [low, low + thick]] {
        let corners = [[s0, -half], [s1, -half], [s1, half], [s0, half]]
            .map(|[s, y]| frame.at([s, outer, y]));
        let inward: Vector = sub(frame.at([s0, 0.0, 0.0]), frame.at([s0, outer, 0.0]));
        lines.push(Line::Bar { corners, inward });
    }
    lines
}

/// The boxes what stands on `design`'s side stands in, from the main box's side out, their edges
/// seen through `at` (the body's point `[s, x, y]` to the frame); a spout's sections where it
/// leaves the side and at its tip.
pub(crate) fn boxed(
    design: &Design,
    at: impl Fn([f64; 3]) -> Point,
) -> (Vec<[Point; 2]>, Vec<EllipseGuide>) {
    let across = |half: f64| [half / design.proportion, half];
    let (low, high, guides) = match design.attachment {
        None => return (Vec::new(), Vec::new()),
        Some(Attachment::Spout { along, wide, reach }) => {
            let [s, y] = across(wide[0] / 2.0);
            let section = |x: f64, half: f64| {
                let [ds, dy] = across(half);
                square_on(
                    [[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]]
                        .map(|[i, j]| at([along + i * ds, x, j * dy])),
                )
            };
            let guides = vec![
                section(0.5, wide[0] / 2.0),
                section(0.5 + reach, wide[1] / 2.0),
            ];
            ([along - s, 0.5, -y], [along + s, 0.5 + reach, y], guides)
        }
        Some(Attachment::Handle { along, reach }) => {
            let half = DEEP / 2.0;
            (
                [along[0], -(0.5 + reach), -half],
                [along[1], -0.5, half],
                Vec::new(),
            )
        }
    };
    let corner = |k: usize| {
        let pick = |bit: usize, axis: usize| if k & bit == 0 { low[axis] } else { high[axis] };
        at([pick(1, 0), pick(2, 1), pick(4, 2)])
    };
    let edges = (0..8)
        .flat_map(|k| {
            [1, 2, 4]
                .into_iter()
                .filter(move |bit| k & bit == 0)
                .map(move |bit| (k, k | bit))
        })
        .map(|(a, b)| [corner(a), corner(b)])
        .collect();
    (edges, guides)
}
