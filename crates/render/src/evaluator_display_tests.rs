//! The screen's canvas at ten bits (S37): it reads, picks and reduces to thumbnails exactly as
//! an 8-bit target of the same artwork, and holds more levels than one where paint is soft.

use std::sync::Arc;

use openpaint_core::{
    Artwork, BrushRecipe, CanvasPoint, CanvasSize, Color, Curve, Dynamics, Grain, StrokeSample,
    StrokeStyle,
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
