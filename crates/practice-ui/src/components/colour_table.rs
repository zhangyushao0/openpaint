//! The colour table (S86, D108): how a colour differs from its original, read at a glance beside
//! the wheel. Its first row is ΔE, the Oklab distance × 100 (about 2 just noticeable); then its
//! three parts in the same unit (ΔE² = ΔL² + ΔC² + ΔH²), lightness, chroma and hue, each its
//! name, a strip and its signed number. A strip runs through the original at its middle, marked
//! by a red line (the original's red, as on the wheel): darker to lighter, greyer to purer, and
//! from the hue before the original's to the one after it round the wheel (the last two drawn
//! light and pure enough to tell). Under it an ink bar from that line to where yours lies, ended by an
//! ink tick across the strip; the largest part's bar reaches near the strip's end (or a
//! just-noticeable step does while every part is smaller), so the way to correct first reads
//! before the numbers do. No sentence.

use egui::{Align2, Color32, Mesh, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, TextStyle, Vec2};
use openpaint_practice::{
    Difference, HUELESS, NOTICEABLE, Rgb8, from_oklab, from_oklch, oklab, oklch,
};

use openpaint_ui::design::{HAIRLINE, INK, INK_SECONDARY, PANEL, PANEL_RULE, SPACE};

use crate::palette::CORRECTION;
use crate::text::{CHROMA, HUE, LIGHTNESS, Text, Words};

/// A row's height, in points.
const ROW: f32 = 4.0 * SPACE;
/// The strip's height, the gap under it and the bar's height, in points.
const STRIP: f32 = 12.0;
const GAP: f32 = 2.0;
const BAR: f32 = 6.0;
/// How much of a strip's half the largest bar reaches, so its tick stays inside the strip.
const REACH: f32 = 0.88;
/// The numbers' column, in points.
const NUMBER: f32 = 7.0 * SPACE;
/// Shades along a strip.
const STEPS: usize = 24;
/// How far a strip runs either side of the original: lightness, chroma (Oklab) and hue (degrees,
/// to the neighbouring hues of six).
const LIGHTNESS_REACH: f64 = 0.25;
const CHROMA_REACH: f64 = 0.08;
const HUE_REACH: f64 = 60.0;
/// The hue strip's least chroma, and the lightness range the chroma and hue strips are drawn in,
/// so greyer and purer, and the hues, show however grey, dark or light the original is (the
/// lightness strip alone is the original's own).
const HUE_CHROMA: f64 = 0.08;
const SHOWN_LIGHTNESS: [f64; 2] = [0.45, 0.8];

/// One part of the difference and its strip.
#[derive(Clone, Copy)]
enum Part {
    Lightness,
    Chroma,
    Hue,
}

impl Part {
    const ALL: [Self; 3] = [Self::Lightness, Self::Chroma, Self::Hue];

    const fn name(self) -> Text {
        match self {
            Self::Lightness => LIGHTNESS,
            Self::Chroma => CHROMA,
            Self::Hue => HUE,
        }
    }

    const fn of(self, difference: &Difference) -> f64 {
        match self {
            Self::Lightness => difference.lightness,
            Self::Chroma => difference.chroma,
            Self::Hue => difference.hue,
        }
    }
}

/// The original as the strips run through it: Oklab `L`, `a`, `b`, its chroma, and the hue the
/// chroma and hue strips turn about (the original's; a grey's, the copy's; two greys', none).
struct Through {
    lab: [f64; 3],
    chroma: f64,
    hue: Option<f64>,
}

impl Through {
    fn of(original: Rgb8, mine: Rgb8) -> Self {
        let [_, chroma, hue] = oklch(original);
        let [_, mine_chroma, mine_hue] = oklch(mine);
        let hue = if chroma >= HUELESS {
            Some(hue)
        } else {
            (mine_chroma >= HUELESS).then_some(mine_hue)
        };
        Self {
            lab: oklab(original),
            chroma,
            hue,
        }
    }

    /// The strip of `part` at `t`, from -1 (its left end) through 0 (the original) to 1.
    fn at(&self, part: Part, t: f64) -> Rgb8 {
        let [l, a, b] = self.lab;
        let [low, high] = SHOWN_LIGHTNESS;
        let shown = l.clamp(low, high);
        match (part, self.hue) {
            (Part::Lightness, _) => from_oklab([(l + t * LIGHTNESS_REACH).clamp(0.0, 1.0), a, b]),
            (Part::Chroma, Some(hue)) => {
                from_oklch([shown, (self.chroma + t * CHROMA_REACH).max(0.0), hue])
            }
            (Part::Hue, Some(hue)) => {
                from_oklch([shown, self.chroma.max(HUE_CHROMA), hue + t * HUE_REACH])
            }
            (_, None) => from_oklab([shown, 0.0, 0.0]),
        }
    }
}

