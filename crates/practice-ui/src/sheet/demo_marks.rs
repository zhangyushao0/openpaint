//! A demonstration's marks on its example (`demo.rs`), as the sheet draws them: what the
//! exercise gives in ink, Sketch's pencil (an object's construction dark in its own step, faint
//! once the next begins), a pass in the air as a dotted trace that goes again,
//! Draw's ink, the dashed pencil of a horizon or a way to see it, a face or a negative shape
//! shaded in pencil (a constructed kind's givens printed grey, a hidden edge dashed, S97, S98), a
//! value scale's order numbered (S97); then Compare's see-through red answer (dashed where a solid
//! hides it), what is off ringed red or drawn on dashed red (S97, S98; nothing blue) and an
//! original laid faint; a division's lines in its levels'
//! colours, a whole copy's dimming once Draw's step begins and gone at Compare (S95). A hand's
//! mark draws along its length as time goes; what the exercise
//! gives and Compare's marks show at once, fading in. Marks keep inside their frame, a wide
//! example's inside the whole example (its vanishing points lie beside its frame).

use egui::{Color32, Mesh, Pos2, Rect, Shape, Stroke, TextureHandle, Vec2};
use openpaint_practice::{Demo, Drawn, Mark, Marker, On, Point, Rgb8, length};
use openpaint_practice_app::DIVISION_DIMMED;

use openpaint_ui::design::{DIVIDER, INK, INK_SECONDARY, PANEL, PANEL_RULE_WIDTH};

use super::demo::{PictureKey, by_hand, drawn, placed};
use super::demo_wheel::wheel;
use super::marks::{level_colour, quad};
use crate::palette::{CORRECTION, PENCIL, PRINTED};

/// Line widths, in points: what is given, the pencil, the ink, the red answer, an edge drawn on
/// past where it heads.
const GIVEN: f32 = 1.75;
const PENCIL_LINE: f32 = 1.25;
const INK_LINE: f32 = 2.0;
const ANSWER: f32 = 3.5;
const MISSED_LINE: f32 = 1.75;
const HATCH_LINE: f32 = 1.5;
/// A ring's radius and a dot's, in points; a cross's arm in frame units; a numeral's size, a share
/// of its frame's side.
const RING: f32 = 5.5;
const DOT: f32 = 3.0;
const CROSS: f64 = 0.045;
const NUMERAL: f32 = 0.06;
/// How strongly the red lies over the copy, a shape's picture shows faint, a combination's
/// original over its copy, pencil shades and a printed face (S98, as the sheet prints it).
const RED: f32 = 0.55;
const FAINT: f32 = 0.35;
const LAID: f32 = 0.5;
const SHADE: f32 = 0.45;
const PRINTED_SHADE: f32 = 0.22;
/// Seconds a pass in the air takes to go again.
const AIR_GOES: f64 = 0.6;

/// Where an example's frames lie: the original's (a copying kind's), the copy's (the only one
/// of a box, an ellipse or a point in space), the wheel (a colour's); an example fitted to the
/// card (`wide`) lies on paper the size of the whole example, its marks kept inside it; frames
/// side by side cropped to the part the example shows (`shown`, an object's, S94) show that part,
/// each its marks inside it.
pub(crate) struct Frames {
    pub(crate) original: Option<Rect>,
    pub(crate) copy: Rect,
    pub(crate) wheel: Option<Rect>,
    pub(crate) wide: Option<Rect>,
    pub(crate) shown: Option<[Rect; 2]>,
}

impl Frames {
    pub(crate) const fn of(original: Option<Rect>, copy: Rect, wheel: Option<Rect>) -> Self {
        Self {
            original,
            copy,
            wheel,
            wide: None,
            shown: None,
        }
    }
}

