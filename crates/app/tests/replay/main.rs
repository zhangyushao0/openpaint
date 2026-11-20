//! The replay (S163, D125): the pace (pen time, capped pauses, untimed strokes, copies at once,
//! edits, hidden steps, sittings), the facts, video frames and sizes; the flow (`flow.rs`:
//! playing, the track, speeds, the stroke named, the export and its notices, refusals while a
//! video is made) and the session's part (`session.rs`: strokes stamped on the UTC clock,
//! Actions' offer).

mod flow;
mod session;

use std::sync::Arc;

use openpaint_app::{
    Replay, ReplayIntent, Speed, Timeline, Title, VideoFrame, VideoFrames, VideoSize, video_size,
};
use openpaint_core::{
    Artwork, ArtworkCommand, BrushRecipe, CanvasPoint, CanvasSize, Color, LayerCommand, LayerId,
    RegionMask, StrokeSample, StrokeStyle,
};

const T: u64 = 1_794_690_420_000;
const MINUTE: u64 = 60_000;

fn blank() -> Artwork {
    Artwork::new(CanvasSize::new(64, 48).expect("size"))
}

/// `artwork` with a stroke of `samples` samples 10 ms apart from x 2, begun at `began` (0: no
/// start).
fn draw(artwork: &Artwork, began: u64, samples: u32) -> Artwork {
    let style = StrokeStyle::new(Arc::new(BrushRecipe::plain()), 4.0, 1.0, Color::BLACK);
    let sample = |i: u32| {
        let point = CanvasPoint::new(2.0 + i as f32, 10.0).expect("point");
        StrokeSample::new(point, 0.5, 0.0, 0.0, i * 10).expect("sample")
    };
    let session = artwork.begin_stroke(style.expect("style"), sample(0));
    let mut session = session.expect("begin").began_at(began);
    session
        .extend(&(1..samples).map(sample).collect::<Vec<_>>())
        .expect("extend");
    session.finish().artwork
}

fn fill(artwork: &Artwork) -> Artwork {
    let region = RegionMask::polygon(artwork.canvas(), &[[2.0, 2.0], [30.0, 2.0], [2.0, 30.0]]);
    let filled = artwork.fill_region(region.expect("region"), Color::BLACK);
    filled.expect("fill").artwork
}

fn layer(artwork: &Artwork, command: LayerCommand) -> Artwork {
    let applied = artwork.apply(ArtworkCommand::Layer(command));
    applied.expect("layer command").artwork
}

/// Strokes A (20 ms), B (100 ms) after a 100 ms pause, a fill, C after ten seconds, D without a
/// start and E after it: at 1× A 0 to 30 (a dot's least), B 130 to 230, the fill 230 to 630,
/// C 930 to 960, D 1260 to 1300, E 1600 to 1630.
fn paced() -> Artwork {
    let b = draw(&draw(&blank(), T, 3), T + 120, 11);
    let c = draw(&fill(&b), T + 10_000, 2);
    draw(&draw(&c, 0, 5), T + 20_000, 2)
}

#[test]
fn strokes_take_their_pen_time_after_capped_pauses_and_edits_their_own() {
    let order = paced().drawing_order();
    let timeline = Timeline::of(&order);
    let begins: Vec<_> = (0..6).filter_map(|step| timeline.begins(step)).collect();
    assert_eq!(begins, [0, 130, 230, 930, 1260, 1600]);
    assert_eq!(timeline.total_ms(), 1630);
    let at = |ms| {
        let at = timeline.at(&order, ms);
        (at.done, at.samples)
    };
    assert_eq!(at(0), (0, 1), "A begun");
    assert_eq!(at(15), (0, 2), "its samples as their times come");
    assert_eq!(at(30), (1, 0), "A done, B in its pause");
    assert_eq!(at(230), (3, 0), "B done, the fill shown from its start");
    assert_eq!(at(1280), (4, 3), "D drawn to its third sample");
    assert_eq!(at(99_999), (6, 0), "the end");
    assert!(timeline.sittings().is_empty());
}

