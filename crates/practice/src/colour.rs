//! Colours as the colour exercises judge them: 8-bit sRGB in, HSV (hue, saturation and value as
//! the colour panel shows them) and Oklab (Björn Ottosson's perceptual space: equal steps look
//! about equally different) out, and the differences between two colours in both.

/// An 8-bit sRGB colour.
pub type Rgb8 = [u8; 3];

/// Below this chroma (max channel minus min channel, 0 to 1) a colour is too grey for its hue to
/// mean anything.
pub const LEAST_CHROMA: f64 = 0.06;

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

/// The chroma of `rgb`: its largest channel minus its smallest, 0 to 1.
#[must_use]
pub fn chroma(rgb: Rgb8) -> f64 {
    let max = rgb.iter().max().copied().unwrap_or(0);
    let min = rgb.iter().min().copied().unwrap_or(0);
    f64::from(max - min) / 255.0
}

/// `rgb` in Oklab: lightness `L` (0 to 1) and the opponent axes `a` (green to red) and `b`
/// (blue to yellow).
#[must_use]
pub fn oklab(rgb: Rgb8) -> [f64; 3] {
    let [r, g, b] = rgb.map(|c| linear(f64::from(c) / 255.0));
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

/// sRGB's transfer function undone.
fn linear(c: f64) -> f64 {
    if c <= 0.040_45 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// The distance between two colours in Oklab: about 0.02 is just noticeable side by side.
#[must_use]
pub fn delta_e(a: Rgb8, b: Rgb8) -> f64 {
    let [a, b] = [oklab(a), oklab(b)];
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// How `copy` differs from `original`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Difference {
    /// The Oklab distance.
    pub delta: f64,
    /// Degrees from the original's hue to the copy's the short way round (positive: on round
    /// the wheel from red towards yellow); `None` when either is too grey (`LEAST_CHROMA`).
    pub hue: Option<f64>,
    /// The copy's saturation and value less the original's.
    pub saturation: f64,
    pub value: f64,
}

impl Difference {
    #[must_use]
    pub fn of(original: Rgb8, copy: Rgb8) -> Self {
        let [h0, s0, v0] = hsv(original);
        let [h1, s1, v1] = hsv(copy);
        let coloured = chroma(original) >= LEAST_CHROMA && chroma(copy) >= LEAST_CHROMA;
        Self {
            delta: delta_e(original, copy),
            hue: coloured.then(|| (h1 - h0 + 180.0).rem_euclid(360.0) - 180.0),
            saturation: s1 - s0,
            value: v1 - v0,
        }
    }
}

/// The name of the hue `degrees` lies nearest, of twelve round the wheel.
#[must_use]
pub fn hue_name(degrees: f64) -> &'static str {
    const NAMES: [&str; 12] = [
        "red",
        "orange",
        "yellow",
        "yellow-green",
        "green",
        "green-cyan",
        "cyan",
        "azure",
        "blue",
        "violet",
        "magenta",
        "rose",
    ];
    let index = ((degrees.rem_euclid(360.0) + 15.0) / 30.0) as usize % NAMES.len();
    NAMES[index]
}
