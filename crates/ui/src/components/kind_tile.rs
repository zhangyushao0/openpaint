//! A kind's tile (S91): a small framed picture of an exercise kind with its name under it, for
//! the practice options' drills. The picture is a line drawing of a fixed example of the kind (a
//! line, two joined lines, a colour square, a shape's outline, a combination's bands, a scale's
//! cells, a box, boxes on a horizon, an ellipse in a square, a dot in a frame, a segment, a
//! slanted line, a box with its face's diagonals, halving line, two uprights, next face or a
//! dot, a cylinder, a cone, a sphere: `kind_tile_solids.rs`), drawn here rather than by the
//! generators: at this size a generated original reads as
//! noise, and a shape's needs a pack's picture. A kind compared at least once is drawn and named
//! in ink, one never compared in pencil, as the road draws what is done and what lies ahead; a
//! drill left unfinished is ringed in ink, as Next is on the road. No ticks, counts or words
//! beyond the name. Pressed, a faint wash fills round the frame. Named by its English
//! (`text.rs`).

use egui::{Color32, Pos2, Rect, Sense, Shape, Stroke, TextStyle, Vec2, pos2};
use openpaint_practice::Family;

use crate::components::controls;
use crate::design::{INK, Outline, PENCIL, SPACE, ink_line, paint_pressed};
use crate::text::Words;

/// A tile's frame, in points; its picture is drawn in as many units.
pub const TILE: Vec2 = Vec2::new(54.0, 40.0);
/// The weights of the frame, the drawing and the ring round an unfinished drill, in points.
const FRAME_LINE: f32 = 1.0;
const DRAWING_LINE: f32 = 1.25;
const RING_LINE: f32 = 1.5;
/// How far the ring keeps off the frame, in points: a ring round it, never a second frame.
const RING_GAP: f32 = 6.0;
/// A dot's radius, in the picture's units.
const DOT: f32 = 1.8;

#[path = "kind_tile_solids.rs"]
mod solids;

/// What a tile shows.
#[derive(Debug, Clone, Copy)]
pub struct KindTile<'a> {
    pub family: Family,
    /// Its name, under the frame.
    pub name: Words<'a>,
    /// Compared at least once: ink, else pencil.
    pub inked: bool,
    /// A drill of it left unfinished: ringed.
    pub ringed: bool,
}

/// A tile `width` points wide (its frame centred, its name wrapped under it); its response.
pub fn kind_tile(ui: &mut egui::Ui, tile: &KindTile<'_>, width: f32) -> egui::Response {
    let colour = if tile.inked { INK } else { PENCIL };
    let font = TextStyle::Small.resolve(ui.style());
    let shown = tile.name.shown(ui.ctx()).to_owned();
    // Broken between words only: a word longer than the cell may reach past it a little.
    let widest = (shown.split(' '))
        .map(|word| {
            let galley = ui
                .painter()
                .layout_no_wrap(word.to_owned(), font.clone(), colour);
            galley.size().x
        })
        .fold(0.0, f32::max);
    let wrap = (width - SPACE / 2.0).max(widest + 1.0);
    let galley = ui.painter().layout(shown, font, colour, wrap);
    let height = TILE.y + 2.0 * SPACE + galley.size().y;
    let (cell, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());
    controls::register(ui, tile.name.id(), cell);
    if !ui.is_rect_visible(cell) {
        return response;
    }
    let top = pos2(cell.center().x - TILE.x / 2.0, cell.top() + SPACE / 2.0);
    let frame = Rect::from_min_size(top, TILE);
    let painter = ui.painter();
    if response.is_pointer_button_down_on() {
        let wash = Outline {
            rect: frame.expand(SPACE / 2.0),
            corner: SPACE / 2.0,
        };
        paint_pressed(painter, wash);
    }
    let frame_line = Stroke::new(FRAME_LINE, colour);
    painter.rect_stroke(frame, 2.0, frame_line, egui::StrokeKind::Inside);
    paint_kind(painter, frame, tile.family, colour);
    if tile.ringed {
        let ring = Stroke::new(RING_LINE, INK);
        painter.rect_stroke(
            frame.expand(RING_GAP),
            2.0 * RING_GAP,
            ring,
            egui::StrokeKind::Middle,
        );
    }
    let name_top = pos2(
        cell.center().x - galley.size().x / 2.0,
        frame.bottom() + SPACE,
    );
    painter.galley(name_top, galley, colour);
    response
}

/// What a tile's picture is made of, in `TILE`'s units.
#[derive(Debug, Clone, PartialEq)]
enum Mark {
    Line(Vec<[f32; 2]>),
    /// A convex area tinted by a share of the colour.
    Tint(Vec<[f32; 2]>, f32),
    Dot([f32; 2]),
    Ring([f32; 2]),
}