/// The table of how `mine` differs from `original`, the card's width.
pub fn colour_table(ui: &mut egui::Ui, (original, mine): (Rgb8, Rgb8)) {
    let difference = Difference::of(original, mine);
    let font = TextStyle::Body.resolve(ui.style());
    let names = Part::ALL.map(|part| Words::from(part.name()).shown(ui.ctx()));
    let label = (names.iter().chain([&"ΔE"]))
        .map(|name| {
            let galley = ui
                .painter()
                .layout_no_wrap((*name).to_owned(), font.clone(), INK);
            galley.size().x
        })
        .fold(0.0, f32::max)
        + SPACE;
    let row = |ui: &mut egui::Ui, name: &str, number: &str| {
        let (rect, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), ROW), Sense::hover());
        let painter = ui.painter();
        let left = rect.left_center();
        painter.text(left, Align2::LEFT_CENTER, name, font.clone(), INK_SECONDARY);
        painter.text(
            rect.right_center(),
            Align2::RIGHT_CENTER,
            number,
            font.clone(),
            INK,
        );
        rect
    };
    row(ui, "ΔE", &format!("{:.1}", difference.delta * 100.0));
    let through = Through::of(original, mine);
    let scale = difference.largest_part().max(NOTICEABLE);
    for (part, name) in Part::ALL.into_iter().zip(names) {
        let value = part.of(&difference);
        let rect = row(ui, name, &signed(value));
        let top = rect.center().y - (STRIP + GAP + BAR) / 2.0;
        let strip = Rect::from_min_max(
            Pos2::new(rect.left() + label, top),
            Pos2::new(rect.right() - NUMBER - SPACE, top + STRIP),
        );
        paint_strip(ui.painter(), strip, |t| through.at(part, t));
        // No bar where the number reads 0.0.
        let share = if (value * 100.0).abs() < 0.05 {
            0.0
        } else {
            value / scale
        };
        paint_bar(ui.painter(), strip, share);
    }
}

/// A part's number × 100, signed, one decimal: "+1.3", "−0.7", "0.0".
fn signed(value: f64) -> String {
    let shown = value * 100.0;
    if shown.abs() < 0.05 {
        "0.0".to_owned()
    } else if shown > 0.0 {
        format!("+{shown:.1}")
    } else {
        format!("\u{2212}{:.1}", shown.abs())
    }
}

/// The strip in `rect`, shaded from `colour(-1)` to `colour(1)`, in a hairline of ink.
fn paint_strip(painter: &egui::Painter, rect: Rect, colour: impl Fn(f64) -> Rgb8) {
    let mut mesh = Mesh::default();
    for step in 0..=STEPS {
        let share = step as f32 / STEPS as f32;
        let [r, g, b] = colour(f64::from(share) * 2.0 - 1.0);
        let shade = Color32::from_rgb(r, g, b);
        let x = rect.left() + share * rect.width();
        mesh.colored_vertex(Pos2::new(x, rect.top()), shade);
        mesh.colored_vertex(Pos2::new(x, rect.bottom()), shade);
        if step > 0 {
            let i = 2 * step as u32;
            mesh.add_triangle(i - 2, i - 1, i);
            mesh.add_triangle(i - 1, i + 1, i);
        }
    }
    painter.add(Shape::mesh(mesh));
    let outline = Stroke::new(HAIRLINE, PANEL_RULE);
    painter.rect_stroke(rect, 0.0, outline, StrokeKind::Outside);
}

/// The original's red line through the strip's middle, and the ink bar under it to `share` of
/// the way to an end (negative: left), ended by an ink tick across the strip.
fn paint_bar(painter: &egui::Painter, strip: Rect, share: f64) {
    let centre = strip.center().x;
    let reach = REACH * strip.width() / 2.0;
    let end = centre + share.clamp(-1.0, 1.0) as f32 * reach;
    let bar_top = strip.bottom() + GAP;
    let haloed = |from: Pos2, to: Pos2, (width, colour): (f32, Color32)| {
        painter.line_segment([from, to], Stroke::new(width + 2.5, PANEL));
        painter.line_segment([from, to], Stroke::new(width, colour));
    };
    if (end - centre).abs() >= 0.5 {
        let bar = Rect::from_x_y_ranges(centre.min(end)..=centre.max(end), bar_top..=bar_top + BAR);
        painter.rect_filled(bar, 0.0, INK);
        let tick = [
            Pos2::new(end, strip.top() - 2.0),
            Pos2::new(end, bar_top + BAR),
        ];
        haloed(tick[0], tick[1], (1.5, INK));
    }
    let line = [
        Pos2::new(centre, strip.top() - 3.0),
        Pos2::new(centre, bar_top + BAR + 1.0),
    ];
    haloed(line[0], line[1], (2.0, CORRECTION));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_part_reads_signed_with_one_decimal() {
        assert_eq!(signed(0.013), "+1.3");
        assert_eq!(signed(-0.007_1), "\u{2212}0.7");
        assert_eq!(signed(0.000_2), "0.0");
        assert_eq!(signed(-0.000_4), "0.0");
    }

    #[test]
    fn the_strips_run_through_the_original_and_its_neighbouring_hues() {
        let olive = [42, 46, 37];
        let through = Through::of(olive, [50, 47, 42]);
        // Lightness: darker, the original itself, lighter.
        let [dark, middle, light] = [-1.0, 0.0, 1.0].map(|t| oklch(through.at(Part::Lightness, t)));
        assert!(dark[0] < middle[0] && middle[0] < light[0]);
        assert_eq!(through.at(Part::Lightness, 0.0), olive);
        // Chroma: grey at the left end, purer at the right, the original's hue.
        let [grey, _, pure] = [-1.0, 0.0, 1.0].map(|t| oklch(through.at(Part::Chroma, t)));
        assert!(grey[1] < HUELESS && pure[1] > 0.06, "{grey:?} {pure:?}");
        // Hue: 60 degrees either way of the olive's, pure enough to tell.
        let [before, at, after] = [-1.0, 0.0, 1.0].map(|t| oklch(through.at(Part::Hue, t)));
        assert!((at[2] - 127.0).abs() < 3.0, "{at:?}");
        assert!((before[2] - 67.0).abs() < 4.0 && (after[2] - 187.0).abs() < 4.0);
        assert!(before[1] > 0.05 && after[1] > 0.05);
        // Two greys have no hue to turn about.
        let greys = Through::of([120, 120, 120], [128, 128, 128]);
        assert!(greys.hue.is_none());
        assert_eq!(greys.at(Part::Hue, 1.0), greys.at(Part::Hue, -1.0));
    }
}
