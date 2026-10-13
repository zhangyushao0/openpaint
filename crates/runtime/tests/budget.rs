//! The GPU memory budget (S49) through the whole frame loop: with every derived cache holding
//! something (a view tier zoomed in, the stroke's snapshot, a smudge in the arena, the layer
//! pictures), a budget lowered step by step drops them in the stated order, the view tiers
//! first, then the snapshots, then the arena's blocks and material, then the layer pictures, and
//! past their share the residuals (S51); the canvas itself is never dropped and stays equal to a
//! cold evaluation throughout, also for a smudge drawn over a residual let go.

mod gpu;
mod hand;
mod support;

use hand::{Hand, assert_cold};
use openpaint_render::CacheBytes;
use openpaint_runtime::{
    Buttons, MotionBatch, MotionPhase, Pointer, PointerSample, Runtime, ToolKind,
};
use support::Nowhere;

/// 800 x 700 points at density 2: the layer panel fits.
const SIZE: [u32; 2] = [1600, 1400];
const PIXELS_PER_POINT: f32 = 2.0;

/// Right of the side bar.
fn line(y: f32) -> Vec<[f32; 2]> {
    (0..=16u8)
        .map(|i| [300.0 + 50.0 * f32::from(i), y])
        .collect()
}

/// Two fingers moving apart from `from` to `to` (a pinch out), then frames on a held clock until
/// the view tier fills in.
fn pinch(runtime: &mut Runtime, hand: &mut Hand, from: [[f32; 2]; 2], to: [[f32; 2]; 2]) {
    let at = |step: u8| -> Vec<Pointer> {
        let t = f32::from(step) / 12.0;
        (0..)
            .zip(from.iter().zip(&to))
            .map(|(id, (a, b))| Pointer {
                id,
                tool: ToolKind::Finger,
                samples: vec![PointerSample {
                    position: [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t],
                    pressure: 1.0,
                    tilt: 0.0,
                    orientation: 0.0,
                    time_ms: hand.clock + i64::from(step) * 16,
                }],
            })
            .collect()
    };
    let phases = [MotionPhase::Down, MotionPhase::PointerDown { index: 1 }];
    let mut batches: Vec<(MotionPhase, Vec<Pointer>)> = (0..2)
        .map(|index| (phases[index], at(0)[..=index].to_vec()))
        .collect();
    batches.extend((1..12).map(|step| (MotionPhase::Move, at(step))));
    batches.push((MotionPhase::PointerUp { index: 1 }, at(12)));
    batches.push((MotionPhase::Up, at(12)[..1].to_vec()));
    for (phase, pointers) in batches {
        hand.clock += 16;
        runtime.fix_clock(hand.clock);
        let batch = MotionBatch {
            phase,
            buttons: Buttons::default(),
            pointers,
        };
        runtime.motion(&batch);
        runtime.frame_offscreen(SIZE).expect("frame");
    }
    settle(runtime, hand);
}

/// Frames 16 ms apart on a held clock until nothing moves or fills in.
fn settle(runtime: &mut Runtime, hand: &mut Hand) {
    for _ in 0..400 {
        hand.clock += 16;
        runtime.fix_clock(hand.clock);
        runtime.frame_offscreen(SIZE).expect("frame");
        if !runtime.moving() && !runtime.storing() {
            return;
        }
    }
}

fn held(runtime: &Runtime) -> CacheBytes {
    runtime.cache_bytes().expect("a canvas")
}

/// Sets the budget, runs a frame or two, and returns what the caches hold then.
fn budget(runtime: &mut Runtime, hand: &mut Hand, bytes: u64) -> CacheBytes {
    runtime.set_memory_budget(bytes);
    settle(runtime, hand);
    held(runtime)
}

#[test]
fn a_lower_budget_drops_tiers_then_snapshots_then_the_arena_then_pictures() {
    let core = gpu::core();
    let mut runtime = support::start(PIXELS_PER_POINT, SIZE, Box::new(Nowhere));
    let mut hand = Hand::new(SIZE);
    // Two strokes on one place: the second's frames take the layer's snapshot, kept after it.
    for y in [600.0, 620.0] {
        hand.gesture(&mut runtime, ToolKind::Stylus, &line(y));
    }
    // A smudge across them: its residual in the arena, its material beside it.
    hand.tap(&mut runtime, "Smudge");
    hand.gesture(&mut runtime, ToolKind::Stylus, &line(610.0));
    hand.tap(&mut runtime, "Layers");
    settle(&mut runtime, &mut hand);
    // Zoomed in about three times: a view tier.
    pinch(
        &mut runtime,
        &mut hand,
        [[700.0, 900.0], [900.0, 900.0]],
        [[500.0, 900.0], [1100.0, 900.0]],
    );
    let all = held(&runtime);
    eprintln!("caches before: {all:?}");
    assert!(
        all.tiers > 0 && all.snapshots > 0 && all.arena > 0 && all.pictures > 0,
        "{all:?}"
    );
    assert_cold(&core, &runtime, "every cache held");
    // Just below what they hold: the tiers go, nothing else.
    let after = budget(&mut runtime, &mut hand, all.total() - 1);
    assert_eq!(after.tiers, 0, "{after:?}");
    assert_eq!(
        (after.snapshots, after.arena, after.pictures),
        (all.snapshots, all.arena, all.pictures)
    );
    assert_cold(&core, &runtime, "without the tiers");
    // Below what the snapshots need too: they go.
    let after = budget(
        &mut runtime,
        &mut hand,
        all.canvas + all.pictures + all.arena + all.snapshots - 1,
    );
    assert_eq!((after.tiers, after.snapshots), (0, 0), "{after:?}");
    assert_eq!((after.arena, after.pictures), (all.arena, all.pictures));
    assert_cold(&core, &runtime, "without the snapshots");
    // Below the arena: it is cut down to the blocks the scene shows.
    let after = budget(
        &mut runtime,
        &mut hand,
        all.canvas + all.pictures + all.arena - 1,
    );
    assert!(after.arena < all.arena, "{after:?}");
    assert_eq!(after.pictures, all.pictures);
    assert_cold(&core, &runtime, "the arena cut down");
    // Below the pictures: they go; the canvas never does.
    let after = budget(&mut runtime, &mut hand, all.canvas);
    assert_eq!(after.pictures, 0, "{after:?}");
    assert_eq!(after.canvas, all.canvas);
    assert_cold(&core, &runtime, "without the pictures");
    // S51: past their share of the budget the residuals go (not one the last frame needed); a
    // smudge drawn across the place evaluates again the one it reads.
    let after = budget(&mut runtime, &mut hand, 1 << 20);
    assert!(after.arena < all.arena, "{after:?}");
    assert_cold(&core, &runtime, "the residuals let go");
    hand.tap(&mut runtime, "Layers");
    hand.gesture(&mut runtime, ToolKind::Stylus, &line(605.0));
    settle(&mut runtime, &mut hand);
    assert_cold(&core, &runtime, "a smudge over a residual let go");
}
