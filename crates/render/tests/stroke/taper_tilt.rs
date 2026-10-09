//! Taper and tilt (S44): an Inker line's width follows its taper at 5, 50 and 95 % of its length;
//! drawn sample by sample, every frame is the stroke so far without its end taper and the frame
//! at the lift is the committed stroke, cold; tilt widens and lightens by its curves; a monoline
//! keeps one width at any pressure; tapered strokes export in bands as in one piece.

use std::sync::Arc;

use openpaint_core::{
    Accumulation, Artwork, BrushRecipe, Color, Curve, Deposit, Dynamics, Grain, StrokeSample,
    StrokeStyle, Taper,
};
use openpaint_render::{
    Banding, CanvasTarget, FrameRequest, PixelRect, RegionEvaluator, RgbaImage,
};

use crate::fixture::{SIZE, blank, cold, core, graphite, pixel, request, with_stroke};
use crate::scale::{export, whole};

/// The Inker's taper (`crates/app/brushes/inker.toml`): diameters at the start and the end, and
/// the share of the radius at the tips.
const INKER: [f32; 3] = [2.0, 3.0, 0.1];
/// The Pencil's size and opacity over tilt (`crates/app/brushes/pencil.toml`).
const WIDER: [[f32; 2]; 3] = [[0.0, 1.0], [60.0, 1.0], [75.0, 3.0]];
const LIGHTER: [[f32; 2]; 3] = [[0.0, 1.0], [60.0, 1.0], [75.0, 0.25]];

fn curve(points: &[[f32; 2]]) -> Curve {
    Curve::new(points).expect("curve")
}

/// The Ink Pen's pressure width with `taper`.
fn inker(taper: [f32; 3]) -> Arc<BrushRecipe> {
    let [start, end, tip] = taper;
    let taper = Taper::new(start, end, tip).expect("taper");
    Arc::new(BrushRecipe::plain().with_taper(taper))
}

/// A sample at `[x, y]`, pressure `pressure`, `tilt` degrees from upright.
fn at([x, y]: [f32; 2], pressure: f32, tilt: f32, time_ms: u32) -> StrokeSample {
    let position = openpaint_core::CanvasPoint::new(x, y).expect("point");
    StrokeSample::new(position, pressure, tilt.to_radians(), 0.0, time_ms).expect("sample")
}

/// A level line along y = 100.5 from x = 20 to 300, a sample every 2 px; the pressure falls by
/// a ten-thousandth per sample from `pressure` so every sample is a knot.
fn level(pressure: f32, tilt: f32) -> Vec<StrokeSample> {
    (0..=140u16)
        .map(|i| {
            let x = 20.0 + 2.0 * f32::from(i);
            at(
                [x, 100.5],
                pressure - 0.0001 * f32::from(i),
                tilt,
                4 * u32::from(i),
            )
        })
        .collect()
}

/// The coverage of black on white summed down column `x`: the line's width there in pixels.
fn width(image: &RgbaImage, x: u32) -> f32 {
    (0..SIZE[1])
        .map(|y| f32::from(255 - pixel(image, [x, y])[0]) / 255.0)
        .sum()
}

#[test]
fn an_inker_lines_width_follows_its_taper_at_five_fifty_and_ninety_five_percent() {
    let core = core();
    let style = StrokeStyle::new(inker(INKER), 40.0, 1.0, Color::BLACK).expect("style");
    let samples = level(1.0, 0.0);
    let image = cold(
        &core,
        &with_stroke(&blank(), (style.clone(), samples.clone())),
    );
    let [start, end, tip] = INKER;
    let taper = Taper::new(start, end, tip).expect("taper");
    let length = 280.0;
    for share in [0.05f32, 0.5, 0.95] {
        let along = share * length;
        let x = 20.0 + along;
        // The pressure at that place, as the samples carry it.
        let pressure = 1.0 - 0.0001 * along / 2.0;
        let factor = taper.factor(along, Some(length - along), 40.0);
        let expected = 40.0 * pressure * factor;
        let measured = width(&image, x as u32);
        eprintln!("inker at {share}: {measured:.2} px wide for {expected:.2}");
        assert!(
            (measured - expected).abs() <= 1.0,
            "at {share}: {measured} px wide for {expected}"
        );
        if share == 0.5 {
            assert_eq!(factor, 1.0, "the middle untouched");
        }
    }
    // Past the start's two diameters and before the end's three the line is full width.
    let full = width(&image, 20 + 81);
    assert!((full - 40.0).abs() <= 1.0, "{full}");
    assert!(
        width(&image, 22) < 6.0 && width(&image, 298) < 6.0,
        "fine points at both ends"
    );
}

