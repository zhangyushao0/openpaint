//! A session's record and its summary beside the sessions before it.

use openpaint_practice::{Axis, ExerciseResult, Metric, SessionRecord, summary};

fn checked(index: u32, metrics: Vec<(Metric, f64)>) -> ExerciseResult {
    ExerciseResult::first(index, 1_791_201_600, 40, metrics)
}

#[test]
fn a_session_knows_its_places_and_results() {
    let mut session = SessionRecord::stage_one(3, 2, 99, 1_791_201_600);
    assert_eq!((session.stage, session.number, session.current), (1, 2, 0));
    assert_eq!(session.place(0), (1, 10));
    assert_eq!(session.place(9), (10, 10));
    assert_eq!(session.place(10), (1, 8));
    assert_eq!(session.place(17), (8, 8));
    assert_eq!(session.place(18), (0, 0));
    session.results.push(checked(4, Vec::new()));
    assert!(session.result(4).is_some() && session.result(3).is_none());
    assert_eq!(session.exercise(), session.exercises.first());
    let result = session.result(4).expect("compared");
    assert_eq!(result.compares, 1);
    assert!(!result.corrects(39) && result.corrects(40) && result.corrects(41));
    let old = ExerciseResult {
        corrected_from: None,
        ..result.clone()
    };
    assert!(
        !old.corrects(u64::MAX),
        "1.1.0's records: nothing after Compare"
    );
}

#[test]
fn the_summary_counts_lines_and_sets_colours_beside_the_earlier_ones() {
    let mut before = SessionRecord::stage_one(1, 1, 7, 0);
    before.results = vec![
        checked(0, Vec::new()),
        checked(
            10,
            vec![
                (Metric::FirstDelta, 0.08),
                (Metric::FinalDelta, 0.05),
                (Metric::Corrections, 3.0),
            ],
        ),
    ];
    let mut this = SessionRecord::stage_one(2, 2, 8, 100);
    this.results = vec![
        checked(0, Vec::new()),
        checked(2, Vec::new()),
        checked(
            11,
            vec![
                (Metric::FirstDelta, 0.04),
                (Metric::FinalDelta, 0.01),
                (Metric::FinalValue, -0.05),
            ],
        ),
    ];
    let kinds = summary(&this, &[&before]);
    assert_eq!(kinds.len(), 2);
    let lines = &kinds[0];
    assert!(lines.lines);
    assert_eq!((lines.checked, lines.held), (2, 10));
    assert!(
        lines.rows.is_empty() && lines.judgement.is_none(),
        "no numbers"
    );
    let colours = &kinds[1];
    assert!(!colours.lines);
    assert_eq!((colours.checked, colours.held), (1, 8));
    assert_eq!(
        colours.rows[0],
        (Metric::FirstDelta, Some(0.04), Some(0.08))
    );
    assert_eq!(
        colours.rows[1],
        (Metric::FinalDelta, Some(0.01), Some(0.05))
    );
    assert_eq!(colours.rows[2], (Metric::Corrections, None, Some(3.0)));
    let [mine, earlier] = colours.judgement.expect("a judgement");
    let value = mine.of(Axis::Value).expect("value");
    assert!((value - 0.8).abs() < 1e-9, "{value}");
    assert_eq!(earlier.of(Axis::Value), None);
    assert!(earlier.of(Axis::FirstToFinal).is_some());
    let first = summary(&this, &[]);
    assert!(first[1].judgement.expect("a judgement")[1].is_empty());
}
