//! A session's record, its places with and without a warm-up, the kinds met, and its summary
//! beside the sessions before it.

use openpaint_practice::{
    Arrangement, Axis, Combination, Exercise, ExerciseResult, Family, ItemRef, Kind, LineTier,
    Metric, PROJECT, Plan, ScaleTone, SessionRecord, Tone, met, summary, used,
};

#[test]
fn a_long_colour_copy_is_kept_beside_the_sessions_and_counts_in_none() {
    let item = ItemRef {
        pack: "test-shapes".to_owned(),
        item: "teapot".to_owned(),
    };
    let mut copy = SessionRecord::project(5, 2, (77, 100), item.clone());
    assert!(copy.is_project());
    assert_eq!((copy.stage, copy.number, copy.warm_up), (PROJECT, 2, 0));
    assert_eq!(copy.exercises.len(), 1);
    assert_eq!(copy.exercises[0].kind, Kind::Study);
    assert_eq!(copy.exercises[0].picture, Some(item));
    copy.results.push(checked(0, Vec::new()));
    assert!(
        met(std::slice::from_ref(&copy)).is_empty(),
        "no warm-up from a copy"
    );
    assert!(used(std::slice::from_ref(&copy)).is_empty());
    assert!(!SessionRecord::stage_one(1, 1, 7, 0, &[]).is_project());
}

#[test]
fn the_summary_sets_combinations_and_scales_beside_the_earlier_ones() {
    let plan = Plan {
        exercises: vec![
            Exercise::generated(Kind::Areas(Arrangement::Strata), 1),
            Exercise::generated(Kind::Scale(ScaleTone::Grey), 2),
        ],
        warm_up: 0,
    };
    let mut before = SessionRecord::begun(1, (2, 1), (5, 0), plan.clone());
    before.results = vec![checked(
        0,
        vec![(Metric::AreasDelta, 0.08), (Metric::AreasMadeStrong, 2.0)],
    )];
    let mut this = SessionRecord::begun(2, (2, 2), (6, 0), plan);
    this.results = vec![
        checked(
            0,
            vec![(Metric::AreasDelta, 0.05), (Metric::AreasMadeStrong, 0.0)],
        ),
        checked(
            1,
            vec![(Metric::ScaleOff, 0.04), (Metric::ScaleReversed, 1.0)],
        ),
    ];
    let kinds = summary(&this, &[&before]);
    let families: Vec<Family> = kinds.iter().map(|k| k.family).collect();
    assert_eq!(families, [Family::Areas, Family::Scale]);
    assert_eq!(
        kinds[0].rows[0],
        (Metric::AreasDelta, Some(0.05), Some(0.08))
    );
    assert_eq!(
        kinds[0].rows[1],
        (Metric::AreasMadeStrong, Some(0.0), Some(2.0))
    );
    assert_eq!(kinds[1].rows[0], (Metric::ScaleOff, Some(0.04), None));
    assert!(
        kinds.iter().all(|k| k.judgement.is_none()),
        "the radar is the colours'"
    );
}

fn checked(index: u32, metrics: Vec<(Metric, f64)>) -> ExerciseResult {
    ExerciseResult::first(index, 1_791_201_600, 40, metrics)
}

#[test]
fn a_session_knows_its_places_and_results() {
    let mut session = SessionRecord::stage_one(3, 2, 99, 1_791_201_600, &[]);
    assert_eq!((session.stage, session.number, session.current), (1, 2, 0));
    assert_eq!(session.warm_up, 0);
    assert_eq!(session.place(0), (1, 6));
    assert_eq!(session.place(5), (6, 6));
    assert_eq!(session.place(6), (1, 4), "combined lines");
    assert_eq!(session.place(9), (4, 4));
    assert_eq!(session.place(10), (1, 6), "colours");
    assert_eq!(session.place(15), (6, 6));
    assert_eq!(session.place(16), (0, 0));
    assert!(!session.warming_up(0));
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
fn a_warm_up_has_its_own_places_before_the_mix() {
    let met = [
        Kind::Line(LineTier::Wave),
        Kind::Colour(Tone::of_code(1).expect("a tone")),
    ];
    let session = SessionRecord::stage_one(4, 3, 7, 0, &met);
    assert_eq!(session.warm_up, 4);
    assert_eq!(session.exercises.len(), 20);
    assert!(session.warming_up(3) && !session.warming_up(4));
    assert_eq!(session.place(0), (1, 4));
    assert_eq!(session.place(3), (4, 4), "a colour in the warm-up");
    assert_eq!(session.place(4), (1, 6), "the mix's first line");
    assert_eq!(session.place(14), (1, 6), "the mix's first colour");
}

#[test]
fn the_kinds_met_are_those_compared_each_once() {
    let mut first = SessionRecord::stage_one(1, 1, 7, 0, &[]);
    first.results = vec![
        checked(0, Vec::new()),
        checked(7, Vec::new()),
        checked(10, Vec::new()),
    ];
    let mut second = SessionRecord::stage_one(2, 2, 8, 0, &met(std::slice::from_ref(&first)));
    second.results = vec![checked(4, Vec::new())];
    let kinds = met(&[first.clone(), second.clone()]);
    let expected = vec![
        first.exercises[0].kind,
        first.exercises[10].kind,
        first.exercises[7].kind,
    ];
    assert_eq!(
        kinds, expected,
        "lines, colours, combined lines by their codes; once each"
    );
    assert!(matches!(kinds[2], Kind::Combined(Combination::Crossing)));
    assert_eq!(
        second.exercises[4].kind, first.exercises[0].kind,
        "the mix again"
    );
    assert!(met(&[]).is_empty());
}

#[test]
fn the_summary_counts_each_family_and_sets_colours_beside_the_earlier_ones() {
    let mut before = SessionRecord::stage_one(1, 1, 7, 0, &[]);
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
    let mut this = SessionRecord::stage_one(2, 2, 8, 100, &[]);
    this.results = vec![
        checked(0, Vec::new()),
        checked(2, Vec::new()),
        checked(7, Vec::new()),
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
    let families: Vec<Family> = kinds.iter().map(|k| k.family).collect();
    assert_eq!(families, [Family::Line, Family::Combined, Family::Colour]);
    let lines = &kinds[0];
    assert_eq!((lines.checked, lines.held), (2, 6));
    assert!(
        lines.rows.is_empty() && lines.judgement.is_none(),
        "no numbers"
    );
    let combined = &kinds[1];
    assert_eq!((combined.checked, combined.held), (1, 4));
    assert!(combined.rows.is_empty() && combined.judgement.is_none());
    let colours = &kinds[2];
    assert_eq!((colours.checked, colours.held), (1, 6));
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
    assert!(first[2].judgement.expect("a judgement")[1].is_empty());
}