/// `style` drawn through `samples` one sample at a time on one canvas target: after every fifth
/// sample the frame equals the stroke so far evaluated cold as a finished stroke of `drawn` (the
/// same brush without its end taper); at the lift the target equals the committed stroke cold.
fn drawn_with_a_blunt_end(style: &StrokeStyle, drawn: &StrokeStyle, samples: &[StrokeSample]) {
    let core = core();
    let base = blank();
    let mut target = CanvasTarget::new(&core, SIZE).expect("target");
    RegionEvaluator::new(&core, &mut target)
        .evaluate(&request(&base), &[PixelRect::full(SIZE)])
        .expect("evaluate");
    let mut session = base.begin_stroke(style.clone(), samples[0]).expect("begin");
    let mut last = None;
    for end in 1..=samples.len() {
        if end > 1 {
            session.extend(&samples[end - 1..end]).expect("extend");
        }
        let frame = FrameRequest {
            active: Some(session.active()),
            ..request(&base)
        };
        RegionEvaluator::new(&core, &mut target)
            .evaluate(&frame, &[])
            .expect("evaluate");
        if end % 5 == 0 || end == samples.len() {
            let so_far = with_stroke(&base, (drawn.clone(), samples[..end].to_vec()));
            let shown = target.read(&core).expect("read");
            assert!(shown == cold(&core, &so_far), "after {end} samples: blunt");
            last = Some(shown);
        }
    }
    let finished: Artwork = session.finish().artwork;
    RegionEvaluator::new(&core, &mut target)
        .evaluate(&request(&finished), &[])
        .expect("evaluate");
    let lifted = target.read(&core).expect("read");
    assert!(
        lifted == cold(&core, &finished),
        "the lift's frame = the commit, cold"
    );
    assert!(Some(lifted) != last, "the lift narrows the end");
}

#[test]
fn a_tapered_stroke_shows_its_end_taper_from_the_lift_and_equals_the_commit_cold() {
    let style = StrokeStyle::new(inker(INKER), 24.0, 0.8, Color::BLACK).expect("style");
    let [start, _, tip] = INKER;
    let blunt = StrokeStyle::new(inker([start, 0.0, tip]), 24.0, 0.8, Color::BLACK);
    // A curve with rising and falling pressure, as a hand draws it.
    let samples: Vec<StrokeSample> = (0..60u16)
        .map(|i| {
            let t = f32::from(i) / 59.0;
            let x = 30.0 + 260.0 * t;
            let y = 100.0 + 60.0 * (std::f32::consts::TAU * t).sin();
            let pressure = 0.3 + 0.7 * (std::f32::consts::PI * t).sin();
            at([x, y], pressure, 0.0, 6 * u32::from(i))
        })
        .collect();
    drawn_with_a_blunt_end(&style, &blunt.expect("style"), &samples);
}

/// A flat tip building up density without paper, sized and flowing over tilt as the Pencil.
fn tilting_flat_tip() -> Arc<BrushRecipe> {
    let flat = || curve(&[[0.0, 1.0]]);
    let size = Dynamics::new(flat(), flat())
        .and_then(|size| size.with_tilt(Curve::gain(&WIDER).expect("wider")));
    let opacity = Dynamics::new(flat(), flat()).and_then(|o| o.with_tilt(curve(&LIGHTER)));
    let recipe = BrushRecipe::new(
        0.0,
        size.expect("size"),
        opacity.expect("opacity"),
        Grain::NONE,
    );
    let deposit = Accumulation::new(openpaint_core::Tip::Flat, 0.3, 1.0, None).expect("deposit");
    let recipe = recipe
        .expect("recipe")
        .with_deposit(Deposit::Accumulate(deposit));
    Arc::new(recipe.expect("accumulating"))
}

/// Down column `x`: the density at the centre row (100) and the density summed across, from
/// coverage `1 - exp(-density)` of black on white.
fn densities(image: &RgbaImage, x: u32) -> [f32; 2] {
    let density = |y: u32| {
        let coverage = f32::from(255 - pixel(image, [x, y])[0]) / 255.0;
        -(1.0 - coverage).ln()
    };
    [density(100), (0..SIZE[1]).map(density).sum()]
}

#[test]
fn tilt_widens_and_lightens_by_the_curves_factors() {
    let core = core();
    let style = StrokeStyle::new(tilting_flat_tip(), 20.0, 1.0, Color::BLACK).expect("style");
    let image = |tilt: f32| {
        cold(
            &core,
            &with_stroke(&blank(), (style.clone(), level(0.5, tilt))),
        )
    };
    let (upright, tilted) = (image(0.0), image(67.5));
    // At 67.5 degrees: halfway along the curves from 60 to 75 degrees.
    let (wider, lighter) = (2.0, 0.625);
    let mut ratios = [0.0f32; 2];
    for x in (120..200).step_by(10) {
        let [centre_u, across_u] = densities(&upright, x);
        let [centre_t, across_t] = densities(&tilted, x);
        // Density at the centre follows the flow alone; summed across, flow times radius.
        ratios[0] += centre_t / centre_u / 8.0;
        ratios[1] += (across_t / centre_t) / (across_u / centre_u) / 8.0;
    }
    eprintln!(
        "tilted 67.5 degrees: density by {}, width by {}",
        ratios[0], ratios[1]
    );
    assert!(
        (ratios[0] - lighter).abs() < 0.02,
        "density by {}",
        ratios[0]
    );
    assert!((ratios[1] - wider).abs() < 0.04, "width by {}", ratios[1]);
}

