//! Colours as the colour exercises judge them: 8-bit sRGB in, HSV (hue, saturation and value as
//! the colour panel shows them, where the hand picks) and Oklab (Björn Ottosson's perceptual
//! space: equal steps look about equally different; its `L` the lightness the eye sees) out and
//! back, in its polar form too (lightness, chroma, hue), and the difference between two colours
//! as the eye sees it: Oklab's distance and its three parts (D108).

/// An 8-bit sRGB colour.
pub type Rgb8 = [u8; 3];

/// The Oklab chroma under which an 8-bit colour is grey to the eye and its hue angle is only the
/// rounding's (a quarter of a just-noticeable step): where a hue is drawn, never what is judged.
pub const HUELESS: f64 = 0.005;

/// Hue in degrees (`0..360`), saturation and value (`0..=1`), as the colour panel's HSV.
#[must_use]
pub fn hsv(rgb: Rgb8) -> [f64; 3] {
    let [r, g, b] = rgb.map(|c| f64::from(c) / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let chroma = max - min;
    let hue = if chroma <= 0.0 {
        0.0
    } else if max == r {
        60.0 * ((g - b) / chroma).rem_euclid(6.0)
    } else if max == g {
        60.0 * ((b - r) / chroma + 2.0)
    } else {
        60.0 * ((r - g) / chroma + 4.0)
    };
    let saturation = if max <= 0.0 { 0.0 } else { chroma / max };
    [hue, saturation, max]
}

/// The 8-bit sRGB colour of hue `h` (degrees), saturation `s` and value `v`.
#[must_use]
pub fn from_hsv([h, s, v]: [f64; 3]) -> Rgb8 {
    let h = h.rem_euclid(360.0) / 60.0;
    let chroma = v * s;
    let x = chroma * (1.0 - (h % 2.0 - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (chroma, x, 0.0),
        1 => (x, chroma, 0.0),
        2 => (0.0, chroma, x),
        3 => (0.0, x, chroma),
        4 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };
    let m = v - chroma;
    [r, g, b].map(|c| ((c + m).clamp(0.0, 1.0) * 255.0).round() as u8)
}

/// `rgb` in Oklab: lightness `L` (0 to 1) and the opponent axes `a` (green to red) and `b`
/// (blue to yellow).
#[must_use]
pub fn oklab(rgb: Rgb8) -> [f64; 3] {
    oklab_of_linear(rgb.map(|c| linear(f64::from(c) / 255.0)))
}

/// A colour given in linear sRGB (0 to 1 a channel) in Oklab: light mixed, as an eye's blur mixes
/// it (S101's crops).
#[must_use]
pub(crate) fn oklab_of_linear([r, g, b]: [f64; 3]) -> [f64; 3] {
    let l = 0.412_221_470_8 * r + 0.536_332_536_3 * g + 0.051_445_992_9 * b;
    let m = 0.211_903_498_2 * r + 0.680_699_545_1 * g + 0.107_396_956_6 * b;
    let s = 0.088_302_461_9 * r + 0.281_718_837_6 * g + 0.629_978_700_5 * b;
    let [l, m, s] = [l, m, s].map(f64::cbrt);
    [
        0.210_454_255_3 * l + 0.793_617_785_0 * m - 0.004_072_046_8 * s,
        1.977_998_495_1 * l - 2.428_592_205_0 * m + 0.450_593_709_9 * s,
        0.025_904_037_1 * l + 0.782_771_766_2 * m - 0.808_675_766_0 * s,
    ]
}

/// The 8-bit sRGB colour of the Oklab point `[l, a, b]`, each channel clipped to sRGB's range.
#[must_use]
pub fn from_oklab([l, a, b]: [f64; 3]) -> Rgb8 {
    let l_ = l + 0.396_337_777_4 * a + 0.215_803_757_3 * b;
    let m_ = l - 0.105_561_345_8 * a - 0.063_854_172_8 * b;
    let s_ = l - 0.089_484_177_5 * a - 1.291_485_548_0 * b;
    let [l, m, s] = [l_, m_, s_].map(|v| v * v * v);
    [
        4.076_741_662_1 * l - 3.307_711_591_3 * m + 0.230_969_929_2 * s,
        -1.268_438_004_6 * l + 2.609_757_401_1 * m - 0.341_319_396_5 * s,
        -0.004_196_086_3 * l - 0.703_418_614_7 * m + 1.707_614_701_0 * s,
    ]
    .map(|c| (encoded(c.clamp(0.0, 1.0)) * 255.0).round() as u8)
}

/// `rgb` in Oklab's polar form: lightness `L`, chroma (the distance from grey) and hue in degrees
/// (`0..360`, from `a`'s positive axis towards `b`'s: red about 29, yellow 110, green 142, cyan
/// 195, blue 264, magenta 328).
#[must_use]
pub fn oklch(rgb: Rgb8) -> [f64; 3] {
    let [l, a, b] = oklab(rgb);
    [l, a.hypot(b), b.atan2(a).to_degrees().rem_euclid(360.0)]
}

/// The 8-bit sRGB colour of lightness `l`, chroma `c` and hue `h` (degrees) in Oklab, clipped to
/// sRGB's range as `from_oklab`.
#[must_use]
pub fn from_oklch([l, c, h]: [f64; 3]) -> Rgb8 {
    let (sin, cos) = h.to_radians().sin_cos();
    from_oklab([l, c * cos, c * sin])
}

/// Oklab's lightness `L` of a colour given in linear sRGB (0 to 1 a channel): light mixed, as an
/// eye's blur mixes it (S96's value masses).
#[must_use]
pub(crate) fn lightness_of_linear([r, g, b]: [f64; 3]) -> f64 {
    let l = 0.412_221_470_8 * r + 0.536_332_536_3 * g + 0.051_445_992_9 * b;
    let m = 0.211_903_498_2 * r + 0.680_699_545_1 * g + 0.107_396_956_6 * b;
    let s = 0.088_302_461_9 * r + 0.281_718_837_6 * g + 0.629_978_700_5 * b;
    let [l, m, s] = [l, m, s].map(f64::cbrt);
    0.210_454_255_3 * l + 0.793_617_785_0 * m - 0.004_072_046_8 * s
}

/// sRGB's transfer function undone.
pub(crate) fn linear(c: f64) -> f64 {
    if c <= 0.040_45 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// sRGB's transfer function.
fn encoded(c: f64) -> f64 {
    if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

/// The distance between two colours in Oklab: about 0.02 is just noticeable side by side.
#[must_use]
pub fn delta_e(a: Rgb8, b: Rgb8) -> f64 {
    let [a, b] = [oklab(a), oklab(b)];
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// How `copy` differs from `original`, as the eye sees it: Oklab's distance and its three parts,
/// each the copy's less the original's, so `delta² = lightness² + chroma² + hue²`. No colour is
/// too grey for its hue: the hue part is the chord between the hues at the chromas' geometric mean
/// (`2 √(C₀ C₁) sin(Δh / 2)`), so it fades as either colour greys and two dark colours side by
/// side, an olive and a brown, still show their hues apart.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Difference {
    /// The Oklab distance.
    pub delta: f64,
    /// Lighter (positive) or darker: Oklab `L`.
    pub lightness: f64,
    /// Purer (positive) or greyer: Oklab chroma.
    pub chroma: f64,
    /// Round the hues the short way (positive: on from the original's hue the way red goes to
    /// yellow, yellow to green, green to cyan, cyan to blue, blue to magenta, magenta to red).
    pub hue: f64,
}

impl Difference {
    #[must_use]
    pub fn of(original: Rgb8, copy: Rgb8) -> Self {
        let [l0, c0, h0] = oklch(original);
        let [l1, c1, h1] = oklch(copy);
        let turn = ((h1 - h0 + 180.0).rem_euclid(360.0) - 180.0).to_radians();
        Self {
            delta: delta_e(original, copy),
            lightness: l1 - l0,
            chroma: c1 - c0,
            hue: 2.0 * (c0 * c1).sqrt() * (turn / 2.0).sin(),
        }
    }

    /// The largest of the three parts, unsigned.
    #[must_use]
    pub fn largest_part(&self) -> f64 {
        self.lightness
            .abs()
            .max(self.chroma.abs())
            .max(self.hue.abs())
    }
}