/// The picture of `family` in `frame` in `colour`.
pub fn paint_kind(painter: &egui::Painter, frame: Rect, family: Family, colour: Color32) {
    let scale = frame.width() / TILE.x;
    let at = |[x, y]: [f32; 2]| frame.min + Vec2::new(x, y) * scale;
    let line = Stroke::new(DRAWING_LINE, colour);
    for mark in sketch(family) {
        match mark {
            Mark::Line(points) => {
                let points: Vec<Pos2> = points.into_iter().map(at).collect();
                ink_line(painter, &points, line);
            }
            Mark::Tint(corners, share) => {
                let points = corners.into_iter().map(at).collect();
                let fill = colour.gamma_multiply(share);
                painter.add(Shape::convex_polygon(points, fill, Stroke::NONE));
            }
            Mark::Dot(centre) => {
                painter.circle_filled(at(centre), DOT * scale, colour);
            }
            Mark::Ring(centre) => {
                painter.circle_stroke(at(centre), 1.3 * DOT * scale, line);
            }
        }
    }
}

/// The fixed example of `family` as marks.
fn sketch(family: Family) -> Vec<Mark> {
    let line = |points: &[[f32; 2]]| Mark::Line(points.to_vec());
    match family {
        Family::Line => vec![Mark::Line(curve(|t| {
            [10.0 + 34.0 * t, 30.0 - 19.0 * t + 4.0 * wave(t)]
        }))],
        Family::Combined => vec![
            Mark::Line(curve(|t| [9.0 + 15.0 * t, 30.0 - 19.0 * t - 2.0 * bow(t)])),
            Mark::Line(curve(|t| [24.0 + 21.0 * t, 11.0 + 16.0 * t - 2.5 * bow(t)])),
        ],
        Family::Colour | Family::Study => {
            let swatch = square([16.0, 9.0], [38.0, 31.0]);
            vec![Mark::Tint(swatch.to_vec(), 0.35), line(&closed(&swatch))]
        }
        Family::Shape => vec![line(&closed(&JUG))],
        Family::Areas => bands(),
        Family::Scale => cells(),
        Family::Box => boxed([0.0, 0.0], 1.0),
        Family::FreeBoxes => {
            let mut marks = vec![line(&[[4.0, 12.0], [50.0, 12.0]])];
            marks.extend(boxed([-12.0, 6.0], 0.45));
            marks.extend(boxed([12.0, 5.0], 0.4));
            marks
        }
        Family::Ellipse => vec![
            line(&closed(&[
                [12.0, 31.0],
                [42.0, 31.0],
                [36.0, 11.0],
                [18.0, 11.0],
            ])),
            Mark::Line(curve(|t| {
                let turn = std::f32::consts::TAU * t;
                [27.0 + 12.6 * turn.cos(), 21.5 + 8.6 * turn.sin()]
            })),
        ],
        Family::Point => vec![
            line(&closed(&square([14.0, 7.0], [40.0, 33.0]))),
            Mark::Dot([31.0, 15.0]),
        ],
        Family::Proportion => vec![
            line(&[[10.0, 28.0], [45.0, 14.0]]),
            Mark::Ring([10.0, 28.0]),
            line(&[[44.0, 11.5], [46.0, 16.5]]),
            Mark::Dot([23.3, 22.7]),
        ],
        Family::Angle => vec![line(&[[14.0, 33.0], [40.0, 7.0]]), Mark::Dot([27.0, 20.0])],
        Family::Centre | Family::Halves | Family::Thirds | Family::Step | Family::Mirror => {
            let mut marks = vec![Mark::Tint(FACE.to_vec(), 0.18)];
            marks.extend(boxed([0.0, 0.0], 1.0));
            marks.extend(trick(family));
            marks
        }
        Family::Cylinder | Family::Cone | Family::Sphere => solids::solid(family),
    }
}

/// The corners of the rectangle from `[left, top]` to `[right, bottom]`, clockwise.
fn square([left, top]: [f32; 2], [right, bottom]: [f32; 2]) -> [[f32; 2]; 4] {
    [[left, top], [right, top], [right, bottom], [left, bottom]]
}

/// A value scale's five cells, darker to the right.
fn cells() -> Vec<Mark> {
    (0..5)
        .flat_map(|cell| {
            let left = 8.0 + 7.6 * cell as f32;
            let corners = square([left, 14.0], [left + 7.6, 26.0]);
            let share = 0.08 + 0.18 * cell as f32;
            [
                Mark::Tint(corners.to_vec(), share),
                Mark::Line(closed(&corners)),
            ]
        })
        .collect()
}

