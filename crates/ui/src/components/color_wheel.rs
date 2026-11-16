//! The colour wheel: a hue ring around a square of saturation (left to right) and value (bottom
//! to top) for that hue. Where the hand presses decides what it moves until it lifts: on the
//! ring the hue, in the square saturation and value; the marker jumps there and follows it.
//! Lifting the hand settles the colour.

use std::f32::consts::{SQRT_2, TAU};

use egui::{Color32, Mesh, Pos2, Rect, Sense, Shape, Stroke, Vec2, WidgetInfo, WidgetType};
use openpaint_app::Hsv;

use crate::components::controls;
use crate::components::swatch::color32;
use crate::design::{HUE_RING, INK, PANEL, SPACE};
use crate::text::{COLOUR_WHEEL, SATURATION_AND_VALUE, Words};

/// Steps around the ring and across the square; enough that the shading looks smooth.
const RING_STEPS: usize = 96;
const SQUARE_STEPS: usize = 16;
/// Radius of a marker, in points, and of one on the comparison picture (smaller, so the way from
/// one colour to the other shows even when they lie near).
const MARKER: f32 = SPACE;
const COMPARED: f32 = 0.75 * SPACE;
/// How far a dimmed colour fades into the paper.
const DIMMED: f32 = 0.72;

/// What a mode asks the wheel to dim, not refusing it (a gamut mask's outside, S153): hues on its
/// ring, colours in its square.
pub trait WheelDims {
    /// Whether the ring dims `hue` (a turn, 0 to 1).
    fn dims_hue(&self, hue: f32) -> bool;
    /// Whether the square dims `hsv`.
    fn dims(&self, hsv: Hsv) -> bool;
}

/// A colour the hand set this frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WheelChange {
    pub hsv: Hsv,
    /// True when the hand lifted: the last colour of this touch.
    pub settled: bool,
}

/// What a touch moves, from where it pressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Part {
    Ring,
    Square,
}

/// The wheel's geometry inside `rect`.
struct Geometry {
    centre: Pos2,
    outer: f32,
    inner: f32,
    square: Rect,
}

impl Geometry {
    fn of(rect: Rect) -> Self {
        Self::with_ring(rect, HUE_RING, SPACE)
    }

    /// Inside `rect`, the ring `ring` points wide and the square `gap` inside it.
    fn with_ring(rect: Rect, ring: f32, gap: f32) -> Self {
        let outer = rect.width().min(rect.height()) / 2.0;
        let inner = outer - ring;
        let side = (inner - gap) * SQRT_2;
        Self {
            centre: rect.center(),
            outer,
            inner,
            square: Rect::from_center_size(rect.center(), Vec2::splat(side)),
        }
    }

    fn part(&self, at: Pos2) -> Option<Part> {
        let distance = at.distance(self.centre);
        if self.square.expand(SPACE).contains(at) {
            Some(Part::Square)
        } else if (self.inner - SPACE..=self.outer + SPACE).contains(&distance) {
            Some(Part::Ring)
        } else {
            None
        }
    }

    /// `hsv` with what `part` sets from the point `at`.
    fn moved(&self, part: Part, at: Pos2, hsv: Hsv) -> Hsv {
        match part {
            Part::Ring => {
                let d = at - self.centre;
                Hsv {
                    h: ((-d.y).atan2(d.x) / TAU).rem_euclid(1.0),
                    ..hsv
                }
            }
            Part::Square => {
                let square = self.square;
                Hsv {
                    s: ((at.x - square.left()) / square.width()).clamp(0.0, 1.0),
                    v: ((square.bottom() - at.y) / square.height()).clamp(0.0, 1.0),
                    ..hsv
                }
            }
        }
    }
}

