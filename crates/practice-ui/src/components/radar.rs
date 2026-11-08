//! The radar: a few scores of one thing (the colour judgement's six axes) round a web, each from
//! 0 at the centre to 1 at the rim, so the shape reads at a glance: larger is better, a dent is
//! the weak side. This session's shape in ink over a faint wash; the earlier sessions' as a
//! dashed outline under it. An axis without a value leaves its point out of the shape; its name
//! says "—".

use std::f32::consts::TAU;

use egui::{Align2, Color32, Mesh, Pos2, Sense, Shape, Stroke, TextStyle, Vec2};

use openpaint_ui::design::{DIVIDER, HAIRLINE, INK, INK_SECONDARY, SPACE};

/// Rings drawn on the web, as shares of the rim.
const RINGS: [f32; 4] = [0.25, 0.5, 0.75, 1.0];
/// Room round the web for the axes' names and values (two lines), across and down, in points.
const LABEL_ROOM: [f32; 2] = [10.0 * SPACE, 5.0 * SPACE];

/// One axis: its name, this value and an earlier one, each `0..=1`.
pub type RadarAxis<'a> = (&'a str, Option<f32>, Option<f32>);

/// The radar of `axes` (clockwise from the top), `width` points wide.
pub fn radar(ui: &mut egui::Ui, axes: &[RadarAxis<'_>], width: f32) {
    let radius = (width / 2.0 - LABEL_ROOM[0]).max(SPACE);
    let height = 2.0 * (radius + LABEL_ROOM[1]);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    if !ui.is_rect_visible(rect) || axes.is_empty() {
        return;
    }
    let centre = rect.center();
    let count = axes.len();
    let direction = |index: usize| {
        let turn = index as f32 / count as f32 * TAU;
        Vec2::new(turn.sin(), -turn.cos())
    };
    let at = |index: usize, share: f32| centre + direction(index) * radius * share.clamp(0.0, 1.0);
    let painter = ui.painter();
    let web = Stroke::new(HAIRLINE, DIVIDER);
    for ring in RINGS {
        let points: Vec<Pos2> = (0..count).map(|index| at(index, ring)).collect();
        painter.add(Shape::closed_line(points, web));
    }
    for index in 0..count {
        painter.line_segment([centre, at(index, 1.0)], web);
    }
    let shape = |pick: fn(&RadarAxis<'_>) -> Option<f32>| -> Vec<Pos2> {
        (axes.iter().enumerate())
            .filter_map(|(index, axis)| pick(axis).map(|share| at(index, share)))
            .collect()
    };
    let before = shape(|axis| axis.2);
    if before.len() > 1 {
        let mut closed = before.clone();
        closed.push(before[0]);
        let dashes = Shape::dashed_line(&closed, Stroke::new(1.5, INK_SECONDARY), 6.0, 4.0);
        painter.extend(dashes);
    }
    this_shape(painter, centre, &shape(|axis| axis.1));
    let font = TextStyle::Small.resolve(ui.style());
    for (index, (name, value, _)) in axes.iter().enumerate() {
        let shown = value.map_or_else(|| "—".to_owned(), |v| format!("{:.0}", v * 100.0));
        let d = direction(index);
        let anchor = match (d.x, d.y) {
            (x, _) if x > 0.3 => Align2::LEFT_CENTER,
            (x, _) if x < -0.3 => Align2::RIGHT_CENTER,
            (_, y) if y < 0.0 => Align2::CENTER_BOTTOM,
            _ => Align2::CENTER_TOP,
        };
        let place = centre + d * (radius + SPACE / 2.0);
        painter.text(place, anchor, format!("{name}\n{shown}"), font.clone(), INK);
    }
}

/// This session's shape round `centre`: a wash in a fan from the centre (the shape is
/// star-shaped round it, though seldom convex), its outline in ink and a dot at each value.
fn this_shape(painter: &egui::Painter, centre: Pos2, this: &[Pos2]) {
    if this.len() > 2 {
        let wash = Color32::from_rgba_unmultiplied(INK.r(), INK.g(), INK.b(), 28);
        let mut mesh = Mesh::default();
        mesh.colored_vertex(centre, wash);
        for point in this {
            mesh.colored_vertex(*point, wash);
        }
        let last = this.len() as u32;
        for k in 1..=last {
            mesh.add_triangle(0, k, k % last + 1);
        }
        painter.add(Shape::mesh(mesh));
    }
    if this.len() > 1 {
        painter.add(Shape::closed_line(this.to_vec(), Stroke::new(2.0, INK)));
    }
    for point in this {
        painter.circle_filled(*point, 3.0, INK);
    }
}
