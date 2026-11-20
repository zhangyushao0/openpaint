//! The video's last line (S163, the mock's sixth screen): on the held finished picture, under it in
//! a band of the frame's width, the line of the record's strokes, its layers, the time spent and
//! the days it was drawn on ("1,288 strokes · 6 layers · 3 h 12 min · 14–18 Nov 2026") at the left
//! and, quieter, "OpenPaint" at the right, in ink on the desk, in the language shown; the time and
//! the days only when a stroke kept its start. Laid by an egui context of its own with the host's
//! faces, so it is made anywhere (the composition root asks it once when a video with the line
//! starts) and painted by a painter of its own into a `ScreenTarget` of the band's size: the
//! screen's painter holds another context's font atlas.

use egui::{Align2, FontId, Pos2, Rect, Vec2};
use openpaint_app::{Language, ReplayFacts};
use openpaint_render::UiFrame;

use super::local;
use crate::design::{self, DESK, INK, INK_SECONDARY, TextFonts};
use crate::text as t;

/// The type's height as a share of the video's (a fortieth), and the band's of the video's (an
/// eighth).
const TYPE_OF_VIDEO: f32 = 1.0 / 40.0;
const BAND_OF_VIDEO: u32 = 8;
/// The app's name beside the line, smaller.
const BRAND_SIZE: f32 = 0.85;
/// From the band's ends to the words, and between the line and the name, in type heights.
const MARGIN: f32 = 2.0;

/// The caption band's pixels in a video of `video` pixels: its width, an eighth of its height
/// (even). The video worker fits the held picture above it.
#[must_use]
pub fn caption_band(video: [u32; 2]) -> [u32; 2] {
    [video[0], (video[1] / BAND_OF_VIDEO) & !1]
}

/// The band of `size` pixels with the line of `facts` in `language`, the days by the device's
/// offset from UTC (`utc_offset`, minutes), the type a fortieth of the video's height.
#[must_use]
pub fn video_caption(
    fonts: &TextFonts,
    facts: &ReplayFacts,
    (language, utc_offset): (Language, i32),
    size: [u32; 2],
) -> UiFrame {
    let ctx = egui::Context::default();
    design::install(&ctx);
    design::install_text_fonts(&ctx, fonts.clone());
    let days = facts
        .began_ms
        .map(|began| began.map(|ms| local(ms, utc_offset).0));
    let line = t::caption(language, facts, days);
    let points = Vec2::new(size[0] as f32, size[1] as f32);
    let input = || {
        let mut input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, points)),
            ..Default::default()
        };
        let root = input.viewports.entry(egui::ViewportId::ROOT).or_default();
        root.native_pixels_per_point = Some(1.0);
        input
    };
    // The faces become the context's from its next pass on: the first lays nothing.
    let mut textures = ctx.run_ui(input(), |_| {}).textures_delta;
    let output = ctx.run_ui(input(), |ui| paint_band(ui, &line, points));
    textures.append(output.textures_delta);
    UiFrame {
        primitives: ctx.tessellate(output.shapes, output.pixels_per_point),
        textures,
        pixels_per_point: output.pixels_per_point,
    }
}

/// The desk over the whole band, the line at its left and the app's name at its right, both
/// centred on its height, the type smaller where the two would not fit side by side.
fn paint_band(ui: &egui::Ui, line: &str, band: Vec2) {
    let painter = ui.painter();
    let whole = Rect::from_min_size(Pos2::ZERO, band);
    painter.rect_filled(whole, 0.0, DESK);
    let video_height = band.y * BAND_OF_VIDEO as f32;
    let size = video_height * TYPE_OF_VIDEO;
    let measure = |text: &str, size: f32| {
        let font = FontId::proportional(size);
        painter.layout_no_wrap(text.to_owned(), font, INK).size().x
    };
    let needed = measure(line, size) + measure(t::BRAND, size * BRAND_SIZE) + 3.0 * MARGIN * size;
    let size = size * (band.x / needed).min(1.0);
    let middle = band.y / 2.0;
    let left = Pos2::new(MARGIN * size, middle);
    painter.text(
        left,
        Align2::LEFT_CENTER,
        line,
        FontId::proportional(size),
        INK,
    );
    let right = Pos2::new(band.x - MARGIN * size, middle);
    let brand = FontId::proportional(size * BRAND_SIZE);
    painter.text(right, Align2::RIGHT_CENTER, t::BRAND, brand, INK_SECONDARY);
}