/// `demo` as it shows `seconds` after its start on `frames`, its pictures from `pictures`.
pub(crate) fn paint(
    painter: &egui::Painter,
    (demo, seconds): (&Demo, f64),
    frames: &Frames,
    pictures: &dyn Fn(&PictureKey) -> Option<TextureHandle>,
) {
    let papers = match (frames.wide, frames.shown) {
        (Some(paper), _) => [None, Some(paper)],
        (None, Some([original, copy])) => [Some(original), Some(copy)],
        (None, None) => [frames.original, Some(frames.copy)],
    };
    for rect in papers.into_iter().flatten() {
        painter.rect_filled(rect, 0.0, Color32::WHITE);
    }
    for mark in &demo.marks {
        let shown = drawn(mark, seconds);
        let Some(rect) = placed(frames, mark.on).filter(|_| shown > 0.0 && mark.on != On::Wheel)
        else {
            continue;
        };
        let cropped = (frames.shown).map(|[original, copy]| {
            if mark.on == On::Original {
                original
            } else {
                copy
            }
        });
        let clip = (frames.wide.or(cropped).unwrap_or(rect)).intersect(painter.clip_rect());
        let alpha = if by_hand(mark.pen) {
            going(mark, seconds)
        } else {
            shown
        } * dimmed(demo, mark, seconds);
        if alpha <= 0.0 {
            continue;
        }
        let colour = toned(demo, mark, seconds).gamma_multiply(alpha);
        on_frame(
            &painter.with_clip_rect(clip),
            rect,
            (mark, shown, alpha, colour),
            pictures,
        );
    }
    for rect in papers.into_iter().flatten() {
        let corners = vec![
            rect.left_top(),
            rect.right_top(),
            rect.right_bottom(),
            rect.left_bottom(),
        ];
        painter.add(Shape::closed_line(
            corners,
            Stroke::new(PANEL_RULE_WIDTH, INK),
        ));
    }
    if let Some(rect) = frames.wheel {
        wheel(painter, demo, seconds, rect);
    }
}

/// How much of a hand's mark still shows: a pass in the air goes once drawn.
fn going(mark: &Mark, seconds: f64) -> f32 {
    if mark.pen == Marker::Air && seconds > mark.to {
        (1.0 - (seconds - mark.to) / AIR_GOES).clamp(0.0, 1.0) as f32
    } else {
        1.0
    }
}

/// How long an object's construction takes to go faint once the next step begins (S94).
const FADING: f64 = 0.4;

/// How much of a whole copy's division shows `seconds` after the start (S95): all of it, going
/// down to `DIVISION_DIMMED` once Draw's step begins, none from Compare's; 1 for any other mark.
fn dimmed(demo: &Demo, mark: &Mark, seconds: f64) -> f32 {
    let (Marker::Level(_), Some([from, gone])) = (mark.pen, demo.dimmed) else {
        return 1.0;
    };
    if seconds >= gone {
        return 0.0;
    }
    let t = ((seconds - from) / FADING).clamp(0.0, 1.0) as f32;
    1.0 + (DIVISION_DIMMED - 1.0) * t
}

/// What `mark` draws in `seconds` after the start: its pen's colour; an object's construction
/// dark while its step draws, going faint once a later step begins (S94).
pub(crate) fn toned(demo: &Demo, mark: &Mark, seconds: f64) -> Color32 {
    if mark.pen != Marker::Construction {
        return colour_of(mark.pen);
    }
    let next = demo.steps.get(demo.step_at(mark.from) + 1).copied();
    let gone = next.map_or(0.0, |next| ((seconds - next) / FADING).clamp(0.0, 1.0));
    INK.lerp_to_gamma(PENCIL, gone as f32)
}

