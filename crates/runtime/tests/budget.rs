//! The GPU memory budget (S49, S58) through the whole frame loop: with every derived cache
//! holding something (two view tiers, the stroke's snapshot, a smudge in the arena, the layer
//! pictures), a budget lowered step by step lets them go by recency, the least recently used
//! first (the arena's material, used by the smudge drawn first, then the snapshots, the
//! pictures, the tier left), never the tier shown, and past their share the residuals (S51); the
//! canvas itself is never let go and stays equal to a cold evaluation throughout, also for a
//! smudge drawn over a residual let go. A zoom round from 2x to 8x and back evaluates no tier
//! tile twice: the tiers left are kept (S58).

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
/// the view tier fills in; returns the tier tiles filled after the fingers lifted.
fn pinch(runtime: &mut Runtime, hand: &mut Hand, from: [[f32; 2]; 2], to: [[f32; 2]; 2]) -> u64 {
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
    let lifted = runtime.core().counts().filled;
    settle(runtime, hand);
    runtime.core().counts().filled - lifted
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

/// Two fingers `apart` pixels apart about the screen's middle, moving apart (or together) until
/// `ratio` times that, then frames until the view tier fills in; returns the tier tiles filled
/// once the fingers lifted (while they move, the tier shown fills what the view newly shows).
fn zoom_by(runtime: &mut Runtime, hand: &mut Hand, apart: f32, ratio: f32) -> u64 {
    let at = |half: f32| [[800.0 - half, 900.0], [800.0 + half, 900.0]];
    pinch(runtime, hand, at(apart / 2.0), at(apart * ratio / 2.0))
}

/// The stroke `points` drawn on a clock held a second on from the last.
fn draw(runtime: &mut Runtime, hand: &mut Hand, points: &[[f32; 2]]) {
    hand.clock += 1000;
    runtime.fix_clock(hand.clock);
    hand.gesture(runtime, ToolKind::Stylus, points);
    settle(runtime, hand);
}

#[test]
fn a_lower_budget_lets_the_least_recently_used_go_and_never_what_the_frame_needs() {
    let core = gpu::core();
    let mut runtime = support::start(PIXELS_PER_POINT, SIZE, Box::new(Nowhere));
    let mut hand = Hand::new(SIZE);
    draw(&mut runtime, &mut hand, &line(600.0));
    // A smudge across it: its residual in the arena, its material beside it (used last now).
    hand.tap(&mut runtime, "Smudge");
    draw(&mut runtime, &mut hand, &line(610.0));
    // A stroke over both: its frames take the layer's snapshot, kept after it.
    hand.tap(&mut runtime, "Brushes");
    hand.tap(&mut runtime, "Brushes");
    draw(&mut runtime, &mut hand, &line(620.0));
    // The layer pictures made while the panel shows, then the panel closed.
    hand.tap(&mut runtime, "Layers");
    settle(&mut runtime, &mut hand);
    hand.tap(&mut runtime, "Layers");
    settle(&mut runtime, &mut hand);
    // Zoomed in about three times, then twice more: the 2x tier left, the 4x tier shown.
    zoom_by(&mut runtime, &mut hand, 200.0, 3.0);
    zoom_by(&mut runtime, &mut hand, 400.0, 2.0);
    let all = held(&runtime);
    eprintln!("caches before: {all:?}");
    assert!(
        all.tiers > 0 && all.snapshots > 0 && all.arena > 0 && all.pictures > 0,
        "{all:?}"
    );
    assert_cold(&core, &mut runtime, "every cache held");
    // Just below what they hold: the least recently used goes, the smudge's material.
    let after = budget(&mut runtime, &mut hand, all.total() - 1);
    assert!(after.arena < all.arena, "{after:?}");
    let kept = (after.tiers, after.snapshots, after.pictures);
    assert_eq!(kept, (all.tiers, all.snapshots, all.pictures));
    assert_cold(&core, &mut runtime, "the arena cut down");
    // Then the snapshots, the stroke drawn after the smudge.
    let before = after;
    let after = budget(&mut runtime, &mut hand, before.total() - 1);
    assert_eq!(after.snapshots, 0, "{after:?}");
    assert_eq!((after.tiers, after.pictures), (all.tiers, all.pictures));
    assert_cold(&core, &mut runtime, "without the snapshots");
    // Then the pictures, made after the strokes.
    let after = budget(&mut runtime, &mut hand, after.total() - 1);
    assert_eq!((after.pictures, after.tiers), (0, all.tiers), "{after:?}");
    assert_cold(&core, &mut runtime, "without the pictures");
    // Then the 2x tier the view left; the 4x tier it shows stays however low the budget, and so
    // does the canvas.
    let after = budget(&mut runtime, &mut hand, after.total() - 1);
    assert!(after.tiers > 0 && after.tiers < all.tiers, "{after:?}");
    let shown = after.tiers;
    let after = budget(&mut runtime, &mut hand, 1 << 20);
    assert_eq!(
        (after.tiers, after.canvas),
        (shown, all.canvas),
        "{after:?}"
    );
    assert_cold(&core, &mut runtime, "only the tier shown");
    // S51: past their share of the budget the residuals go (not one the last frame needed); a
    // smudge drawn across the place evaluates again the one it reads.
    assert!(after.arena < all.arena, "{after:?}");
    hand.tap(&mut runtime, "Smudge");
    draw(&mut runtime, &mut hand, &line(605.0));
    assert_cold(&core, &mut runtime, "a smudge over a residual let go");
}

#[test]
fn a_zoom_round_to_8x_and_back_evaluates_no_tier_tile_twice() {
    let core = gpu::core();
    let mut runtime = support::start(PIXELS_PER_POINT, SIZE, Box::new(Nowhere));
    let mut hand = Hand::new(SIZE);
    for y in [500.0, 700.0, 900.0, 1100.0] {
        draw(&mut runtime, &mut hand, &line(y));
    }
    let inwards = [
        zoom_by(&mut runtime, &mut hand, 200.0, 2.2),
        zoom_by(&mut runtime, &mut hand, 300.0, 2.0),
        zoom_by(&mut runtime, &mut hand, 300.0, 2.0),
    ];
    assert!(inwards.iter().all(|filled| *filled > 0), "{inwards:?}");
    let back = [
        zoom_by(&mut runtime, &mut hand, 600.0, 0.5),
        zoom_by(&mut runtime, &mut hand, 600.0, 0.5),
    ];
    eprintln!("tier tiles filled: {inwards:?} going in, {back:?} coming back");
    assert_eq!(back, [0, 0], "the 4x and 2x tiers were kept");
    assert_cold(&core, &mut runtime, "after the round");
}
