//! The harness's video encoder: frames taken only as the runner lets them, the runner going on
//! once the worker waits again; frames kept evenly spaced with the last; a video checked by its
//! calls, frames and size.

use std::thread;

use openpaint_app::{Delivery, VideoEncoder, VideoOrder};

use super::*;

const SIZE: [u32; 2] = [4, 2];

fn order() -> VideoOrder {
    VideoOrder {
        name: "Untitled.mp4".into(),
        size: SIZE,
        fps: 30,
        delivery: Delivery::Save,
    }
}

fn check(frames: Option<u32>, ended: Ended) -> VideoCheck {
    VideoCheck {
        name: "full".into(),
        frames,
        size: Some(SIZE),
        ended,
        last: None,
    }
}

/// A worker's encoder making `count` frames (frame `k` all of value `k`), then finishing.
fn made(mut encoder: Box<dyn VideoEncoder>, count: u32) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        encoder.begin(&order()).expect("begin");
        for k in 0..count {
            let pixels = vec![(k % 251) as u8; (SIZE[0] * SIZE[1] * 4) as usize];
            let at = u64::from(k) * 1_000_000 / 30;
            encoder.frame(&pixels, at).expect("frame");
        }
        encoder.finish().expect("finish");
    })
}

#[test]
fn frames_come_as_the_runner_lets_them_and_the_video_checks() {
    let sink = VideoSink::default();
    let worker = made(sink.lend(), 70);
    for taken in [30, 60] {
        sink.release().expect("released");
        let video = sink.last().expect("a video");
        assert_eq!(video.frames, taken, "the worker waits after each release");
        assert!(!video.ended());
    }
    sink.release().expect("released");
    worker.join().expect("the worker");
    let video = sink.last().expect("a video");
    assert!(video.ended());
    assert_eq!(video.frames, 70);
    sink.release().expect("nothing to wait for");
    let records = video
        .check(&check(Some(70), Ended::Finished))
        .expect("passes");
    // Frames 0, 16, 32, 48, 64 kept as the spacing doubled past eight, then the last.
    let names: Vec<String> = (records.iter())
        .map(|record| match record {
            Record::Screenshot(shot) => shot.name.clone(),
            other => format!("{other:?}"),
        })
        .collect();
    let expected = ["0000", "0016", "0032", "0048", "0064", "last"].map(|at| format!("full-{at}"));
    assert_eq!(names, expected);
    assert!(
        video.check(&check(Some(71), Ended::Finished)).is_err(),
        "a count"
    );
    assert!(
        video.check(&check(None, Ended::Cancelled)).is_err(),
        "its end"
    );
}

#[test]
fn a_last_hash_is_a_checkpoint_of_the_last_frame() {
    let sink = VideoSink::default();
    let _open = sink.opener();
    sink.set_open(true);
    made(sink.lend(), 3).join().expect("the worker");
    let video = sink.last().expect("a video");
    let pixels = vec![2; (SIZE[0] * SIZE[1] * 4) as usize];
    let last = RgbaImage::from_rgba(SIZE, pixels)
        .expect("image")
        .content_hash();
    let wanted = VideoCheck {
        last: Some(last),
        ..check(Some(3), Ended::Finished)
    };
    let records = video.check(&wanted).expect("passes");
    let Some(Record::Checkpoint(checkpoint)) = records.last() else {
        panic!("a checkpoint last: {records:?}");
    };
    assert!(checkpoint.passed());
    assert_eq!(checkpoint.name, "full");
}

#[test]
fn an_encoder_dropped_unbegun_ends_its_video_and_a_wrong_size_is_a_flaw() {
    let sink = VideoSink::default();
    drop(sink.lend());
    assert!(sink.last().expect("lent").ended());
    sink.release().expect("nothing waits");
    let mut encoder = sink.lend();
    sink.set_open(true);
    encoder.begin(&order()).expect("begin");
    encoder.frame(&[0; 7], 0).expect("frame");
    encoder.finish().expect("finish");
    let video = sink.last().expect("a video");
    assert!(video.check(&check(None, Ended::Finished)).is_err());
}