/// `mark` on the frame `rect`, drawn `shown` of the way, at `alpha`, its lines and dots in
/// `colour`.
fn on_frame(
    painter: &egui::Painter,
    rect: Rect,
    (mark, shown, alpha, colour): (&Mark, f32, f32, Color32),
    pictures: &dyn Fn(&PictureKey) -> Option<TextureHandle>,
) {
    let at = |p: Point| rect.min + Vec2::new(p[0] as f32, p[1] as f32) * rect.width();
    let pen = mark.pen;
    match &mark.drawn {
        Drawn::Line(points) => {
            let share = if by_hand(pen) { shown } else { 1.0 };
            let path: Vec<Pos2> = along(points, share).into_iter().map(at).collect();
            line(painter, pen, path, colour);
        }
        Drawn::Ring(p) => ring(painter, (at(*p), rect.width()), pen, alpha),
        Drawn::Cross(p) => {
            let arms = [[CROSS, 0.0], [0.0, CROSS]];
            for (index, [dx, dy]) in arms.into_iter().enumerate() {
                let share = if by_hand(pen) {
                    (shown * 2.0 - index as f32).clamp(0.0, 1.0)
                } else {
                    1.0
                };
                let ends = [[p[0] - dx, p[1] - dy], [p[0] + dx, p[1] + dy]];
                let path: Vec<Pos2> = along(&ends, share).into_iter().map(at).collect();
                line(painter, pen, path, colour);
            }
        }
        Drawn::Dot(p) => {
            painter.circle_filled(at(*p), DOT, colour);
        }
        Drawn::Numeral(p, number) => numeral(painter, (at(*p), rect.width()), *number, colour),
        Drawn::Band { rows, colour } => {
            let fill = colour.map_or(shade_of(pen), rgb);
            let strength = if pen == Marker::Faint { FAINT } else { 1.0 };
            band(painter, rows, &at, fill.gamma_multiply(alpha * strength));
        }
        Drawn::Areas { colours, soft } => {
            let strength = if pen == Marker::Faint { LAID } else { 1.0 };
            let key = PictureKey::Areas(colours.clone(), *soft);
            laid(painter, rect, pictures(&key), alpha * strength);
        }
        Drawn::Shaded => laid(painter, rect, pictures(&PictureKey::Object), alpha),
        Drawn::Masses([low, high]) => {
            let strength = if pen == Marker::Faint { FAINT } else { 1.0 };
            let place = Rect::from_min_max(at(*low), at(*high));
            let picture = pictures(&PictureKey::Masses);
            laid(painter, place, picture, alpha * strength);
        }
        Drawn::Values {
            rect: [low, high],
            copy,
            shown,
        } => {
            // The answer's greys over the copy at the slider's first strength (S96).
            let strength = if pen == Marker::Faint { LAID } else { 1.0 };
            let key = PictureKey::Values {
                copy: *copy,
                shown: *shown,
            };
            let place = Rect::from_min_max(at(*low), at(*high));
            laid(painter, place, pictures(&key), alpha * strength);
        }
        Drawn::Pick(_) | Drawn::Ringed(_) => {}
    }
}

/// `number` centred at `centre`, in step with its frame's `side` (points), in `colour`.
fn numeral(painter: &egui::Painter, (centre, side): (Pos2, f32), number: u8, colour: Color32) {
    let font = egui::FontId::proportional((side * NUMERAL).max(8.0));
    let middle = egui::Align2::CENTER_CENTER;
    painter.text(centre, middle, number.to_string(), font, colour);
}

/// `texture` (if made) laid on `rect`, `strength` strong.
fn laid(painter: &egui::Painter, rect: Rect, texture: Option<TextureHandle>, strength: f32) {
    if let Some(texture) = texture {
        let corners = [
            rect.left_top(),
            rect.right_top(),
            rect.right_bottom(),
            rect.left_bottom(),
        ];
        let tint = Color32::WHITE.gamma_multiply(strength);
        painter.add(quad(&texture, corners, tint));
    }
}

/// The shade `pen` lays a face or a shape in: printed grey (a constructed kind's given face,
/// S98) or pencil.
fn shade_of(pen: Marker) -> Color32 {
    match pen {
        Marker::Printed => PRINTED.gamma_multiply(PRINTED_SHADE),
        _ => PENCIL.gamma_multiply(SHADE),
    }
}

/// The colour `pen` draws in: a division's level's in V大's colour (S95).
pub(crate) fn colour_of(pen: Marker) -> Color32 {
    match pen {
        Marker::Given | Marker::Ink | Marker::Construction => INK,
        Marker::Printed | Marker::PrintedHidden => PRINTED,
        Marker::Pencil | Marker::Air | Marker::Guide => INK_SECONDARY,
        Marker::Answer | Marker::Hidden | Marker::Missed => CORRECTION,
        Marker::Rule => DIVIDER,
        Marker::Shade | Marker::Faint => PENCIL,
        Marker::Level(level) => level_colour(level),
        Marker::Hatch(value) => {
            let [r, g, b] = value.grey();
            Color32::from_rgb(r, g, b)
        }
    }
}

