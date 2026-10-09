//! The screen's canvas at ten bits (S37): it reads, picks and reduces to thumbnails exactly as
//! an 8-bit target of the same artwork, and holds more levels than one where paint is soft. A
//! steep curve over a soft ramp (S43) steps by one level and one code, not by its slope.

use std::sync::Arc;

use openpaint_core::{
    Adjustment, Artwork, BrushRecipe, CanvasPoint, CanvasSize, Color, Curve, Curves, Dynamics,
    Grain, StrokeSample, StrokeStyle, ToneChannel, ToneCurve,
};

use crate::core::{Core, test_core};
use crate::gpu::TEN_BITS;
use crate::readback::read_buffer;
use crate::{CanvasTarget, FrameRequest, PixelRect, RegionEvaluator};

const SIZE: [u32; 2] = [160, 96];

/// A wide, faint, fully soft black stroke across the middle on white: a gentle ramp from the
/// paper down a dozen levels over some 40 rows each side.
fn soft_ramp() -> Artwork {
    let flat = || Curve::new(&[[0.0, 1.0]]).expect("curve");
    let dynamics = || Dynamics::new(flat(), flat()).expect("dynamics");
    let recipe = BrushRecipe::new(1.0, dynamics(), dynamics(), Grain::NONE).expect("recipe");
    let style = StrokeStyle::new(Arc::new(recipe), 84.0, 0.05, Color::BLACK).expect("style");
    let sample = |x: f32, time_ms| {
        StrokeSample::new(
            CanvasPoint::new(x, 48.0).expect("point"),
            1.0,
            0.0,
            0.0,
            time_ms,
        )
        .expect("sample")
    };
    let artwork = Artwork::new(CanvasSize::new(SIZE[0], SIZE[1]).expect("size"));
    let mut session = artwork
        .begin_stroke(style, sample(-60.0, 0))
        .expect("begin");
    session.extend(&[sample(220.0, 16)]).expect("extend");
    session.finish().artwork
}

fn evaluated(core: &Core, mut target: CanvasTarget, artwork: &Artwork) -> CanvasTarget {
    let request = FrameRequest {
        artwork,
        active: None,
        background: Color::WHITE,
    };
    RegionEvaluator::new(core, &mut target)
        .evaluate(&request, &[PixelRect::full(SIZE)])
        .expect("evaluated");
    target
}

/// The target's texels as the GPU holds them, one word each, row by row.
fn words(core: &Core, target: &CanvasTarget) -> Vec<u32> {
    let texture = target.texture();
    let row = (SIZE[0] * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    let buffer = core.gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("raw words"),
        size: u64::from(row * SIZE[1]),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let mut encoder = core.gpu.device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: Some(SIZE[1]),
            },
        },
        texture.size(),
    );
    core.gpu.queue.submit([encoder.finish()]);
    let bytes = read_buffer(&core.gpu, &buffer, u64::from(row * SIZE[1])).expect("read");
    bytes
        .chunks_exact(row as usize)
        .flat_map(|padded| padded[..(SIZE[0] * 4) as usize].as_chunks::<4>().0)
        .map(|word| u32::from_le_bytes(*word))
        .collect()
}

#[test]
fn the_ten_bit_canvas_reads_picks_and_reduces_as_eight_bits_with_more_levels() {
    let core = test_core();
    let artwork = soft_ramp();
    let shown = evaluated(
        core,
        CanvasTarget::new(core, SIZE).expect("target"),
        &artwork,
    );
    let exact = evaluated(
        core,
        CanvasTarget::exact(core, SIZE).expect("target"),
        &artwork,
    );
    let eight = exact.read(core).expect("read");
    assert_eq!(shown.read(core).expect("read").pixels(), eight.pixels());
    for at in [[80, 48], [80, 20], [80, 75], [3, 90], [159, 0]] {
        let picked = shown.pick(core, at).expect("pick");
        assert_eq!(picked, exact.pick(core, at).expect("pick"), "{at:?}");
    }
    let thumbnail = |target| core.thumbnail(target, [40, 24]).expect("thumbnail");
    assert_eq!(thumbnail(&shown).pixels(), thumbnail(&exact).pixels());
    if core.gpu.display != TEN_BITS {
        eprintln!("this device shows the canvas at 8 bits: nothing more to compare");
        return;
    }
    assert_eq!(shown.texture().format(), TEN_BITS);
    // Down the middle column: the distinct 8-bit values and ten-bit codes of red.
    let column = |values: Vec<u32>| {
        let mut seen: Vec<u32> = (0..SIZE[1])
            .map(|y| values[(y * SIZE[0] + 80) as usize])
            .collect();
        seen.sort_unstable();
        seen.dedup();
        seen.len()
    };
    let levels = column(
        eight
            .pixels()
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| u32::from(p[0]))
            .collect(),
    );
    let codes = column(words(core, &shown).iter().map(|word| word & 1023).collect());
    assert!(levels >= 8, "a ramp of {levels} levels");
    assert!(codes >= 2 * levels, "{codes} codes over {levels} levels");
}

/// A curve rising with slope 4 from 0.45 to 0.55 (exactly straight there: the tangents equal
/// the slopes), gentle outside it.
fn steep_curve() -> ToneCurve {
    ToneCurve::new(&[
        [0.0, 0.0],
        [0.4, 0.12],
        [0.45, 0.32],
        [0.5, 0.52],
        [0.55, 0.72],
        [0.6, 0.92],
        [1.0, 1.0],
    ])
    .expect("curve")
}

/// The tables the evaluator gets for `curve` over every channel.
fn tables(curve: ToneCurve) -> [[u8; 256]; 3] {
    Curves::default().with(ToneChannel::Composite, curve).luts()
}

