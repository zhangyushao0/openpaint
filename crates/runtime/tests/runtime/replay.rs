//! GPU tests of the replay on the open canvas and of its video's path (S163), through the whole
//! frame loop: Actions offers it by its strokes; while it is open the pen draws nothing and the
//! replay ends on the artwork's very pixels; closed, the canvas is the artwork's again and undo
//! still works; the speed chosen is the next replay's. The video's path hands the encoder every
//! frame in order at its time, each the replay at its moment through the video's own scale (the
//! end the last moment again, or the card with the line), and finishes; Cancel and an encoder's
//! failure drop what it began.
//!
//! These hold with part A's stubs (S163a: no steps, every frame the finished artwork, an empty
//! timeline, a flow that asks the host for nothing) and with its bodies; what needs those (blank
//! paper at opening, the playhead moving, Export asking for an encoder) the scenarios walk.

use std::sync::atomic::{AtomicBool, AtomicU32};
use std::sync::{Arc, Mutex, PoisonError};

use openpaint_app::{
    Availability, Delivery, PortError, ReplayIntent, ReplayShown, Speed, Timeline, VideoEncoder,
    VideoError, VideoFrame, VideoFrames, VideoJob, VideoOrder,
};
use openpaint_core::{
    Artwork, BrushRecipe, CanvasPoint, CanvasSize, Color, StrokeSample, StrokeStyle,
};
use openpaint_render::{CanvasTarget, FrameRequest, RgbaImage, UiFrame, video_reference};
use openpaint_runtime::{Runtime, ToolKind, make_video};

use crate::support::hand::{HANG, Hand};
use crate::support::{Nowhere, canvas, core, frame, line, settle, start, strokes};

const SIZE: [u32; 2] = [1280, 1000];
const PIXELS_PER_POINT: f32 = 2.0;

/// Frames until the open replay's playhead reaches the end (it stops there on the artwork).
fn to_the_end(runtime: &mut Runtime, hand: &mut Hand) {
    let started = std::time::Instant::now();
    loop {
        frame(runtime, hand);
        let replay = runtime
            .projection()
            .replay()
            .cloned()
            .expect("the replay shows");
        if replay.at_ms >= replay.total_ms {
            return;
        }
        assert!(started.elapsed() < HANG, "the replay never reached its end");
    }
}

#[test]
fn the_replay_shows_instead_and_closing_leaves_the_artwork_as_it_was() {
    let (mut runtime, mut hand) = start(PIXELS_PER_POINT, SIZE, Box::new(Nowhere::default()));
    for y in [300.0, 440.0] {
        hand.gesture(&mut runtime, ToolKind::Stylus, &line(y));
    }
    settle(&mut runtime, &mut hand);
    let artwork = runtime.artwork().expect("artwork");
    let starts: Vec<Option<u64>> = (artwork.layers().iter())
        .flat_map(|layer| layer.strokes().iter().map(|stroke| stroke.began_ms()))
        .collect();
    let on_the_utc_clock = |start: &Option<u64>| start.is_some_and(|ms| ms > 1_700_000_000_000);
    assert!(
        starts.iter().all(on_the_utc_clock),
        "each stroke keeps when it began: {starts:?}"
    );
    let drawn = canvas(&mut runtime).content_hash();
    let offered = ReplayShown::Offered {
        strokes: 2,
        availability: Availability::Available,
    };
    assert_eq!(*runtime.projection().replay_shown(), offered);
    runtime.replay_intent(ReplayIntent::Open);
    to_the_end(&mut runtime, &mut hand);
    assert_eq!(
        canvas(&mut runtime).content_hash(),
        drawn,
        "it ends on the artwork"
    );
    hand.gesture(&mut runtime, ToolKind::Stylus, &line(580.0));
    assert_eq!(
        strokes(&runtime),
        2,
        "the pen draws nothing while it is open"
    );
    runtime.replay_intent(ReplayIntent::Close);
    settle(&mut runtime, &mut hand);
    assert_eq!(*runtime.projection().replay_shown(), offered);
    assert_eq!(
        canvas(&mut runtime).content_hash(),
        drawn,
        "the artwork as it was"
    );
    hand.tap(&mut runtime, "Undo");
    assert_eq!(strokes(&runtime), 1, "undo still works");
}

#[test]
fn the_speed_chosen_is_the_next_replays() {
    let (mut runtime, mut hand) = start(PIXELS_PER_POINT, SIZE, Box::new(Nowhere::default()));
    hand.gesture(&mut runtime, ToolKind::Stylus, &line(300.0));
    runtime.replay_intent(ReplayIntent::Open);
    to_the_end(&mut runtime, &mut hand);
    let speed = |runtime: &Runtime| runtime.projection().replay().map(|replay| replay.speed);
    assert_eq!(speed(&runtime), Some(Speed::Four), "4x at first");
    runtime.replay_intent(ReplayIntent::Speed(Speed::Sixteen));
    runtime.replay_intent(ReplayIntent::Close);
    settle(&mut runtime, &mut hand);
    assert_eq!(speed(&runtime), None);
    runtime.replay_intent(ReplayIntent::Open);
    to_the_end(&mut runtime, &mut hand);
    assert_eq!(speed(&runtime), Some(Speed::Sixteen));
}