#[test]
fn copies_come_at_once_hidden_steps_take_no_time_and_sittings_are_marked() {
    let two = draw(&draw(&blank(), T, 2), T + 1_000, 2);
    let copied = layer(&two, LayerCommand::Duplicate(LayerId::new(1)));
    let hidden = layer(&copied, LayerCommand::Create);
    let hidden = draw(&hidden, T + 2_000, 2);
    let id = hidden.active();
    let hidden = layer(
        &hidden,
        LayerCommand::SetVisible {
            layer: id,
            visible: false,
        },
    );
    let later = draw(
        &layer(&hidden, LayerCommand::Select(LayerId::new(1))),
        T + 40 * MINUTE,
        2,
    );
    let order = later.drawing_order();
    let timeline = Timeline::of(&order);
    let spans: Vec<_> = (0..6).filter_map(|step| timeline.begins(step)).collect();
    // A 0-30, B 330-360, A's copy at once, B's copy after no pause, the hidden one at once,
    // the next sitting's first stroke after the longest pause.
    assert_eq!(spans, [0, 330, 360, 360, 390, 690]);
    assert_eq!(timeline.sittings(), [690]);
    let facts = timeline.facts();
    assert_eq!((facts.strokes, facts.layers), (6, 3));
    assert_eq!(facts.began_ms, Some([T, T + 40 * MINUTE]));
    // A 10, the second's pause and 10, B's copy within B, the hidden one's pause and 10, the
    // last 10 after a sitting's pause.
    assert_eq!(facts.spent_ms, Some(10 + 990 + 10 + 990 + 10 + 10));
    let untimed = Timeline::of(&draw(&blank(), 0, 3).drawing_order()).facts();
    assert_eq!((untimed.spent_ms, untimed.began_ms), (None, None));
}

#[test]
fn a_video_draws_its_range_to_its_last_moment_then_holds_the_end() {
    let order = paced().drawing_order();
    let timeline = Arc::new(Timeline::of(&order));
    let frames = VideoFrames::new((order, Arc::clone(&timeline)), [0, 1630], 1.0, 90);
    assert_eq!(
        frames.count(),
        50 + 90,
        "a frame each 33 ms, the last at the end"
    );
    let at = |index| match frames.frame(index) {
        VideoFrame::At(at) => Some((at.done, at.samples)),
        VideoFrame::End => None,
    };
    assert_eq!(at(0), Some((0, 1)));
    assert_eq!(
        at(4),
        Some((1, 1)),
        "133 ms: B begun at 130, its first sample"
    );
    assert_eq!(at(49), Some((6, 0)), "exactly the end");
    assert_eq!((at(50), at(139)), (None, None), "the end held");
    // A long replay's highlight is 30 s, the hold inside; never slower than 1×.
    let mut long = blank();
    for _ in 0..100 {
        long = draw(&long, 0, 31);
    }
    let mut replay = Replay::open(&long, &Title::untitled(), Speed::One);
    let lengths = replay.projection().export.lengths_ms;
    assert_eq!(lengths, [30_000, 59_733 + 3_000, 59_733 + 3_000]);
    replay
        .dispatch(&ReplayIntent::Hold(false))
        .expect("no hold");
    assert_eq!(replay.projection().export.lengths_ms[0], 30_000);
    let short = Replay::open(&paced(), &Title::untitled(), Speed::One);
    assert_eq!(
        short.projection().export.lengths_ms[0],
        (50 + 90) * 1000 / 30
    );
}

#[test]
fn a_video_is_the_canvas_fitted_in_1080p_or_its_own_size_within_the_encoder_s() {
    let size = |w, h, choice| video_size(CanvasSize::new(w, h).expect("size"), choice);
    assert_eq!(size(2880, 1800, VideoSize::Hd), Some([1728, 1080]));
    assert_eq!(size(2880, 1800, VideoSize::Canvas), Some([2880, 1800]));
    assert_eq!(
        size(2480, 3508, VideoSize::Hd),
        Some([1080, 1526]),
        "standing"
    );
    assert_eq!(size(2480, 3508, VideoSize::Canvas), Some([2480, 3508]));
    assert_eq!(
        size(1000, 700, VideoSize::Hd),
        Some([1000, 700]),
        "never larger"
    );
    assert_eq!(
        size(999, 555, VideoSize::Hd),
        Some([998, 554]),
        "sides even"
    );
    assert_eq!(size(2000, 2000, VideoSize::Hd), Some([1080, 1080]));
    assert_eq!(size(5000, 3000, VideoSize::Hd), Some([1800, 1080]));
    assert_eq!(size(5000, 3000, VideoSize::Canvas), None);
    assert_eq!(size(4096, 2304, VideoSize::Canvas), Some([4096, 2304]));
    assert_eq!(
        size(4096, 2400, VideoSize::Canvas),
        None,
        "beyond level 5.1's pixels"
    );
}
