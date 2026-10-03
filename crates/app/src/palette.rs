//! The colour the brush paints with and the colours chosen recently (Procreate's colour
//! history). Editor state like the brushes: it belongs to the app, not to one artwork, and is
//! stored apart from artworks. A colour joins the recent ones when it settles: the finger lifts
//! from the colour wheel, a recent colour is tapped, the eyedropper picks.

use std::sync::Arc;

use openpaint_core::Color;

/// How many recent colours the palette keeps, newest first.
pub const RECENT_COLORS: usize = 8;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Palette {
    color: Color,
    /// Newest first, no colour twice, at most `RECENT_COLORS`.
    recent: Vec<Color>,
}

impl Default for Palette {
    /// Black, as every stroke before colours could be chosen; no recent colours.
    fn default() -> Self {
        Self {
            color: Color::BLACK,
            recent: Vec::new(),
        }
    }
}

impl Palette {
    pub(crate) const fn color(&self) -> Color {
        self.color
    }

    pub(crate) fn recent(&self) -> Arc<[Color]> {
        self.recent.iter().copied().collect()
    }

    /// Paints with `color`, rounded to 8 bits per channel, from now on; a `settled` colour moves
    /// to the front of the recent ones. Returns whether anything changed. Rounded, a colour from
    /// the wheel and the same colour picked from the canvas (8-bit) are one colour.
    pub(crate) fn set(&mut self, color: Color, settled: bool) -> bool {
        let color = eight_bit(color);
        let mut changed = self.color != color;
        self.color = color;
        if settled && self.recent.first() != Some(&color) {
            self.recent.retain(|recent| *recent != color);
            self.recent.insert(0, color);
            self.recent.truncate(RECENT_COLORS);
            changed = true;
        }
        changed
    }

    /// Takes over stored colours, `[r, g, b]` each; one out of range is skipped. Returns
    /// whether everything was taken over.
    pub(crate) fn restore(&mut self, color: [f32; 3], recent: &[[f32; 3]]) -> bool {
        let rgb = |[r, g, b]: [f32; 3]| Color::rgb(r, g, b).ok();
        let mut whole = recent.len() <= RECENT_COLORS;
        match rgb(color) {
            Some(color) => self.color = color,
            None => whole = false,
        }
        self.recent.clear();
        for stored in recent.iter().take(RECENT_COLORS) {
            match rgb(*stored) {
                Some(color) if !self.recent.contains(&color) => self.recent.push(color),
                _ => whole = false,
            }
        }
        whole
    }
}

/// `color` with each channel at the nearest of the 256 steps `k / 255`.
fn eight_bit(color: Color) -> Color {
    let step = |channel: f32| (channel * 255.0).round() / 255.0;
    Color::rgb(step(color.r()), step(color.g()), step(color.b())).unwrap_or(color)
}

/// A colour as hue, saturation and value, each in `0..=1` (hue 0 and 1 are both red), as the
/// colour wheel shows it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hsv {
    pub h: f32,
    pub s: f32,
    pub v: f32,
}

impl Hsv {
    /// The hue, saturation and value of `color`; a grey has hue 0, black saturation 0.
    #[must_use]
    pub fn of(color: Color) -> Self {
        let (r, g, b) = (color.r(), color.g(), color.b());
        let max = r.max(g).max(b);
        let span = max - r.min(g).min(b);
        let h = if span <= 0.0 {
            0.0
        } else if max == r {
            ((g - b) / span).rem_euclid(6.0) / 6.0
        } else if max == g {
            ((b - r) / span + 2.0) / 6.0
        } else {
            ((r - g) / span + 4.0) / 6.0
        };
        let s = if max <= 0.0 { 0.0 } else { span / max };
        Self { h, s, v: max }
    }

    /// The colour, every channel in `0..=1` whatever the inputs (clamped first).
    #[must_use]
    pub fn color(self) -> Color {
        let h = self.h.clamp(0.0, 1.0) * 6.0;
        let (s, v) = (self.s.clamp(0.0, 1.0), self.v.clamp(0.0, 1.0));
        let channel = |n: f32| {
            let k = (n + h) % 6.0;
            (v - v * s * k.min(4.0 - k).clamp(0.0, 1.0)).clamp(0.0, 1.0)
        };
        Color::rgb(channel(5.0), channel(3.0), channel(1.0)).unwrap_or(Color::BLACK)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rgb(r: f32, g: f32, b: f32) -> Color {
        Color::rgb(r, g, b).expect("colour")
    }

    #[test]
    fn hsv_round_trips_the_primaries_and_greys() {
        for (color, hsv) in [
            (rgb(1.0, 0.0, 0.0), [0.0, 1.0, 1.0]),
            (rgb(0.0, 1.0, 0.0), [1.0 / 3.0, 1.0, 1.0]),
            (rgb(0.0, 0.0, 1.0), [2.0 / 3.0, 1.0, 1.0]),
            (rgb(0.5, 0.5, 0.5), [0.0, 0.0, 0.5]),
            (Color::BLACK, [0.0, 0.0, 0.0]),
        ] {
            let [h, s, v] = hsv;
            assert_eq!(Hsv::of(color), Hsv { h, s, v });
            assert_eq!(Hsv { h, s, v }.color(), color);
        }
        let orange = rgb(1.0, 0.5, 0.25);
        let back = Hsv::of(orange).color();
        for (a, b) in [(back.r(), 1.0), (back.g(), 0.5), (back.b(), 0.25)] {
            assert!((a - b).abs() < 1e-6, "{back:?}");
        }
        assert_eq!(
            Hsv {
                h: 1.0,
                s: 1.0,
                v: 1.0
            }
            .color(),
            rgb(1.0, 0.0, 0.0)
        );
    }

    #[test]
    fn settled_colours_join_the_front_of_the_recent_ones_once() {
        let mut palette = Palette::default();
        assert!(palette.set(rgb(1.0, 0.0, 0.0), false));
        assert!(
            palette.recent().is_empty(),
            "a colour still moving is not recent"
        );
        let byte = |k: u8| f32::from(k) / 255.0;
        for step in 0..10u8 {
            palette.set(rgb(byte(20 * step), 0.0, 0.0), true);
        }
        palette.set(rgb(byte(100), 0.0, 0.0), true);
        let recent = palette.recent();
        assert_eq!(recent.len(), RECENT_COLORS);
        assert_eq!(recent[0], rgb(byte(100), 0.0, 0.0));
        assert_eq!(recent[1], rgb(byte(180), 0.0, 0.0));
        assert!(
            !palette.set(rgb(byte(100) + 1e-4, 0.0, 0.0), true),
            "the same 8-bit colour: already the newest"
        );
        palette.set(rgb(0.5, 0.0, 0.0), false);
        assert_eq!(
            palette.color(),
            rgb(byte(128), 0.0, 0.0),
            "rounded to 8 bits"
        );
    }

    #[test]
    fn restore_skips_colours_out_of_range() {
        let mut palette = Palette::default();
        assert!(palette.restore([0.1, 0.2, 0.3], &[[0.1, 0.2, 0.3], [1.0, 1.0, 1.0]]));
        assert_eq!(palette.color(), rgb(0.1, 0.2, 0.3));
        assert_eq!(palette.recent().len(), 2);
        assert!(!palette.restore([2.0, 0.0, 0.0], &[[f32::NAN, 0.0, 0.0]]));
        assert_eq!(palette.color(), rgb(0.1, 0.2, 0.3), "kept");
        assert!(palette.recent().is_empty());
    }
}
