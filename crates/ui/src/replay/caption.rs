//! The video's last line (S163, the mock's sixth screen): on the held finished picture, under it in
//! a band of the frame's width, the line of the record's strokes, its layers, the time spent and
//! the days it was drawn on ("1,288 strokes · 6 layers · 3 h 12 min · 14–18 Nov 2026") at the left
//! and, quieter, "OpenPaint" at the right, in ink on the desk, in the language shown; the time and
//! the days only when a stroke kept its start. Laid by an egui context of its own with the host's
//! faces, so it is made anywhere (the composition root asks it once when a video with the line
//! starts) and painted by a painter of its own into a `ScreenTarget` of the band's size: the
//! screen's painter holds another context's font atlas.

use openpaint_app::{Language, ReplayFacts};
use openpaint_render::UiFrame;

use crate::design::TextFonts;

/// The caption band's pixels in a video of `video` pixels: its width, an eighth of its height
/// (even). The video worker fits the held picture above it.
#[must_use]
pub fn caption_band(video: [u32; 2]) -> [u32; 2] {
    [video[0], (video[1] / 8) & !1]
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
    // S163c: a context of its own with `fonts` and the theme, the words from the tables, laid
    // and tessellated for a target of `size`.
    let _ = (fonts, facts, language, utc_offset, size);
    UiFrame {
        primitives: Vec::new(),
        textures: egui::TexturesDelta::default(),
        pixels_per_point: 1.0,
    }
}
