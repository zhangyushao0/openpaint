//! A copy drawn small on a card (S147): a blind pair's side on a lesson's end and a re-test's copy
//! beside its curve in the book. White paper with a thin frame (outlined in ink when it is the one
//! chosen); the copy's frame fitted in it, its pack picture under it faintly, a colour's original
//! as a band along its top, what was laid in it before its first Compare in its own colours, and
//! the original or the answer over it in see-through red, hidden parts dashed. Nothing more: no
//! date or mark here, the caller writes those under it.

use egui::{Color32, Pos2, Rect, Shape, Stroke, TextureHandle, Vec2};
use openpaint_practice::Point;
use openpaint_practice_app::{CardMark, CopyCard, UNDER};
use openpaint_ui::components::{Alpha, paint_sheet, picture_texture};
use openpaint_ui::design::{INK, PANEL_RULE};

use crate::palette::CORRECTION;

/// The frame's line, the chosen one's, the laid lines' width and the least a stroke shows at, in
/// points; how far in from the card the copy's frame keeps; the swatch band's share of the frame.
const FRAME_LINE: f32 = 1.0;
const CHOSEN_LINE: f32 = 2.25;
const LAID: f32 = 2.0;
const LEAST: f32 = 1.0;
const INSET: f32 = 6.0;
const BAND: f64 = 0.22;
/// How strongly the laid lines show.
const SEE_THROUGH: f32 = 0.7;
/// A dash and its gap, in points.
const DASH: f32 = 5.0;
const GAP: f32 = 4.0;

/// `card` in `rect` (none: still being read, the paper alone), outlined in ink when `chosen`.
pub(crate) fn paint_copy(ui: &egui::Ui, rect: Rect, card: Option<&CopyCard>, chosen: bool) {
    let painter = ui.painter();
    paint_sheet(painter, None, rect, Color32::WHITE);
    let line = if chosen {
        Stroke::new(CHOSEN_LINE, INK)
    } else {
        Stroke::new(FRAME_LINE, PANEL_RULE)
    };
    painter.rect_stroke(rect, 0.0, line, egui::StrokeKind::Inside);
    let Some(card) = card else {
        return;
    };
    let inner = rect.shrink(INSET);
    let [w, h] = card.extent;
    let scale = (f64::from(inner.width()) / w).min(f64::from(inner.height()) / h) as f32;
    let size = Vec2::new(w as f32, h as f32) * scale;
    let frame = Rect::from_center_size(inner.center(), size);
    let at = |p: Point| frame.min + Vec2::new(p[0] as f32, p[1] as f32) * scale;
    let clipped = painter.with_clip_rect(frame.intersect(painter.clip_rect()));
    if let Some((image, [top_left, bottom_right])) = &card.picture {
        let texture = texture(ui.ctx(), card, image);
        let [a, b] = [at(*top_left), at(*bottom_right)];
        let corners = [a, Pos2::new(b.x, a.y), b, Pos2::new(a.x, b.y)];
        let tint = Color32::from_white_alpha((UNDER * 255.0) as u8);
        clipped.add(quad(&texture, corners, tint));
    }
    for mark in &card.marks {
        match mark {
            CardMark::Fill { rect, colour: fill } => {
                let shown = Rect::from_min_max(at(rect[0]), at(rect[1]));
                clipped.rect_filled(shown, 0.0, colour(*fill));
            }
            CardMark::Line {
                points,
                colour: ink,
                width,
            } => {
                let stroke = Stroke::new((*width as f32 * scale).max(LEAST), colour(*ink));
                clipped.add(Shape::line(points.iter().map(|p| at(*p)).collect(), stroke));
            }
        }
    }
    // A colour's original over the top of what was laid, as the copy's own band beside it.
    if let Some(swatch) = card.swatch {
        let band = Rect::from_min_max(at([0.0, 0.0]), at([w, h * BAND]));
        clipped.rect_filled(band, 0.0, colour(swatch));
    }
    let red = Stroke::new(LAID, CORRECTION.gamma_multiply(SEE_THROUGH));
    for (points, dashed) in &card.laid {
        let shown: Vec<Pos2> = points.iter().map(|p| at(*p)).collect();
        if *dashed {
            clipped.extend(Shape::dashed_line(&shown, red, DASH, GAP));
        } else {
            clipped.add(Shape::line(shown, red));
        }
    }
}

/// `card`'s picture as a texture, uploaded once and kept with the context.
fn texture(
    ctx: &egui::Context,
    card: &CopyCard,
    image: &openpaint_practice_app::PictureImage,
) -> TextureHandle {
    let id = egui::Id::new(("copy card", card.copy.sheet, card.copy.exercise));
    if let Some(kept) = ctx.data(|data| data.get_temp::<TextureHandle>(id)) {
        return kept;
    }
    let name = format!("copy-card-{}-{}", card.copy.sheet, card.copy.exercise);
    let made = picture_texture(ctx, name, (image.size, &image.pixels, Alpha::Straight));
    ctx.data_mut(|data| data.insert_temp(id, made.clone()));
    made
}

fn colour([r, g, b]: openpaint_practice::Rgb8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

/// `texture` laid on the quad `corners` (top left, top right, bottom right, bottom left), tinted.
fn quad(texture: &TextureHandle, corners: [Pos2; 4], tint: Color32) -> Shape {
    let mut mesh = egui::Mesh::with_texture(texture.id());
    let uv = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    for (pos, [u, v]) in corners.into_iter().zip(uv) {
        mesh.vertices.push(egui::epaint::Vertex {
            pos,
            uv: egui::pos2(u, v),
            color: tint,
        });
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    Shape::mesh(mesh)
}
