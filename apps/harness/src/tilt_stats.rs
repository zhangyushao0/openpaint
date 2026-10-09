//! The tilt line of `stroke-stats` (S44): how far the pen leaned from upright over every sample
//! of a library's pen strokes, so we learn whether the author's pen reports tilt in practice and
//! where a brush's tilt curve should start to respond. Strokes of one constant pressure (injected
//! by device routines) are left out, as for the other statistics.

use openpaint_core::Stroke;

/// "tilt over N samples of M strokes: min, p50, p90, max degrees; the share at exactly 0".
pub fn tilt_line<'a>(strokes: impl Iterator<Item = &'a Stroke>) -> String {
    let drawn: Vec<&Stroke> = strokes
        .filter(|stroke| {
            let samples = stroke.samples();
            samples.len() > 1
                && samples
                    .iter()
                    .any(|s| s.pressure() != samples[0].pressure())
        })
        .collect();
    let mut degrees: Vec<f32> = drawn
        .iter()
        .flat_map(|stroke| stroke.samples().iter().map(|s| s.tilt().to_degrees()))
        .collect();
    if degrees.is_empty() {
        return "tilt: no pen samples".to_owned();
    }
    let upright = degrees.iter().filter(|degrees| **degrees == 0.0).count();
    degrees.sort_by(f32::total_cmp);
    let at = |share: f32| degrees[((degrees.len() - 1) as f32 * share) as usize];
    format!(
        "tilt over {} samples of {} pen strokes (degrees from upright): min {:.1}, p50 {:.1}, \
         p90 {:.1}, max {:.1}; {:.1} % at exactly 0",
        degrees.len(),
        drawn.len(),
        at(0.0),
        at(0.5),
        at(0.9),
        at(1.0),
        upright as f64 * 100.0 / degrees.len() as f64
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use openpaint_core::{
        Artwork, BrushRecipe, CanvasPoint, CanvasSize, Color, StrokeSample, StrokeStyle,
    };

    use super::tilt_line;

    /// An artwork of one stroke per `(pressure step, tilts in degrees)`.
    fn artwork(strokes: &[(f32, &[f32])]) -> Artwork {
        let style = StrokeStyle::new(Arc::new(BrushRecipe::plain()), 4.0, 1.0, Color::BLACK);
        let style = style.expect("style");
        let mut artwork = Artwork::new(CanvasSize::new(64, 64).expect("size"));
        for (step, tilts) in strokes {
            let samples: Vec<StrokeSample> = (0u16..)
                .zip(tilts.iter())
                .map(|(i, tilt)| {
                    let at = CanvasPoint::new(f32::from(i), 4.0).expect("point");
                    let pressure = 0.5 + step * f32::from(i);
                    StrokeSample::new(at, pressure, tilt.to_radians(), 0.0, u32::from(i))
                })
                .collect::<Result<_, _>>()
                .expect("samples");
            let mut session = artwork
                .begin_stroke(style.clone(), samples[0])
                .expect("begin");
            session.extend(&samples[1..]).expect("extend");
            artwork = session.finish().artwork;
        }
        artwork
    }

    #[test]
    fn the_line_gives_the_tilts_quantiles_and_the_share_upright() {
        let tilts = [0.0, 0.0, 30.0, 40.0, 50.0, 60.0, 35.0, 45.0, 55.0, 70.0];
        // The second stroke has one constant pressure: injected, left out.
        let drawn = artwork(&[(0.01, &tilts), (0.0, &[80.0, 80.0, 80.0])]);
        let line = tilt_line(drawn.layers()[0].strokes().iter());
        assert_eq!(
            line,
            "tilt over 10 samples of 1 pen strokes (degrees from upright): min 0.0, p50 40.0, \
             p90 60.0, max 70.0; 20.0 % at exactly 0"
        );
        let none = artwork(&[(0.0, &[10.0, 10.0])]);
        assert_eq!(
            tilt_line(none.layers()[0].strokes().iter()),
            "tilt: no pen samples"
        );
    }
}
