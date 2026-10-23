//! A session's record and its summary beside the sessions before it.

use openpaint_practice::{ExerciseResult, Metric, SessionRecord, summary};

fn checked(index: u32, metrics: Vec<(Metric, f64)>) -> ExerciseResult {
    ExerciseResult {
        index,
        checked_at: 1_791_201_600,
        metrics,
    }
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
    session
        .results
        .push(checked(4, vec![(Metric::Frame, 0.02)]));
    assert!(session.result(4).is_some() && session.result(3).is_none());
    assert_eq!(session.exercise(), session.exercises.first());
}

#[test]
fn the_summary_sets_this_sessions_means_beside_the_earlier_ones() {
    let mut before = SessionRecord::stage_one(1, 1, 7, 0);
    before.results = vec![
        checked(0, vec![(Metric::Frame, 0.04), (Metric::Speed, 1.0)]),
        checked(1, vec![(Metric::Frame, 0.06)]),
        checked(
            10,
            vec![(Metric::FinalDelta, 0.05), (Metric::Corrections, 3.0)],
        ),
    ];
    let mut this = SessionRecord::stage_one(2, 2, 8, 100);
    this.results = vec![
        checked(0, vec![(Metric::Frame, 0.02), (Metric::Speed, 2.0)]),
        checked(2, vec![(Metric::Frame, 0.03), (Metric::Speed, 1.0)]),
    ];
    let kinds = summary(&this, &[&before]);
    assert_eq!(kinds.len(), 2);
    let lines = &kinds[0];
    assert!(lines.lines);
    assert_eq!((lines.checked, lines.held), (2, 10));
    let row = |metric| {
        lines
            .rows
            .iter()
            .find(|(m, ..)| *m == metric)
            .map(|(_, this, before)| (*this, *before))
    };
    assert_eq!(row(Metric::Frame), Some((Some(0.025), Some(0.05))));
    assert_eq!(row(Metric::Speed), Some((Some(1.5), Some(1.0))));
    assert_eq!(row(Metric::Shape), Some((None, None)));
    let colours = &kinds[1];
    assert!(!colours.lines);
    assert_eq!((colours.checked, colours.held), (0, 8));
    assert_eq!(colours.rows[1], (Metric::FinalDelta, None, Some(0.05)));
    let first = summary(&this, &[]);
    assert_eq!(first[0].rows[1], (Metric::Frame, Some(0.025), None));
}
