//! The replay's video made through the whole frame loop beside the screen (S163; the tablet's
//! export that ran out of memory, 2026-11-21), the replay standing half way along a stroke: once
//! it starts, the screen lets its view tiers and raster targets go (its snapshots but the pages
//! that stroke holds), draws nothing new and takes nothing ahead while it is made, however long
//! nothing touches, and shows the replay as it stood after; the GPU memory budget counts the
//! video's target meanwhile; once it ended the video holds nothing, and back in the studio the
//! screen takes its snapshots ahead again.

use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::Instant;

use openpaint_app::{
    Delivery, HostAnswer, HostRequest, ReplayIntent, VideoEncoder, VideoError, VideoOrder,
};
use openpaint_render::CacheBytes;
use openpaint_runtime::{Runtime, ToolKind};

use crate::support::hand::{HANG, Hand};
use crate::support::{Nowhere, canvas, frame, line, start, until};

const SIZE: [u32; 2] = [1280, 1000];
const PIXELS_PER_POINT: f32 = 2.0;
/// The video frame its encoder holds the worker at.
const HELD_AT: u32 = 3;
/// Frames of the screen while the worker is held: two seconds of the hand's clock, nothing
/// touching.
const IDLE_FRAMES: usize = 120;

/// Whether the worker waits in the encoder, and whether it may go on.
#[derive(Debug, Default)]
struct Gate {
    waiting: bool,
    go: bool,
}

/// An encoder holding the video worker at its `HELD_AT`th frame until let go.
#[derive(Clone, Default)]
struct Holding {
    gate: Arc<(Mutex<Gate>, Condvar)>,
    frames: u32,
}

impl Holding {
    fn gate(&self) -> MutexGuard<'_, Gate> {
        self.gate.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn let_go(&self) {
        self.gate().go = true;
        self.gate.1.notify_all();
    }
}

impl VideoEncoder for Holding {
    fn begin(&mut self, _order: &VideoOrder) -> Result<(), VideoError> {
        Ok(())
    }

    fn frame(&mut self, _rgba: &[u8], _at_us: u64) -> Result<(), VideoError> {
        if self.frames == HELD_AT {
            let started = Instant::now();
            let mut gate = self.gate();
            gate.waiting = true;
            while !gate.go && started.elapsed() < HANG {
                gate = (self.gate.1.wait_timeout(gate, HANG))
                    .unwrap_or_else(PoisonError::into_inner)
                    .0;
            }
        }
        self.frames += 1;
        Ok(())
    }

    fn finish(&mut self) -> Result<(), VideoError> {
        Ok(())
    }

    fn cancel(&mut self) {}
}

fn held(runtime: &Runtime) -> CacheBytes {
    runtime.cache_bytes().expect("a canvas")
}

/// Frames until the canvas has no background work left: half a second of the hand's clock after
/// the last touch, the snapshots under the next stroke are taken ahead.
fn ahead(runtime: &mut Runtime, hand: &mut Hand) {
    until(runtime, hand, |runtime| !runtime.canvas_waiting());
}

/// The paused replay's playhead moved along its track to the first place showing a stroke half
/// drawn (its tip shows), as the author's export found it.
fn mid_stroke(runtime: &mut Runtime, hand: &mut Hand) {
    for step in 1..40u8 {
        runtime.replay_intent(ReplayIntent::Seek(f32::from(step) / 40.0));
        frame(runtime, hand);
        let replay = runtime.projection().replay().cloned().expect("the replay");
        if replay.tip.is_some() {
            return;
        }
    }
    panic!("no place of the track shows a stroke half drawn");
}

#[test]
fn while_a_video_is_made_the_screen_holds_its_canvas_alone_and_the_budget_counts_the_video() {
    let (mut runtime, mut hand) = start(PIXELS_PER_POINT, SIZE, Box::new(Nowhere::default()));
    for y in [300.0, 440.0, 580.0] {
        hand.gesture(&mut runtime, ToolKind::Stylus, &line(y));
    }
    ahead(&mut runtime, &mut hand);
    assert!(held(&runtime).snapshots > 0, "{:?}", held(&runtime));
    runtime.replay_intent(ReplayIntent::Open);
    runtime.replay_intent(ReplayIntent::Play(false));
    mid_stroke(&mut runtime, &mut hand);
    ahead(&mut runtime, &mut hand);
    let shown = canvas(&mut runtime).content_hash();
    let before = held(&runtime);
    assert!(before.snapshots > 0, "{before:?}");
    runtime.replay_intent(ReplayIntent::Export(Delivery::Save));
    frame(&mut runtime, &mut hand);
    let asked = runtime.host_request();
    assert!(
        matches!(asked, Some(HostRequest::Video { .. })),
        "{asked:?}"
    );
    let holding = Holding::default();
    runtime.answer(HostAnswer::Video(Box::new(holding.clone())));
    let started = Instant::now();
    while !holding.gate().waiting {
        frame(&mut runtime, &mut hand);
        assert!(
            started.elapsed() < HANG,
            "the worker never reached its frame"
        );
    }
    for _ in 0..IDLE_FRAMES {
        frame(&mut runtime, &mut hand);
    }
    let during = held(&runtime);
    assert_eq!(
        (during.tiers, during.raster),
        (0, 0),
        "the screen took nothing ahead: {during:?}"
    );
    assert!(
        during.snapshots <= before.snapshots,
        "no snapshot taken while the video is made: {during:?} after {before:?}"
    );
    assert!(during.video > 0, "the budget counts the video: {during:?}");
    runtime.replay_intent(ReplayIntent::Cancel);
    holding.let_go();
    until(&mut runtime, &mut hand, |runtime| !runtime.making_video());
    assert_eq!(held(&runtime).video, 0, "the video's target let go");
    assert_eq!(
        canvas(&mut runtime).content_hash(),
        shown,
        "the replay as it stood"
    );
    runtime.replay_intent(ReplayIntent::Close);
    ahead(&mut runtime, &mut hand);
    assert!(
        held(&runtime).snapshots > 0,
        "back in the studio, taken ahead again: {:?}",
        held(&runtime)
    );
}