/// A line through `path` as `pen` draws it, in `colour`.
fn line(painter: &egui::Painter, pen: Marker, path: Vec<Pos2>, colour: Color32) {
    if path.len() < 2 {
        return;
    }
    let (width, dashes) = match pen {
        Marker::Given | Marker::Printed => (GIVEN, None),
        Marker::PrintedHidden => (GIVEN / 2.0, Some((5.0, 4.0))),
        Marker::Ink | Marker::Level(_) => (INK_LINE, None),
        Marker::Answer => (ANSWER, None),
        Marker::Hidden => (ANSWER / 2.0, Some((6.0, 4.0))),
        Marker::Missed => (MISSED_LINE, Some((6.0, 4.0))),
        Marker::Guide => (1.0, Some((5.0, 4.0))),
        Marker::Air => (PENCIL_LINE, Some((2.0, 3.0))),
        Marker::Rule => (1.0, None),
        Marker::Pencil | Marker::Construction | Marker::Shade | Marker::Faint => {
            (PENCIL_LINE, None)
        }
        Marker::Hatch(_) => (HATCH_LINE, None),
    };
    let colour = if matches!(pen, Marker::Answer | Marker::Hidden) {
        colour.gamma_multiply(RED)
    } else {
        colour
    };
    let stroke = Stroke::new(width, colour);
    match dashes {
        Some((dash, gap)) => painter.extend(Shape::dashed_line(&path, stroke, dash, gap)),
        None => {
            painter.add(Shape::line(path, stroke));
        }
    }
}

/// A ring at `centre` as `pen` draws it: a given end's small ring on paper (ink or printed), a
/// pencil ring, or Compare's red ring over the paper's halo; `side` the frame's, in points.
fn ring(painter: &egui::Painter, (centre, side): (Pos2, f32), pen: Marker, alpha: f32) {
    let colour = colour_of(pen).gamma_multiply(alpha);
    match pen {
        Marker::Given | Marker::Printed => {
            let radius = side * 0.025;
            painter.circle(centre, radius, PANEL, Stroke::new(GIVEN * 0.75, colour));
        }
        Marker::Answer => {
            painter.circle_stroke(centre, RING, Stroke::new(4.0, PANEL.gamma_multiply(alpha)));
            painter.circle_stroke(centre, RING, Stroke::new(2.0, colour));
        }
        _ => {
            painter.circle_stroke(centre, RING, Stroke::new(PENCIL_LINE, colour));
        }
    }
}

/// The area between `rows`, each row joined to the next.
fn band(painter: &egui::Painter, rows: &[[Point; 2]], at: &dyn Fn(Point) -> Pos2, fill: Color32) {
    let mut mesh = Mesh::default();
    for [left, right] in rows {
        mesh.colored_vertex(at(*left), fill);
        mesh.colored_vertex(at(*right), fill);
    }
    for row in 1..rows.len() as u32 {
        let i = 2 * row;
        mesh.add_triangle(i - 2, i - 1, i);
        mesh.add_triangle(i - 1, i + 1, i);
    }
    painter.add(Shape::mesh(mesh));
}

/// The first `share` of `points` along their length.
fn along(points: &[Point], share: f32) -> Vec<Point> {
    if share >= 1.0 {
        return points.to_vec();
    }
    let goal = length(points) * f64::from(share.max(0.0));
    let mut out = Vec::new();
    let mut walked = 0.0;
    for pair in points.windows(2) {
        out.push(pair[0]);
        let step = openpaint_practice::distance(pair[0], pair[1]);
        if walked + step >= goal {
            let t = if step > 0.0 {
                (goal - walked) / step
            } else {
                0.0
            };
            out.push([
                pair[0][0] + (pair[1][0] - pair[0][0]) * t,
                pair[0][1] + (pair[1][1] - pair[0][1]) * t,
            ]);
            return out;
        }
        walked += step;
    }
    points.to_vec()
}

fn rgb([r, g, b]: Rgb8) -> Color32 {
    Color32::from_rgb(r, g, b)
}