/// The rows a column's paint reaches: the first to the last not white.
fn extent(image: &RgbaImage, x: u32) -> u32 {
    let painted: Vec<u32> = (0..SIZE[1])
        .filter(|y| pixel(image, [x, *y])[0] < 255)
        .collect();
    painted
        .last()
        .zip(painted.first())
        .map_or(0, |(l, f)| l - f + 1)
}

#[test]
fn the_pencil_tilted_draws_wider_and_lighter_than_upright() {
    let core = core();
    let pencil = Arc::unwrap_or_clone(graphite());
    let size = pencil
        .size()
        .clone()
        .with_tilt(Curve::gain(&WIDER).expect("wider"));
    let opacity = pencil.opacity().clone().with_tilt(curve(&LIGHTER));
    let tilting = BrushRecipe::new(
        0.0,
        size.expect("size"),
        opacity.expect("opacity"),
        Grain::NONE,
    )
    .and_then(|recipe| recipe.with_deposit(pencil.deposit().clone()))
    .expect("pencil");
    let style = StrokeStyle::new(Arc::new(tilting), 16.0, 1.0, Color::BLACK).expect("style");
    let image = |tilt: f32| {
        cold(
            &core,
            &with_stroke(&blank(), (style.clone(), level(0.6, tilt))),
        )
    };
    let (upright, tilted) = (image(0.0), image(67.5));
    let ink = |image: &RgbaImage, x: u32| -> f32 {
        (0..SIZE[1])
            .map(|y| f32::from(255 - pixel(image, [x, y])[0]))
            .sum()
    };
    let (mut wide, mut narrow, mut light, mut dark) = (0, 0, 0.0, 0.0);
    for x in (100..220).step_by(4) {
        wide += extent(&tilted, x);
        narrow += extent(&upright, x);
        light += ink(&tilted, x) / extent(&tilted, x) as f32;
        dark += ink(&upright, x) / extent(&upright, x) as f32;
    }
    let ratio = wide as f32 / narrow as f32;
    eprintln!(
        "pencil tilted: {ratio} times as wide, {} as dark per row",
        light / dark
    );
    assert!(
        (ratio - 2.0).abs() < 0.2,
        "the paint reaches {ratio} times as far"
    );
    assert!(
        light < 0.8 * dark,
        "lighter: {light} against {dark} per row"
    );
}

#[test]
fn a_monoline_keeps_one_width_at_any_pressure() {
    let core = core();
    let flat = || curve(&[[0.0, 1.0]]);
    let dynamics = || Dynamics::new(flat(), flat()).expect("dynamics");
    let recipe = BrushRecipe::new(0.0, dynamics(), dynamics(), Grain::NONE).expect("recipe");
    let style = StrokeStyle::new(Arc::new(recipe), 16.0, 1.0, Color::BLACK).expect("style");
    let image = |pressure: f32| {
        cold(
            &core,
            &with_stroke(&blank(), (style.clone(), level(pressure, 0.0))),
        )
    };
    let (light, hard) = (image(0.1), image(1.0));
    assert!(light == hard, "pressure changes nothing");
    for x in [40, 160, 280] {
        let (at_light, at_hard) = (width(&light, x), width(&hard, x));
        eprintln!("monoline at {x}: {at_light:.2} px at pressure 0.1, {at_hard:.2} at 1.0");
        assert!((at_light - 16.0).abs() <= 0.5, "at {x}: {at_light}");
    }
}

#[test]
fn a_tapered_stroke_exports_in_bands_as_in_one_piece() {
    let core = core();
    let style = StrokeStyle::new(inker(INKER), 30.0, 0.9, Color::BLACK).expect("style");
    let artwork = with_stroke(&blank(), (style, level(0.8, 0.0)));
    assert_eq!(
        export(&core, &artwork, 1, Banding::DEFAULT),
        cold(&core, &artwork)
    );
    for scale in [2, 4] {
        let banding = Banding {
            rows: 37 * scale,
            columns: 61 * scale + 5,
        };
        let banded = export(&core, &artwork, scale, banding);
        assert_eq!(banded, whole(&core, &artwork, scale), "{scale}x");
    }
}