/// What the encoder was asked, in order.
#[derive(Debug, Clone, PartialEq)]
enum Call {
    Begin(VideoOrder),
    Frame(u64),
    Finish,
    Cancel,
}

/// An encoder keeping its calls and frames; failing at its first frame when `fails`.
#[derive(Clone, Default)]
struct Keeper {
    calls: Arc<Mutex<Vec<Call>>>,
    frames: Arc<Mutex<Vec<Vec<u8>>>>,
    fails: bool,
}

impl Keeper {
    fn calls(&self) -> Vec<Call> {
        self.calls
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn call(&self, call: Call) {
        self.calls
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(call);
    }
}

impl VideoEncoder for Keeper {
    fn begin(&mut self, order: &VideoOrder) -> Result<(), VideoError> {
        self.call(Call::Begin(order.clone()));
        Ok(())
    }

    fn frame(&mut self, rgba: &[u8], at_us: u64) -> Result<(), VideoError> {
        self.call(Call::Frame(at_us));
        if self.fails {
            return Err(VideoError::Write(PortError::Device {
                detail: "no space left (test)".into(),
            }));
        }
        let mut frames = self.frames.lock().unwrap_or_else(PoisonError::into_inner);
        frames.push(rgba.to_vec());
        Ok(())
    }

    fn finish(&mut self) -> Result<(), VideoError> {
        self.call(Call::Finish);
        Ok(())
    }

