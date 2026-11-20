//! The replay's flow: playing from blank paper to the finished artwork, the track, the speeds,
//! the stroke named, the segment's handles; the video asked for, made, saved, shared, cancelled
//! or failed, and only Cancel while it is made.

use openpaint_app::{
    AppError, Availability, Delivery, EditorAction, HostAnswer, HostRequest, PortError, Replay,
    ReplayIntent, Speed, Title, VideoEncoder, VideoError, VideoLength, VideoNotice, VideoOrder,
    VideoSize,
};
use openpaint_core::{Artwork, CanvasSize};

use super::{T, draw, paced};

#[test]
fn it_plays_from_blank_paper_stops_on_the_finished_artwork_and_follows_the_track() {
    let artwork = paced();
    let mut replay = Replay::open(&artwork, &Title::new("Hafen").expect("title"), Speed::Four);
    assert!(replay.playing());
    let blank = |replay: &Replay| replay.frame().artwork.layers()[0].strokes().is_empty();
    assert!(blank(&replay), "blank paper");
    replay.tick(1_000);
    replay.tick(1_100);
    let shown = replay.projection();
    let at = (shown.at_ms, shown.total_ms);
    assert_eq!(at, (100, 407), "400 ms of the replay at 4×");
    let stroke = shown.stroke.expect("a stroke begun");
    assert_eq!((stroke.number, stroke.began_ms), (2, Some(T + 120)));
    let edits = replay.frame().artwork.layers()[0].edits().len();
    assert_eq!(edits, 1, "the fill shown");
    replay.tick(2_000);
    assert!(!replay.playing(), "stopped at the end");
    let finished = replay.frame().artwork.same_version(&artwork);
    assert!(finished, "the finished artwork");
    replay.dispatch(&ReplayIntent::Play(true)).expect("again");
    assert!(replay.playing() && replay.projection().at_ms == 0 && blank(&replay));
    let seek = ReplayIntent::Seek(180.0 / 1630.0);
    replay.dispatch(&seek).expect("seek");
    let shown = replay.projection();
    assert_eq!(shown.tip, Some([7.0, 10.0]), "B drawn to its sixth sample");
    assert_eq!(shown.stroke.map(|stroke| stroke.number), Some(2));
    let faster = ReplayIntent::Speed(Speed::Sixteen);
    replay.dispatch(&faster).expect("speed");
    assert_eq!(replay.projection().total_ms, 1630 / 16);
    replay.dispatch(&ReplayIntent::Seek(0.5)).expect("seek");
    let segment = ReplayIntent::Length(VideoLength::Segment);
    replay.dispatch(&segment).expect("segment");
    let export = replay.projection().export;
    assert_eq!(export.segment, [0.5, 1.0], "from the playhead to the end");
    assert_eq!(export.segment_ms, [815 / 16, 1630 / 16]);
    let handles = ReplayIntent::Segment([0.75, 0.25]);
    replay.dispatch(&handles).expect("handles");
    let segment_ms = replay.projection().export.segment_ms;
    assert_eq!(segment_ms, [408 / 16, 1223 / 16]);
}

/// An encoder that keeps nothing.
struct Kept;

impl VideoEncoder for Kept {
    fn begin(&mut self, _: &VideoOrder) -> Result<(), VideoError> {
        Ok(())
    }
    fn frame(&mut self, _: &[u8], _: u64) -> Result<(), VideoError> {
        Ok(())
    }
    fn finish(&mut self) -> Result<(), VideoError> {
        Ok(())
    }
    fn cancel(&mut self) {}
}

/// A replay of the paced artwork, titled "Hafen".
fn hafen() -> Replay {
    Replay::open(&paced(), &Title::new("Hafen").expect("title"), Speed::Four)
}

