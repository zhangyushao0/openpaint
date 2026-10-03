//! GPU hashes of the three built-in brushes: each draws one fixed wave, which must evaluate to
//! fixed pixels (the same on Vulkan and DX12), and drawn a few samples per frame it must equal
//! the finished stroke evaluated cold. Here because the brushes are the app's files and the
//! evaluator is the renderer's.

use std::f32::consts::{PI, TAU};
use std::sync::Arc;

use openpaint_app::BrushLibrary;
use openpaint_core::{
    Artwork, CanvasPoint, CanvasSize, Color, StrokeSample, StrokeSession, StrokeStyle,
};
use openpaint_render::{CanvasTarget, Core, FrameRequest, PixelRect, RegionEvaluator, RgbaImage};

const SIZE: [u32; 2] = [320, 200];
/// Per brush key: the wave's golden hash. Re-recorded in S16 for the pencil (draweave's tuned
/// graphite on the sketch paper) and the soft brush (density building up under a bell tip);
/// the brushes before keep their hashes below.
const GOLDEN: [(&str, u64); 3] = [
    ("pencil", 0x0b09_3229_5b55_3b12),
    ("ink-pen", 0xa8c1_7ea8_d659_a519),
    ("soft-brush", 0x91ac_be03_f571_da3e),
];

/// An S-curve whose pressure rises from 0.1 to 1 and falls back, drawn faster in its middle,
/// at `size` with `brush`'s recipe, 80 % opaque.
fn wave(library: &BrushLibrary, key: &str, size: f32) -> (StrokeStyle, Vec<StrokeSample>) {
    let index = library.find(key).expect("a built-in brush");
    let recipe = Arc::clone(library.brushes()[index].recipe());
    let color = Color::rgb(0.12, 0.25, 0.65).expect("colour");
    let style = StrokeStyle::new(recipe, size, 0.8, color).expect("style");
    let mut time = 0;
    let samples = (0..64u16)
        .map(|i| {
            let t = f32::from(i) / 63.0;
            let position = CanvasPoint::new(30.0 + 260.0 * t, 100.0 + 55.0 * (TAU * t).sin());
            let pressure = 0.1 + 0.9 * (PI * t).sin();
            time += 2 + u32::from(i % 16 == 0) * 10;
            StrokeSample::new(position.expect("point"), pressure, 0.0, 0.0, time).expect("sample")
        })
        .collect();
    (style, samples)
}

fn blank() -> Artwork {
    Artwork::new(CanvasSize::new(SIZE[0], SIZE[1]).expect("size"))
}

fn begin((style, samples): &(StrokeStyle, Vec<StrokeSample>)) -> StrokeSession {
    blank()
        .begin_stroke(style.clone(), samples[0])
        .expect("begin")
}

fn request(artwork: &Artwork) -> FrameRequest<'_> {
    FrameRequest {
        artwork,
        active: None,
        background: Color::WHITE,
    }
}

fn cold(core: &Core, artwork: &Artwork) -> RgbaImage {
    let mut target = CanvasTarget::new(core, SIZE).expect("target");
    RegionEvaluator::new(core, &mut target)
        .evaluate(&request(artwork), &[PixelRect::full(SIZE)])
        .expect("evaluate");
    target.read(core).expect("readback")
}

/// The stroke drawn three samples per frame as an in-progress stroke, then committed.
fn incremental(core: &Core, stroke: &(StrokeStyle, Vec<StrokeSample>)) -> (Artwork, RgbaImage) {
    let base = blank();
    let mut target = CanvasTarget::new(core, SIZE).expect("target");
    let mut evaluator = RegionEvaluator::new(core, &mut target);
    evaluator
        .evaluate(&request(&base), &[PixelRect::full(SIZE)])
        .expect("evaluate");
    let mut session = begin(stroke);
    for chunk in stroke.1[1..].chunks(3) {
        let frame = FrameRequest {
            active: Some(session.active()),
            ..request(&base)
        };
        evaluator.evaluate(&frame, &[]).expect("evaluate");
        session.extend(chunk).expect("extend");
    }
    let finished = session.finish().artwork;
    evaluator
        .evaluate(&request(&finished), &[])
        .expect("evaluate");
    (finished, target.read(core).expect("readback"))
}

#[test]
fn every_built_in_brush_draws_its_golden_wave_and_incremental_equals_cold() {
    let library = BrushLibrary::built_in().expect("built-in brushes");
    draws_the_goldens(&library, &GOLDEN);
}

/// The pencil and the soft brush as S11 to S15 shipped them, whose recipes the strokes drawn
/// then keep (D07): they draw exactly the pixels they drew then.
const BEFORE_S16: [(&str, &str); 2] = [
    (
        "pencil",
        "name = \"Pencil\"\n[size]\ndefault = 5\nmin = 1\nmax = 40\n\
         pressure = [[0.0, 0.45], [1.0, 1.0]]\n[opacity]\ndefault = 1.0\n\
         pressure = [[0.0, 0.15], [0.8, 0.9], [1.0, 1.0]]\nspeed = [[0.0, 1.0], [6.0, 0.7]]\n\
         [grain]\nstrength = 0.65\nscale = 3\n",
    ),
    (
        "soft-brush",
        "name = \"Soft Brush\"\n[shape]\nsoftness = 1.0\n[size]\ndefault = 120\nmin = 8\n\
         max = 600\npressure = [[0.0, 0.8], [1.0, 1.0]]\n[opacity]\ndefault = 0.8\n\
         pressure = [[0.0, 0.05], [1.0, 1.0]]\n",
    ),
];
/// Their golden hashes before S16, unchanged.
const GOLDEN_BEFORE_S16: [(&str, u64); 2] = [
    ("pencil", 0xc7db_069e_23a2_2e48),
    ("soft-brush", 0x9e52_723c_6f49_00f9),
];

#[test]
fn strokes_of_the_brushes_before_s16_draw_exactly_as_they_did() {
    let library = BrushLibrary::from_files(&BEFORE_S16, "pencil").expect("the old brushes");
    draws_the_goldens(&library, &GOLDEN_BEFORE_S16);
}

fn draws_the_goldens(library: &BrushLibrary, goldens: &[(&str, u64)]) {
    let core = Core::headless().expect("a GPU adapter is required for runtime tests");
    let mut hashes = Vec::new();
    for &(key, golden) in goldens {
        let stroke = wave(library, key, 40.0);
        let (finished, drawn) = incremental(&core, &stroke);
        let image = cold(&core, &finished);
        assert!(drawn == image, "{key}: incremental differs from cold");
        hashes.push((key, image.content_hash(), golden));
    }
    let all: Vec<String> = hashes
        .iter()
        .map(|(key, hash, _)| format!("{key} {hash:016x}"))
        .collect();
    let adapter = core.adapter();
    for (key, hash, golden) in hashes {
        assert_eq!(hash, golden, "{key}: hashes {all:?} on {adapter}");
    }
}