    fn cancel(&mut self) {
        self.call(Call::Cancel);
    }
}

const CANVAS: [u32; 2] = [320, 200];
const VIDEO: [u32; 2] = [192, 120];

/// Two strokes on a small canvas.
fn artwork() -> Artwork {
    let style = |[r, g, b]: [f32; 3]| {
        let color = Color::rgb(r, g, b).expect("colour");
        StrokeStyle::new(Arc::new(BrushRecipe::plain()), 18.0, 0.8, color).expect("style")
    };
    let mut artwork = Artwork::new(CanvasSize::new(CANVAS[0], CANVAS[1]).expect("size"));
    for (y, colour) in [(60.0, [0.1, 0.3, 0.7]), (140.0, [0.8, 0.2, 0.1])] {
        let at = |i: u16| {
            let point = CanvasPoint::new(30.0 + 13.0 * f32::from(i), y).expect("point");
            StrokeSample::new(point, 0.9, 0.0, 0.0, u32::from(i) * 8).expect("sample")
        };
        let mut session = artwork.begin_stroke(style(colour), at(0)).expect("begin");
        session
            .extend(&(1..20).map(at).collect::<Vec<_>>())
            .expect("extend");
        artwork = session.finish().artwork;
    }
    artwork
}

/// The whole replay at 4x with `held` frames of the end, into `keeper`.
fn job(keeper: &Keeper, held: u32) -> (VideoJob, VideoFrames) {
    let drawing = artwork().drawing_order();
    let timeline = Arc::new(Timeline::of(&drawing));
    let whole = [0, timeline.total_ms()];
    let frames = VideoFrames::new((drawing, timeline), whole, 4.0, held);
    let order = VideoOrder {
        name: "Untitled.mp4".into(),
        size: VIDEO,
        fps: 30,
        delivery: Delivery::Save,
    };
    let job = VideoJob {
        order,
        frames: frames.clone(),
        encoder: Box::new(keeper.clone()),
        caption: None,
    };
    (job, frames)
}

/// The replay at a moment drawn cold and scaled to the video.
fn moment(frames: &VideoFrames, done: usize, samples: usize) -> RgbaImage {
    let frame = frames.drawing().frame(done, samples);
    let request = FrameRequest {
        artwork: &frame.artwork,
        active: frame.active.as_ref(),
        background: Color::WHITE,
    };
    let canvas = CanvasTarget::whole(&core(), &request, false).expect("cold");
    video_reference(&canvas, VIDEO)
}

#[test]
fn the_video_hands_every_frame_in_order_at_its_time_and_finishes() {
    let keeper = Keeper::default();
    let (job, frames) = job(&keeper, 6);
    let order = job.order.clone();
    let (made, cancel) = (AtomicU32::new(0), AtomicBool::new(false));
    make_video(&core(), (job, None), (&made, &cancel)).expect("made");
    let count = frames.count();
    assert!(count >= 6, "the end's frames at least");
    assert_eq!(made.into_inner(), count);
    let mut expected = vec![Call::Begin(order)];
    expected.extend((0..u64::from(count)).map(|k| Call::Frame(k * 1_000_000 / 30)));
    expected.push(Call::Finish);
    assert_eq!(keeper.calls(), expected);
    let kept = keeper.frames.lock().unwrap_or_else(PoisonError::into_inner);
    let steps = frames.drawing().steps().len();
    for (index, pixels) in kept.iter().enumerate() {
        let shown = match frames.frame(index as u32) {
            VideoFrame::At(at) => moment(&frames, at.done, at.samples),
            VideoFrame::End => moment(&frames, steps, 0),
        };
        assert!(*pixels == shown.into_pixels(), "frame {index}");
    }
}

#[test]
fn a_segment_ends_on_the_finished_picture() {
    let keeper = Keeper::default();
    let (asked, whole) = job(&keeper, 3);
    let drawing = whole.drawing().clone();
    let timeline = Arc::new(Timeline::of(&drawing));
    let half = [0, timeline.total_ms() / 2];
    let frames = VideoFrames::new((drawing, timeline), half, 4.0, 3);
    let job = VideoJob {
        frames: frames.clone(),
        ..asked
    };
    let (made, cancel) = (AtomicU32::new(0), AtomicBool::new(false));
    make_video(&core(), (job, None), (&made, &cancel)).expect("made");
    let kept = keeper.frames.lock().unwrap_or_else(PoisonError::into_inner);
    let count = kept.len();
    let steps = frames.drawing().steps().len();
    let finished = moment(&frames, steps, 0).into_pixels();
    let VideoFrame::At(last) = frames.frame(frames.count() - 4) else {
        panic!("a drawing frame before the end");
    };
    let halfway = moment(&frames, last.done, last.samples).into_pixels();
    assert!(halfway != finished, "the segment stops half way");
    assert!(
        kept[count - 3..].iter().all(|end| *end == finished),
        "Stop on the finished picture"
    );
}

#[test]
fn the_end_card_holds_the_picture_above_its_line() {
    let keeper = Keeper::default();
    let (job, frames) = job(&keeper, 3);
    let (made, cancel) = (AtomicU32::new(0), AtomicBool::new(false));
    let caption = UiFrame {
        primitives: Vec::new(),
        textures: egui::TexturesDelta::default(),
        pixels_per_point: 1.0,
    };
    make_video(&core(), (job, Some(caption)), (&made, &cancel)).expect("made");
    let kept = keeper.frames.lock().unwrap_or_else(PoisonError::into_inner);
    let ends: Vec<&Vec<u8>> = (kept.iter().enumerate())
        .filter(|(index, _)| frames.frame(*index as u32) == VideoFrame::End)
        .map(|(_, pixels)| pixels)
        .collect();
    assert_eq!(ends.len(), 3);
    assert!(
        ends.windows(2).all(|pair| pair[0] == pair[1]),
        "the card held"
    );
    let band = openpaint_ui::caption_band(VIDEO);
    let desk = openpaint_ui::design::SURROUND;
    let bottom = ((VIDEO[1] - band[1]) * VIDEO[0] * 4) as usize;
    let empty = ends[0][bottom..]
        .as_chunks::<4>()
        .0
        .iter()
        .all(|pixel| *pixel == desk);
    assert!(empty, "an empty line is the desk");
}

#[test]
fn cancel_and_a_failing_encoder_drop_what_was_begun() {
    let keeper = Keeper::default();
    let (asked, _) = job(&keeper, 2);
    let order = asked.order.clone();
    let (made, cancel) = (AtomicU32::new(0), AtomicBool::new(true));
    let cancelled = make_video(&core(), (asked, None), (&made, &cancel));
    assert_eq!(cancelled, Err(VideoError::Cancelled));
    assert_eq!(keeper.calls(), [Call::Begin(order.clone()), Call::Cancel]);
    let failing = Keeper {
        fails: true,
        ..Keeper::default()
    };
    let (asked, _) = job(&failing, 2);
    let cancel = AtomicBool::new(false);
    let failed = make_video(&core(), (asked, None), (&made, &cancel));
    assert!(matches!(failed, Err(VideoError::Write(_))), "{failed:?}");
    let expected = [Call::Begin(order), Call::Frame(0), Call::Cancel];
    assert_eq!(failing.calls(), expected);
}

#[test]
fn a_video_cancelled_before_its_card_lets_the_line_go_and_is_cancelled() {
    let keeper = Keeper::default();
    let (asked, _) = job(&keeper, 2);
    let mut textures = egui::TexturesDelta::default();
    let texel = egui::ColorImage::filled([1, 1], egui::Color32::WHITE);
    let delta = egui::epaint::ImageDelta::full(texel, egui::TextureOptions::default());
    textures.push(egui::TextureId::Managed(0), delta);
    let caption = UiFrame {
        primitives: Vec::new(),
        textures,
        pixels_per_point: 1.0,
    };
    let (made, cancel) = (AtomicU32::new(0), AtomicBool::new(true));
    let cancelled = make_video(&core(), (asked, Some(caption)), (&made, &cancel));
    assert_eq!(cancelled, Err(VideoError::Cancelled), "never a failure");
}
