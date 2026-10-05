//! Strokes follow the curve through their samples (S34): a sparse arc keeps to its circle at 8x,
//! resamples on the chord are no knots, an accumulating stroke lays evenly along the curve, a
//! stroke drawn sample by sample ends as the finished stroke evaluated cold, and straight
//! strokes keep their pixels.

use std::sync::Arc;

use openpaint_core::{Artwork, BrushRecipe, Color, StrokeSample, StrokeStyle};
use openpaint_render::{CanvasTarget, Core, FrameRequest, PixelRect, RegionEvaluator, RgbaImage};

use crate::fixture::{
    SIZE, airbrush, blank, cold, core, graphite, pixel, plain, request, sample, with_stroke,
};
use crate::scale::{difference, whole};

const CENTRE: [f32; 2] = [160.0, 100.0];
const RADIUS: f32 = 80.0;
/// Degrees between the arc's samples: chords about 28 px, 1.2 px off the circle at the middle.
const STEP_DEG: f32 = 20.0;

/// An arc of `count` samples on the circle, every `STEP_DEG`, pressure rising slightly; with
/// `resamples`, one more sample in the middle of every other chord with the pressure before it,
/// as Android adds one per frame.
fn arc(count: u16, resamples: bool) -> Vec<StrokeSample> {
    let at = |i: f32| {
        let angle = (i * STEP_DEG).to_radians();
        [
            CENTRE[0] + RADIUS * angle.cos(),
            CENTRE[1] + RADIUS * angle.sin(),
        ]
    };
    let mut samples = Vec::new();
    for i in 0..count {
        let k = f32::from(i);
        let pressure = 0.8 + 0.01 * k;
        let [x, y] = at(k);
        samples.push(sample(x, y, pressure, u32::from(i) * 4));
        if resamples && i % 2 == 0 && i + 1 < count {
            let [nx, ny] = at(k + 1.0);
            samples.push(sample(
                (x + nx) / 2.0,
                (y + ny) / 2.0,
                pressure,
                u32::from(i) * 4 + 2,
            ));
        }
    }
    samples
}

fn ink(size: f32) -> StrokeStyle {
    plain(size, 1.0, Color::BLACK)
}

/// The radius from `CENTRE`, in canvas pixels, where the stroke's outer edge crosses half
/// coverage along the angle `degrees`, in `image` at `scale` (black ink on white).
fn outer_edge(image: &RgbaImage, scale: u32, degrees: f32) -> f32 {
    let (cos, sin) = (degrees.to_radians().cos(), degrees.to_radians().sin());
    let step = 1.0 / (4.0 * scale as f32);
    let mut r = RADIUS;
    while r < RADIUS + 20.0 {
        let x = (CENTRE[0] + r * cos) * scale as f32;
        let y = (CENTRE[1] + r * sin) * scale as f32;
        if pixel(image, [x as u32, y as u32])[0] > 127 {
            return r;
        }
        r += step;
    }
    r
}

#[test]
fn a_sparse_arc_keeps_to_its_circle_at_eight_x_between_its_samples() {
    let core = core();
    let artwork = with_stroke(&blank(), (ink(8.0), arc(15, true)));
    let image = whole(&core, &artwork, 8);
    // Away from the ends, at the samples and halfway between them (where the chords lay 1.2 px
    // inside the circle), the edge lies on the circle widened by the radius.
    for k in 2..12u16 {
        for half in [0.0, 0.5] {
            let degrees = (f32::from(k) + half) * STEP_DEG;
            let edge = outer_edge(&image, 8, degrees);
            // The plain brush's radius is half its size times the pressure.
            let pressure = 0.8 + 0.01 * degrees / STEP_DEG;
            let off = edge - (RADIUS + 4.0 * pressure);
            assert!(
                off.abs() <= 0.15,
                "at {degrees} deg the edge is {off} px off the circle"
            );
        }
    }
}

