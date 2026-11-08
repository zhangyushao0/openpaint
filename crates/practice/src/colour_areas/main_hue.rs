//! A crop's main hue (S107): where its colours gather on the hue circle, which the packs keep for a
//! crop and a key shift's tendency keeps away from; and the turn between two hues.

use crate::colour_areas::areas::Areas;
use crate::maths::colour::oklch;

/// A tendency's hues, and a main hue, lie on this many degrees' steps.
pub const HUE_STEP: u16 = 10;
/// A crop's colours gathered on the hue circle by share and chroma under this peak give no main
/// hue: any tendency will do.
pub const MAIN_CHROMA: f64 = 0.012;

/// The turn from hue `from` to hue `to`, degrees, the short way round (`-180..180`, positive
/// the way red goes to yellow).
#[must_use]
pub fn turn(from: f64, to: f64) -> f64 {
    (to - from + 180.0).rem_euclid(360.0) - 180.0
}

/// The crop's main hue: where its colours gather on the hue circle, each by its share of the crop
/// and its chroma, near hues together (each counted at `cos²` of its turn off, none past a quarter
/// turn), so a crop of several greens beside a blue is green and opposite colours never cancel;
/// none where even that peak is greyer than `MAIN_CHROMA` (a grey crop). On `HUE_STEP`'s steps.
#[must_use]
pub fn main_hue(areas: &Areas) -> Option<f64> {
    let sizes = areas.sizes();
    let total = sizes.iter().sum::<usize>().max(1) as f64;
    let held: Vec<(f64, [f64; 3])> = (areas.colours.iter().zip(&sizes))
        .map(|(colour, size)| (*size as f64 / total, oklch(*colour)))
        .collect();
    let gathered = |hue: f64| -> f64 {
        (held.iter())
            .map(|(share, [_, chroma, at])| {
                let off = turn(*at, hue).to_radians();
                let near = if off.abs() < std::f64::consts::FRAC_PI_2 {
                    off.cos().powi(2)
                } else {
                    0.0
                };
                share * chroma * near
            })
            .sum()
    };
    let (hue, peak) = (0..360 / HUE_STEP)
        .map(|step| f64::from(step * HUE_STEP))
        .map(|hue| (hue, gathered(hue)))
        .max_by(|x, y| x.1.total_cmp(&y.1))?;
    (peak >= MAIN_CHROMA).then_some(hue)
}