/// Shows the wheel at `hsv`, `side` points square, what `dims` asks dimmed; returns the colour the
/// hand set, if any.
pub fn color_wheel(
    ui: &mut egui::Ui,
    hsv: Hsv,
    side: f32,
    dims: Option<&dyn WheelDims>,
) -> Option<WheelChange> {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(side), Sense::click_and_drag());
    let name = Words::from(COLOUR_WHEEL).shown(ui.ctx());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, name));
    let shape = Geometry::of(rect);
    // The square lies on the ring's control.
    controls::register(ui, COLOUR_WHEEL.id(), rect);
    controls::register(ui, SATURATION_AND_VALUE.id(), shape.square);
    let held = response.is_pointer_button_down_on();
    let settled = response.drag_stopped() || response.clicked();
    let mut part = ui.data(|data| data.get_temp::<Part>(response.id));
    if part.is_none() && (held || settled) {
        // A tap can press and lift within one frame: then it starts where it lifted.
        part = ui
            .input(|input| input.pointer.press_origin())
            .or(response.interact_pointer_pos())
            .and_then(|origin| shape.part(origin));
        if let Some(part) = part {
            ui.data_mut(|data| data.insert_temp(response.id, part));
        }
    }
    if settled || !held {
        ui.data_mut(|data| data.remove::<Part>(response.id));
    }
    let change = part
        .zip(response.interact_pointer_pos())
        .filter(|_| held || settled)
        .map(|(part, at)| WheelChange {
            hsv: shape.moved(part, at, hsv),
            settled,
        })
        .filter(|change| change.settled || change.hsv != hsv);
    if ui.is_rect_visible(rect) {
        paint(ui, &shape, change.map_or(hsv, |change| change.hsv), dims);
    }
    change
}

fn paint(ui: &egui::Ui, shape: &Geometry, hsv: Hsv, dims: Option<&dyn WheelDims>) {
    let painter = ui.painter();
    paint_wheel(painter, shape, hsv.h, dims);
    for centre in [shape.on_ring(hsv.h), shape.spot(hsv)] {
        painter.circle_stroke(centre, MARKER, Stroke::new(4.0, PANEL));
        painter.circle_stroke(centre, MARKER, Stroke::new(1.5, INK));
    }
}

impl Geometry {
    /// The point of the ring's middle at `hue`.
    fn on_ring(&self, hue: f32) -> Pos2 {
        let radius = (self.inner + self.outer) / 2.0;
        self.centre + radius * Vec2::new((hue * TAU).cos(), -(hue * TAU).sin())
    }

    /// Where `hsv`'s saturation and value lie in the square.
    fn spot(&self, hsv: Hsv) -> Pos2 {
        let square = self.square;
        Pos2::new(
            square.left() + hsv.s * square.width(),
            square.bottom() - hsv.v * square.height(),
        )
    }
}

/// Two colours on the wheel as a picture (no hand moves it), `side` points square, the square
/// shaded at `to`'s hue: `from` ringed in ink, `to` in `mark`, and arrows in `mark` from one to
/// the other round the ring (when both have a hue, `hued`) and across the square. A mode's
/// card's comparison of a pick with its original (S76): the geometry the hand picks in.
pub fn color_compare(
    ui: &mut egui::Ui,
    (from, to): (Hsv, Hsv),
    (hued, mark): ([bool; 2], Color32),
    side: f32,
) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(side), Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let shape = Geometry::of(rect);
    let painter = ui.painter();
    paint_wheel(painter, &shape, to.h, None);
    let marks = [(from, hued[0], INK), (to, hued[1], mark)];
    for (hsv, has_hue, colour) in marks {
        let ring = has_hue.then(|| shape.on_ring(hsv.h));
        for centre in ring.into_iter().chain([shape.spot(hsv)]) {
            painter.circle_stroke(centre, COMPARED, Stroke::new(4.0, PANEL));
            painter.circle_stroke(centre, COMPARED, Stroke::new(2.0, colour));
        }
    }
    if hued[0] && hued[1] {
        // The short way round, along the ring's middle.
        let turn = (to.h - from.h + 0.5).rem_euclid(1.0) - 0.5;
        let steps = ((turn.abs() * RING_STEPS as f32).ceil() as usize).max(1);
        let path: Vec<Pos2> = (0..=steps)
            .map(|k| shape.on_ring(from.h + turn * k as f32 / steps as f32))
            .collect();
        arrow(painter, &path, mark);
    }
    arrow(painter, &[shape.spot(from), shape.spot(to)], mark);
}

/// A small wheel as a picture in `rect` (no hand moves it), its ring a share of its side: the
/// square at `pick`'s hue with `pick` marked as the wheel marks it, and each colour of `ringed` in
/// its own (the How card's demonstration, S92).
pub fn wheel_picture(painter: &egui::Painter, rect: Rect, pick: Hsv, ringed: &[(Hsv, Color32)]) {
    let ring = rect.width().min(rect.height()) * 0.13;
    let shape = Geometry::with_ring(rect, ring, ring * 0.3);
    paint_wheel(painter, &shape, pick.h, None);
    let marker = ring * 0.4;
    for (hsv, colour) in ringed {
        for centre in [shape.on_ring(hsv.h), shape.spot(*hsv)] {
            painter.circle_stroke(centre, marker, Stroke::new(4.0, PANEL));
            painter.circle_stroke(centre, marker, Stroke::new(2.0, *colour));
        }
    }
    for centre in [shape.on_ring(pick.h), shape.spot(pick)] {
        painter.circle_stroke(centre, marker, Stroke::new(3.5, PANEL));
        painter.circle_stroke(centre, marker, Stroke::new(1.5, INK));
    }
}