/// `replay` exporting by `delivery`: the encoder asked for and lent; the job's frames.
fn exported(replay: &mut Replay, delivery: Delivery) -> u32 {
    let export = ReplayIntent::Export(delivery);
    replay.dispatch(&export).expect("export");
    let name = "Hafen.mp4".to_owned();
    let asked = Some(HostRequest::Video { name, delivery });
    assert_eq!(replay.request(), asked);
    assert!(replay.waiting() && replay.making() && !replay.playing());
    let job = replay.answer(HostAnswer::Video(Box::new(Kept)));
    let job = job.expect("the job");
    assert_eq!((job.order.size, job.order.fps), ([64, 48], 30));
    job.frames.count()
}

fn refused(availability: Availability) -> Result<(), AppError> {
    Err(AppError::Unavailable {
        action: EditorAction::Replay,
        availability,
    })
}

#[test]
fn a_video_is_made_saved_or_shared_and_only_cancel_works_meanwhile() {
    let mut replay = hafen();
    let frames = exported(&mut replay, Delivery::Save);
    let running = refused(Availability::Running);
    assert_eq!(replay.dispatch(&ReplayIntent::Play(true)), running);
    assert_eq!(replay.dispatch(&ReplayIntent::Close), running);
    assert_eq!(replay.projection().controls, Availability::Running);
    replay.progress(frames / 2);
    let working = VideoNotice::Working { percent: 50 };
    assert_eq!(replay.projection().video, working);
    replay.finished(Ok(()));
    let seconds = (frames + 15) / 30;
    assert_eq!(replay.projection().video, VideoNotice::Saved { seconds });
    assert_eq!(replay.projection().controls, Availability::Available);
    replay.dispatch(&ReplayIntent::ShareSaved).expect("share");
    let name = "Hafen.mp4".to_owned();
    let send = Some(HostRequest::SendVideo { name });
    assert_eq!(replay.request(), send);
    assert_eq!(replay.projection().video, VideoNotice::None);
    exported(&mut replay, Delivery::Share);
    replay.finished(Ok(()));
    assert_eq!(replay.request(), send, "the share sheet at once");
    assert_eq!(replay.projection().video, VideoNotice::None);
}

#[test]
fn a_video_cancelled_tells_nothing_one_failed_tells_and_choices_not_offered_are_refused() {
    let mut replay = hafen();
    exported(&mut replay, Delivery::Save);
    replay.dispatch(&ReplayIntent::Cancel).expect("cancel");
    assert!(replay.making(), "until the worker stops");
    replay.finished(Err(VideoError::Cancelled));
    let told = (replay.making(), replay.projection().video);
    assert_eq!(told, (false, VideoNotice::None));
    exported(&mut replay, Delivery::Save);
    replay.finished(Err(VideoError::Render));
    let failed = VideoNotice::Failed(VideoError::Render);
    assert_eq!(replay.projection().video, failed);
    replay.dispatch(&ReplayIntent::Dismiss).expect("OK");
    let export = ReplayIntent::Export(Delivery::Save);
    replay.dispatch(&export).expect("export");
    let error = PortError::Device {
        detail: "busy".into(),
    };
    assert!(replay.answer(HostAnswer::Failed(error.clone())).is_none());
    let failed = VideoNotice::Failed(VideoError::Encoder(error));
    assert_eq!(replay.projection().video, failed);
    let none = refused(Availability::NothingLeft);
    assert_eq!(replay.dispatch(&ReplayIntent::ShareSaved), none);
    assert_eq!(replay.projection().video, failed, "the notice stays");
    replay.dispatch(&ReplayIntent::Dismiss).expect("OK");
    assert_eq!(replay.dispatch(&ReplayIntent::Dismiss), none);
    assert_eq!(replay.dispatch(&ReplayIntent::Cancel), none);
    replay
        .dispatch(&ReplayIntent::Hold(false))
        .expect("no hold");
    assert_eq!(replay.dispatch(&ReplayIntent::Caption(true)), none);
    let wide = Artwork::new(CanvasSize::new(5000, 3000).expect("size"));
    let mut wide = Replay::open(&draw(&wide, T, 2), &Title::untitled(), Speed::One);
    let own = ReplayIntent::Size(VideoSize::Canvas);
    assert_eq!(wide.dispatch(&own), none, "beyond the encoder");
}
