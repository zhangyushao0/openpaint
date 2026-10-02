//! GPU test of the runner: a stroke changes the canvas, a finger tap and a wait do not, a
//! restart starts a blank app (nothing survives until storage exists, S7), and the diff marks
//! exactly the pixels the stroke changed.

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
        { "checkpoint": { "name": "restarted", "canvas": "0000000000000000" } }
    ]
}"#;

#[test]
fn steps_reach_the_canvas_and_restart_starts_blank() {
    let scenario = Scenario::parse(SCENARIO).expect("valid scenario");
    let report = run(&scenario).expect("a GPU adapter is required for devtools tests");
    let checkpoints: Vec<&Checkpoint> = report.checkpoints().collect();
    let hashes: Vec<u64> = checkpoints.iter().map(|c| c.actual()).collect();
    assert_ne!(hashes[0], hashes[1], "the stroke reached the canvas");
    assert_eq!(hashes[1], hashes[2], "a finger tap and a wait draw nothing");
    assert_eq!(hashes[3], hashes[0], "a restart opens a blank canvas");
    assert!(!report.passed(), "placeholder hashes fail");
    assert_eq!(
        report.records.len(),
        5,
        "four checkpoints and one screenshot"
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