/// Draws `samples` as an in-progress stroke one sample at a time on one canvas target; after
/// every sample its pixels equal a fresh target's (the stroke evaluated cold with its tail).
/// Returns the target and the finished artwork.
fn drawn_sample_by_sample(
    core: &Core,
    style: &StrokeStyle,
    samples: &[StrokeSample],
) -> (CanvasTarget, Artwork) {
    let base = blank();
    let mut target = CanvasTarget::new(core, SIZE).expect("target");
    RegionEvaluator::new(core, &mut target)
        .evaluate(&request(&base), &[PixelRect::full(SIZE)])
        .expect("evaluate");
    let mut session = base.begin_stroke(style.clone(), samples[0]).expect("begin");
    for end in 1..=samples.len() {
        if end > 1 {
            session.extend(&samples[end - 1..end]).expect("extend");
        }
        let frame = FrameRequest {
            active: Some(session.active()),
            ..request(&base)
        };
        RegionEvaluator::new(core, &mut target)
            .evaluate(&frame, &[])
            .expect("evaluate");
        if end % 5 == 0 {
            let mut fresh = CanvasTarget::new(core, SIZE).expect("target");
            RegionEvaluator::new(core, &mut fresh)
                .evaluate(&frame, &[PixelRect::full(SIZE)])
                .expect("evaluate");
            let (kept, cold) = (target.read(core), fresh.read(core));
            let (kept, cold) = (kept.expect("read"), cold.expect("read"));
            let differing = kept
                .pixels()
                .iter()
                .zip(cold.pixels())
                .filter(|(a, b)| a != b);
            assert_eq!(differing.count(), 0, "after {end} samples");
        }
    }
    let finished = session.finish().artwork;
    RegionEvaluator::new(core, &mut target)
        .evaluate(&request(&finished), &[])
        .expect("evaluate");
    (target, finished)
}

#[test]
fn a_curve_drawn_sample_by_sample_ends_as_the_finished_stroke_cold() {
    let core = core();
    let recipes = [
        Arc::new(BrushRecipe::plain()),
        crate::fixture::rich(),
        graphite(),
        airbrush(),
    ];
    for recipe in recipes {
        let style = StrokeStyle::new(recipe, 14.0, 0.8, Color::BLACK).expect("style");
        let samples = arc(17, true);
        let (target, finished) = drawn_sample_by_sample(&core, &style, &samples);
        let drawn = target.read(&core).expect("read");
        assert!(
            drawn == cold(&core, &finished),
            "committed = the last frame = cold"
        );
    }
}

#[test]
fn an_accumulating_stroke_lays_evenly_along_its_curve() {
    let core = core();
    let style = StrokeStyle::new(airbrush(), 12.0, 0.6, Color::BLACK).expect("style");
    // Constant pressure, so only the path changes along it.
    let samples: Vec<StrokeSample> = arc(15, true)
        .into_iter()
        .map(|s| {
            let p = s.position();
            sample(p.x(), p.y(), 0.8, s.time_ms())
        })
        .collect();
    let artwork = with_stroke(&blank(), (style, samples));
    let image = whole(&core, &artwork, 4);
    let along: Vec<u8> = (60..=220)
        .map(|tenth| {
            let angle = (tenth as f32 / 10.0 * STEP_DEG / 2.0).to_radians();
            let x = (CENTRE[0] + RADIUS * angle.cos()) * 4.0;
            let y = (CENTRE[1] + RADIUS * angle.sin()) * 4.0;
            pixel(&image, [x as u32, y as u32])[0]
        })
        .collect();
    let (low, high) = (along.iter().min(), along.iter().max());
    let spread = high.zip(low).map_or(255, |(h, l)| h - l);
    assert!(
        spread <= 2,
        "the centre line ranges {spread} levels: {along:?}"
    );
}

#[test]
fn straight_strokes_keep_their_pixels_and_resamples_change_none() {
    let core = core();
    // Collinear, one pressure: every other sample is a resample on the chord.
    let line = |resamples: bool| -> Vec<StrokeSample> {
        (0..=20u32)
            .filter(|i| resamples || i % 2 == 0)
            .map(|i| sample(40.0 + 10.0 * i as f32, 60.0 + 4.0 * i as f32, 0.7, i * 4))
            .collect()
    };
    let with = cold(&core, &with_stroke(&blank(), (ink(10.0), line(true))));
    let without = cold(&core, &with_stroke(&blank(), (ink(10.0), line(false))));
    let (largest, mean) = difference(with.pixels(), without.pixels());
    assert_eq!((largest, mean), (0, 0.0), "the same knots");
    // Two samples are their straight segment at every scale: 8x reduced comes back to 1x.
    let two = with_stroke(
        &blank(),
        (
            ink(10.0),
            vec![sample(30.0, 40.0, 0.5, 0), sample(260.0, 170.0, 1.0, 40)],
        ),
    );
    let (one, eight) = (cold(&core, &two), whole(&core, &two, 8));
    let back = crate::scale::reduced(&eight, 8);
    let (largest, mean) = difference(one.pixels(), &back);
    assert!(
        largest <= 16 && mean < 0.05,
        "largest {largest}, mean {mean}"
    );
}
