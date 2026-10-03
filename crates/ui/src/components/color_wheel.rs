//! The colour wheel: a hue ring around a square of saturation (left to right) and value (bottom
//! to top) for that hue. Where the hand presses decides what it moves until it lifts: on the
//! ring the hue, in the square saturation and value; the marker jumps there and follows it.
//! Lifting the hand settles the colour.

use std::f32::consts::{SQRT_2, TAU};

use egui::{Color32, Mesh, Pos2, Rect, Sense, Shape, Stroke, Vec2, WidgetInfo, WidgetType};
use openpaint_app::Hsv;

use crate::components::swatch::color32;
use crate::design::{HUE_RING, INK, PANEL, SPACE};

/// Steps around the ring and across the square; enough that the shading looks smooth.
const RING_STEPS: usize = 96;
const SQUARE_STEPS: usize = 16;
/// Radius of a marker, in points.
const MARKER: f32 = SPACE;

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
        let outer = rect.width().min(rect.height()) / 2.0;
        let inner = outer - HUE_RING;
        let side = (inner - SPACE) * SQRT_2;
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

/// Shows the wheel at `hsv`, `side` points square; returns the colour the hand set, if any.
pub fn color_wheel(ui: &mut egui::Ui, hsv: Hsv, side: f32) -> Option<WheelChange> {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(side), Sense::click_and_drag());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, "Colour wheel"));
    let shape = Geometry::of(rect);
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
        paint(ui, &shape, change.map_or(hsv, |change| change.hsv));
    }
    change
}

fn paint(ui: &egui::Ui, shape: &Geometry, hsv: Hsv) {
    let painter = ui.painter();
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
        mesh.colored_vertex(at(shape.inner, turn), color);
        mesh.colored_vertex(at(shape.outer, turn), color);
        if step > 0 {
            let i = 2 * step as u32;
            mesh.add_triangle(i - 2, i - 1, i);
            mesh.add_triangle(i - 1, i + 1, i);
        }
    }
    square_mesh(&mut mesh, shape.square, hsv.h);
    painter.add(Shape::mesh(mesh));
    let ring = at((shape.inner + shape.outer) / 2.0, hsv.h);
    let square = shape.square;
    let spot = Pos2::new(
        square.left() + hsv.s * square.width(),
        square.bottom() - hsv.v * square.height(),
    );
    for centre in [ring, spot] {
        painter.circle_stroke(centre, MARKER, Stroke::new(4.0, PANEL));
        painter.circle_stroke(centre, MARKER, Stroke::new(1.5, INK));
    }
}

/// The square of saturation and value for `hue`, as a grid of shaded cells.
fn square_mesh(mesh: &mut Mesh, square: Rect, hue: f32) {
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
            let color: Color32 = color32(Hsv { h: hue, s, v }.color());
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