/// A line along `path` in `colour` from its start (inside the hollow marker there) to a head
/// whose tip touches the marker at its end; none where the path is too short for the head.
fn arrow(painter: &egui::Painter, path: &[Pos2], colour: Color32) {
    let (Some(&tip), Some(&start)) = (path.last(), path.first()) else {
        return;
    };
    let length: f32 = path.windows(2).map(|pair| pair[0].distance(pair[1])).sum();
    let head = 1.6 * COMPARED;
    if length < COMPARED + head {
        return;
    }
    let before = (path.iter().rev())
        .find(|p| p.distance(tip) >= COMPARED + head)
        .copied()
        .unwrap_or(start);
    let towards = (tip - before).normalized();
    let end = tip - towards * COMPARED;
    let back = end - towards * head;
    let mut line: Vec<Pos2> = (path.iter().copied())
        .take_while(|p| p.distance(tip) > COMPARED + head)
        .collect();
    line.push(back);
    painter.add(Shape::line(line, Stroke::new(2.5, colour)));
    let across = Vec2::new(-towards.y, towards.x) * head * 0.45;
    painter.add(Shape::convex_polygon(
        vec![end, back + across, back - across],
        colour,
        Stroke::NONE,
    ));
}

/// The hue ring and the square of saturation and value for `hue`, what `dims` asks faded into the
/// paper.
fn paint_wheel(painter: &egui::Painter, shape: &Geometry, hue: f32, dims: Option<&dyn WheelDims>) {
    let mut mesh = Mesh::default();
    let at = |radius: f32, turn: f32| {
        shape.centre + radius * Vec2::new((turn * TAU).cos(), -(turn * TAU).sin())
    };
    for step in 0..=RING_STEPS {
        let turn = step as f32 / RING_STEPS as f32;
        let color = color32(
            Hsv {
                h: turn,
                s: 1.0,
                v: 1.0,
            }
            .color(),
        );
        let color = dimmed(color, dims.is_some_and(|dims| dims.dims_hue(turn)));
        mesh.colored_vertex(at(shape.inner, turn), color);
        mesh.colored_vertex(at(shape.outer, turn), color);
        if step > 0 {
            let i = 2 * step as u32;
            mesh.add_triangle(i - 2, i - 1, i);
            mesh.add_triangle(i - 1, i + 1, i);
        }
    }
    square_mesh(&mut mesh, shape.square, hue, dims);
    painter.add(Shape::mesh(mesh));
}

/// `color`, faded into the paper where `dim`.
fn dimmed(color: Color32, dim: bool) -> Color32 {
    if dim {
        color.lerp_to_gamma(PANEL, DIMMED)
    } else {
        color
    }
}

/// The square of saturation and value for `hue`, as a grid of shaded cells, what `dims` asks
/// faded.
fn square_mesh(mesh: &mut Mesh, square: Rect, hue: f32, dims: Option<&dyn WheelDims>) {
    let first = mesh.vertices.len() as u32;
    let row = SQUARE_STEPS as u32 + 1;
    for j in 0..=SQUARE_STEPS {
        for i in 0..=SQUARE_STEPS {
            let (s, v) = (
                i as f32 / SQUARE_STEPS as f32,
                1.0 - j as f32 / SQUARE_STEPS as f32,
            );
            let at = Pos2::new(
                square.left() + s * square.width(),
                square.top() + (1.0 - v) * square.height(),
            );
            let hsv = Hsv { h: hue, s, v };
            let color = dimmed(
                color32(hsv.color()),
                dims.is_some_and(|dims| dims.dims(hsv)),
            );
            mesh.colored_vertex(at, color);
            if i > 0 && j > 0 {
                let corner = first + j as u32 * row + i as u32;
                let (left, up) = (corner - 1, corner - row);
                mesh.add_triangle(up - 1, up, corner);
                mesh.add_triangle(up - 1, corner, left);
            }
        }
    }
}
