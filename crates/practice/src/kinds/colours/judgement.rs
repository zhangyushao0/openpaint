//! The colour judgement of a set of matched colours: the summary's radar (the author's
//! "战斗力图"). Six axes, each from 0 at the centre to 1 at the rim, where every colour was
//! matched exactly; a larger shape is a better eye. Each axis is the mean, over the colours it
//! counts, of a share: 1 for no error, falling evenly to 0 at a span off or more, so one wild
//! match costs no more than a span.
//!
//! - Hue, chroma and lightness: the final colour's parts of its Oklab difference as the eye sees
//!   them (`Difference`, D108), each against `PART_SPAN` (two and a half just-noticeable steps
//!   in that part alone), every colour counted: a grey's hue part is near nothing, as it looks.
//! - Dark and low-saturation colours: the final Oklab difference against `DELTA_SPAN` (five
//!   just-noticeable steps), over the swatches of the dark band and of the grey and muted bands:
//!   the colours V大 says are judged worst.
//! - First pick to final: the share of the first pick's difference the corrections took away; a
//!   first pick within `NOTICEABLE` counts as 1 (nothing was left to correct).
//!
//! Every value comes from the numbers a result stores at the first Compare. Results of 1.6.0
//! and before hold their parts on the wheel's HSV (codes retired), so they count in the last
//! three axes only.

use crate::colour_match::{Lightness, Purity, Tone};
use crate::metric::{Metric, value};

/// One of a judgement's axes, in the radar's order (clockwise from the top).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Axis {
    Hue,
    Chroma,
    Lightness,
    Dark,
    LowSaturation,
    FirstToFinal,
}

impl Axis {
    pub const ALL: [Self; 6] = [
        Self::Hue,
        Self::Chroma,
        Self::Lightness,
        Self::Dark,
        Self::LowSaturation,
        Self::FirstToFinal,
    ];
}

/// The Oklab difference an error falls to 0 at, and the one that is just noticeable.
pub const DELTA_SPAN: f64 = 0.10;
pub const NOTICEABLE: f64 = 0.02;
/// The Oklab lightness, chroma or hue part an error falls to 0 at.
pub const PART_SPAN: f64 = 0.05;

/// Each axis's value, `0..=1`, in `Axis::ALL`'s order; `None` where no colour counts.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Judgement(pub [Option<f64>; 6]);

impl Judgement {
    #[must_use]
    pub fn of(&self, axis: Axis) -> Option<f64> {
        let index = Axis::ALL.iter().position(|a| *a == axis)?;
        self.0[index]
    }

    /// True when no axis has a value.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.iter().all(Option::is_none)
    }
}

/// The judgement of `colours`: each matched swatch's tone and stored numbers.
#[must_use]
pub fn judgement(colours: &[(Tone, &[(Metric, f64)])]) -> Judgement {
    Judgement(Axis::ALL.map(|axis| {
        let shares: Vec<f64> = colours
            .iter()
            .filter_map(|(tone, metrics)| share(axis, *tone, metrics))
            .collect();
        (!shares.is_empty()).then(|| shares.iter().sum::<f64>() / shares.len() as f64)
    }))
}

/// What one colour gives `axis`, if it counts there.
fn share(axis: Axis, tone: Tone, metrics: &[(Metric, f64)]) -> Option<f64> {
    let off = |metric: Metric, span: f64| value(metrics, metric).map(|v| fall(v.abs(), span));
    match axis {
        Axis::Hue => off(Metric::FinalHue, PART_SPAN),
        Axis::Chroma => off(Metric::FinalChroma, PART_SPAN),
        Axis::Lightness => off(Metric::FinalLightness, PART_SPAN),
        Axis::Dark => (tone.lightness == Lightness::Dark)
            .then(|| off(Metric::FinalDelta, DELTA_SPAN))
            .flatten(),
        Axis::LowSaturation => matches!(tone.purity, Purity::Grey | Purity::Muted)
            .then(|| off(Metric::FinalDelta, DELTA_SPAN))
            .flatten(),
        Axis::FirstToFinal => {
            let first = value(metrics, Metric::FirstDelta)?;
            let last = value(metrics, Metric::FinalDelta)?;
            Some(if first <= NOTICEABLE {
                1.0
            } else {
                ((first - last) / first).clamp(0.0, 1.0)
            })
        }
    }
}

/// 1 for no `error`, evenly to 0 at `span`.
fn fall(error: f64, span: f64) -> f64 {
    (1.0 - error / span).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DARK_GREY: Tone = Tone {
        purity: Purity::Grey,
        lightness: Lightness::Dark,
    };
    const LIGHT_VIVID: Tone = Tone {
        purity: Purity::Vivid,
        lightness: Lightness::Light,
    };

    #[test]
    fn exact_matches_reach_the_rim_and_errors_fall_evenly_to_the_centre() {
        let exact = [
            (Metric::FirstDelta, 0.0),
            (Metric::FinalDelta, 0.0),
            (Metric::FinalHue, 0.0),
            (Metric::FinalChroma, 0.0),
            (Metric::FinalLightness, 0.0),
        ];
        let all = judgement(&[(DARK_GREY, &exact[..])]);
        assert_eq!(all, Judgement([Some(1.0); 6]));
        let off = [
            (Metric::FirstDelta, 0.08),
            (Metric::FinalDelta, 0.04),
            (Metric::FinalHue, -0.025),
            (Metric::FinalChroma, 0.06),
            (Metric::FinalLightness, -0.01),
        ];
        let judged = judgement(&[(LIGHT_VIVID, &off[..])]);
        let near = |axis, expected: f64| {
            let got = judged.of(axis).expect("a value");
            assert!((got - expected).abs() < 1e-9, "{axis:?}: {got}");
        };
        near(Axis::Hue, 0.5);
        near(Axis::Chroma, 0.0);
        near(Axis::Lightness, 0.8);
        near(Axis::FirstToFinal, 0.5);
        assert_eq!(judged.of(Axis::Dark), None, "a light swatch");
        assert_eq!(judged.of(Axis::LowSaturation), None, "a vivid one");
    }

    #[test]
    fn each_axis_is_the_mean_of_the_colours_it_counts() {
        let a = [(Metric::FinalDelta, 0.0), (Metric::FirstDelta, 0.01)];
        let b = [(Metric::FinalDelta, 0.05), (Metric::FirstDelta, 0.04)];
        let judged = judgement(&[(DARK_GREY, &a[..]), (DARK_GREY, &b[..])]);
        let near = |axis, expected: f64| {
            let got = judged.of(axis).expect("a value");
            assert!((got - expected).abs() < 1e-12, "{axis:?}: {got}");
        };
        near(Axis::Dark, 0.75);
        near(Axis::LowSaturation, 0.75);
        // The first within a noticeable step: 1; the second worse after: 0.
        near(Axis::FirstToFinal, 0.5);
        assert_eq!(judged.of(Axis::Hue), None, "no hue part stored");
        assert!(judgement(&[]).is_empty());
    }
}