fn stroked(artwork: &Artwork, style: StrokeStyle, [from, to]: [[f32; 2]; 2]) -> Artwork {
    let sample = |[x, y]: [f32; 2], time_ms| {
        let at = CanvasPoint::new(x, y).expect("point");
        StrokeSample::new(at, 1.0, 0.0, 0.0, time_ms).expect("sample")
    };
    let mut session = artwork.begin_stroke(style, sample(from, 0)).expect("begin");
    session.extend(&[sample(to, 16)]).expect("extend");
    session.finish().artwork
}

/// An opaque grey layer (sRGB 135) darkened towards the left by a wide, faint, fully soft black
/// stroke just off the canvas: along each row a gentle ramp of about four levels, evaluated in
/// floats, then `steep_curve` over it, so the canvas rises about 16 levels from left to right.
fn ramp_through_a_steep_curve() -> Artwork {
    let flat = || Curve::new(&[[0.0, 1.0]]).expect("curve");
    let dynamics = || Dynamics::new(flat(), flat()).expect("dynamics");
    let recipe = |softness| {
        let recipe = BrushRecipe::new(softness, dynamics(), dynamics(), Grain::NONE);
        Arc::new(recipe.expect("recipe"))
    };
    let level = 135.0 / 255.0;
    let grey = Color::rgb(level, level, level).expect("grey");
    let base = StrokeStyle::new(recipe(0.0), 400.0, 1.0, grey).expect("style");
    let shade = StrokeStyle::new(recipe(1.0), 340.0, 0.03, Color::BLACK).expect("style");
    let artwork = Artwork::new(CanvasSize::new(SIZE[0], SIZE[1]).expect("size"));
    let artwork = stroked(&artwork, base, [[-100.0, 48.0], [260.0, 48.0]]);
    let artwork = stroked(&artwork, shade, [[-10.0, -100.0], [-10.0, 200.0]]);
    let curves = Curves::default().with(ToneChannel::Composite, steep_curve());
    let applied = artwork.adjust_region(None, Adjustment::Curves(curves));
    applied.expect("adjusted").artwork
}

/// The middle row of `values` (one per pixel), its largest step between neighbours and
/// whether it never falls from left to right.
fn middle_row(values: &[u32]) -> (Vec<u32>, u32, bool) {
    let width = SIZE[0] as usize;
    let row = values[width * 48..width * 49].to_vec();
    let largest = row.windows(2).map(|w| w[0].abs_diff(w[1])).max();
    let rising = row.windows(2).all(|w| w[0] <= w[1]);
    (row.clone(), largest.unwrap_or(0), rising)
}

#[test]
fn a_steep_curve_over_a_soft_ramp_steps_by_one_level_and_one_code() {
    let core = test_core();
    let artwork = ramp_through_a_steep_curve();
    let exact = evaluated(
        core,
        CanvasTarget::exact(core, SIZE).expect("target"),
        &artwork,
    );
    let eight = exact.read(core).expect("read");
    let reds: Vec<u32> = (eight.pixels().as_chunks::<4>().0.iter())
        .map(|p| u32::from(p[0]))
        .collect();
    let (levels, step, rising) = middle_row(&reds);
    eprintln!("8-bit row: largest step {step} levels: {levels:?}");
    assert!(levels[159] - levels[0] >= 12, "stretched by the curve");
    assert!(rising, "monotone");
    assert!(step <= 1, "steps of {step} levels");
    if core.gpu.display != TEN_BITS {
        eprintln!("this device shows the canvas at 8 bits: nothing more to compare");
        return;
    }
    let shown = evaluated(
        core,
        CanvasTarget::new(core, SIZE).expect("target"),
        &artwork,
    );
    let codes: Vec<u32> = words(core, &shown).iter().map(|word| word & 1023).collect();
    let (codes, step, rising) = middle_row(&codes);
    eprintln!("ten-bit row: largest step {step} codes: {codes:?}");
    assert!(rising, "monotone");
    assert!(step <= 1, "steps of {step} codes");
}

/// The shader's lookup (`curved` in `adjust.wgsl`) on the CPU: between the two entries around
/// `x`, by its fraction.
fn interpolated(table: &[u8; 256], x: f32) -> f32 {
    let scaled = x.clamp(0.0, 1.0) * 255.0;
    let below = scaled.floor().min(254.0);
    let at = below as usize;
    let (a, b) = (f32::from(table[at]), f32::from(table[at + 1]));
    (a + (b - a) * (scaled - below)) / 255.0
}

#[test]
fn interpolated_tables_keep_within_a_level_of_the_curve() {
    let lifted = ToneCurve::new(&[[0.0, 0.1], [0.3, 0.8], [1.0, 0.9]]).expect("curve");
    let mut seed = 0x2545_f491_u32;
    for (curve, nearest_off) in [
        (steep_curve(), 1.9),
        (lifted, 0.9),
        (ToneCurve::identity(), 0.4),
    ] {
        let table = tables(curve.clone())[0];
        let mut worst = (0.0f32, 0.0f32);
        for _ in 0..4096 {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            let x = (seed >> 8) as f32 / (1 << 24) as f32;
            let exact = curve.value(x) * 255.0;
            let between = interpolated(&table, x) * 255.0;
            let nearest = f32::from(table[(x * 255.0 + 0.5) as usize]);
            worst.0 = worst.0.max((between - exact).abs());
            worst.1 = worst.1.max((nearest - exact).abs());
        }
        assert!(worst.0 <= 1.0, "{} levels off between entries", worst.0);
        // The nearest entry alone is off by up to half the slope (S28's lookup).
        assert!(
            worst.1 >= nearest_off,
            "{} levels off at the nearest",
            worst.1
        );
    }
}
