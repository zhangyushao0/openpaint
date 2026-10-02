//! GPU test of the runner: a stroke changes the canvas, a finger tap and a wait do not, a
//! restart reopens the library with the stroke, a library that fails to open starts blank, and
//! the diff marks exactly the pixels the stroke changed.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use openpaint_devtools::{Checkpoint, Scenario, diff, run};

const SCENARIO: &str = r#"{
    "screen": [320, 240],
    "density": 2,
    "steps": [
        { "checkpoint": { "name": "blank", "canvas": "0000000000000000" } },
        { "stroke": { "points": [[40, 150, 0.4], [280, 150, 1.0]] } },
        { "checkpoint": { "name": "stroke", "canvas": "0000000000000000" } },
        { "tap": { "at": [160, 200] } },
        { "wait": { "frames": 2 } },
        { "checkpoint": { "name": "tapped", "canvas": "0000000000000000" } },
        { "screenshot": { "name": "screen" } },
        "restart",
        { "checkpoint": { "name": "restarted", "canvas": "0000000000000000" } },
        "storage_fails",
        "restart",
        { "checkpoint": { "name": "unreadable", "canvas": "0000000000000000" } }
    ]
}"#;

/// A library directory no earlier run used; fjall creates it.
fn library() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos());
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("devtools-runner-{nanos}"))
}

#[test]
fn steps_reach_the_canvas_and_restart_reopens_the_library() {
    let scenario = Scenario::parse(SCENARIO).expect("valid scenario");
    let report = run(&scenario, &library()).expect("a GPU adapter is required for devtools tests");
    let checkpoints: Vec<&Checkpoint> = report.checkpoints().collect();
    let hashes: Vec<u64> = checkpoints.iter().map(|c| c.actual()).collect();
    assert_ne!(hashes[0], hashes[1], "the stroke reached the canvas");
    assert_eq!(hashes[1], hashes[2], "a finger tap and a wait draw nothing");
    assert_eq!(hashes[3], hashes[1], "a restart reopens the stroke");
    assert_eq!(hashes[4], hashes[0], "an unreadable library starts blank");
    assert!(!report.passed(), "placeholder hashes fail");
    assert_eq!(
        report.records.len(),
        6,
        "five checkpoints and one screenshot"
    );

    let blank = &checkpoints[0].canvas;
    let stroke = &checkpoints[1].canvas;
    let marked = diff(blank.size(), blank.pixels(), stroke).expect("same size");
    let (before, _) = blank.pixels().as_chunks::<4>();
    let (after, _) = stroke.pixels().as_chunks::<4>();
    let changed = before.iter().zip(after).filter(|(a, b)| a != b).count();
    assert_eq!(marked.differing, changed as u64);
    assert!(marked.differing > 0);
    let same = diff(stroke.size(), stroke.pixels(), stroke).expect("same size");
    assert_eq!(same.differing, 0);
    assert!(diff([1, 1], &[0; 4], stroke).is_none(), "sizes differ");
}