/// A V大 trick's marks on the box's shaded face.
fn trick(family: Family) -> Vec<Mark> {
    let [top_back, top_front, bottom_front, bottom_back] = FACE;
    let on =
        |a: [f32; 2], b: [f32; 2], t: f32| [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
    match family {
        Family::Centre => vec![
            Mark::Line(vec![top_back, bottom_front]),
            Mark::Line(vec![top_front, bottom_back]),
            Mark::Dot([20.0, 22.2]),
        ],
        Family::Halves => vec![Mark::Line(vec![
            on(top_back, bottom_back, 0.5),
            on(top_front, bottom_front, 0.5),
        ])],
        Family::Thirds => [0.38, 0.68]
            .map(|t| {
                Mark::Line(vec![
                    on(top_back, top_front, t),
                    on(bottom_back, bottom_front, t),
                ])
            })
            .to_vec(),
        Family::Step => vec![Mark::Line(vec![
            top_back,
            [4.0, 9.9],
            [4.0, 23.8],
            bottom_back,
        ])],
        _ => vec![Mark::Dot([16.5, 19.0])],
    }
}

/// The box every perspective tile draws, moved by `shift` and scaled by `scale` about its middle:
/// seen from above, its front corner nearest, nine edges.
fn boxed(shift: [f32; 2], scale: f32) -> Vec<Mark> {
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
    .map(|edge| Mark::Line(edge.to_vec()))
    .to_vec()
}

/// A combination's three bands in a frame, darker downwards.
fn bands() -> Vec<Mark> {
    let (left, right) = (9.0, 45.0);
    let edges = [(8.0, 8.0), (15.0, 19.0), (25.0, 22.0), (32.0, 32.0)];
    let mut marks: Vec<Mark> = (edges.windows(2).enumerate())
        .map(|(band, pair)| {
            let ((a, b), (c, d)) = (pair[0], pair[1]);
            let quad = vec![[left, a], [right, b], [right, d], [left, c]];
            Mark::Tint(quad, 0.12 + 0.2 * band as f32)
        })
        .collect();
    let outline = [[left, 8.0], [right, 8.0], [right, 32.0], [left, 32.0]];
    marks.push(Mark::Line(closed(&outline)));
    for (a, b) in &edges[1..3] {
        marks.push(Mark::Line(vec![[left, *a], [right, *b]]));
    }
    marks
}

/// The box's front face on the left, which the tricks' tiles shade: top back, top front, bottom
/// front, bottom back.
const FACE: [[f32; 2]; 4] = [[12.0, 12.0], [27.0, 16.0], [27.0, 33.0], [12.0, 27.0]];
/// A jug's outline, the shape's tile.
const JUG: [[f32; 2]; 10] = [
    [22.0, 8.0],
    [32.0, 8.0],
    [31.0, 12.0],
    [37.0, 17.0],
    [38.0, 25.0],
    [34.0, 32.0],
    [20.0, 32.0],
    [16.0, 25.0],
    [17.0, 17.0],
    [23.0, 12.0],
];

/// `at` sampled along t from 0 to 1.
fn curve(at: impl Fn(f32) -> [f32; 2]) -> Vec<[f32; 2]> {
    (0..=24).map(|step| at(step as f32 / 24.0)).collect()
}

/// `points` with the first again at the end.
fn closed(points: &[[f32; 2]]) -> Vec<[f32; 2]> {
    points.iter().chain(points.first()).copied().collect()
}

/// A full wave and a bow over t from 0 to 1, each 0 at both ends.
fn wave(t: f32) -> f32 {
    (std::f32::consts::TAU * t).sin()
}

fn bow(t: f32) -> f32 {
    (std::f32::consts::PI * t).sin()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_s_picture_has_a_line_and_lies_inside_its_frame() {
        let inside = |[x, y]: [f32; 2]| {
            (2.0..=TILE.x - 2.0).contains(&x) && (2.0..=TILE.y - 2.0).contains(&y)
        };
        for family in Family::ALL {
            let marks = sketch(family);
            assert!(
                marks.iter().any(|m| matches!(m, Mark::Line(_))),
                "{family:?}"
            );
            for mark in &marks {
                let points = match mark {
                    Mark::Line(points) | Mark::Tint(points, _) => points.clone(),
                    Mark::Dot(at) | Mark::Ring(at) => vec![*at],
                };
                assert!(points.iter().all(|p| inside(*p)), "{family:?}: {mark:?}");
            }
        }
        // The tricks differ only by their marks on the same shaded box.
        let tricks = [
            Family::Centre,
            Family::Halves,
            Family::Thirds,
            Family::Step,
            Family::Mirror,
        ];
        let pictures: Vec<Vec<Mark>> = tricks.iter().map(|f| sketch(*f)).collect();
        for (a, first) in pictures.iter().enumerate() {
            for second in &pictures[a + 1..] {
                assert_ne!(first, second);
            }
        }
    }
}
