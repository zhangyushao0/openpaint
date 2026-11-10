//! A sheet's record, its places with and without a warm-up, the kinds met, a lesson's number among
//! the lessons alike, and its summary beside the lessons before it.

use openpaint_practice::{
    Arrangement, Axis, Combination, Exercise, Family, First, ItemRef, Kind, LineTier, Metric,
    Picked, Plan, Purpose, ScaleTone, Sheet, Tone, met, number, scales_met, stage_one, summary,
    used,
};

#[test]
fn a_long_colour_copy_is_kept_beside_the_sessions_and_counts_in_none() {
    let item = ItemRef {
        pack: "test-shapes".to_owned(),
        item: "teapot".to_owned(),
    };
    let mut copy = Sheet::copy(5, (77, 100), item.clone());
    assert!(copy.is_copy());
    assert_eq!(
        (copy.purpose, copy.stage(), copy.warm_up()),
        (Purpose::Copy, None, 0)
    );
    assert_eq!(copy.drilled(), None);
    assert_eq!(copy.exercises().count(), 1);
    let laid = copy.laid(0).expect("its exercise");
    assert_eq!(laid.exercise.kind, Kind::Study);
    assert_eq!(laid.exercise.picture, Some(item));
    checked(&mut copy, 0, Vec::new());
    assert!(
        met(std::slice::from_ref(&copy)).is_empty(),
        "no warm-up from a copy"
    );
    assert!(used(std::slice::from_ref(&copy)).is_empty());
    assert!(!Sheet::session(1, 1, 0, stage_one(7, &[])).is_copy());
}

#[test]
fn a_sheet_is_numbered_among_the_sheets_alike() {
    let item = ItemRef {
        pack: "test-shapes".to_owned(),
        item: "teapot".to_owned(),
    };
    let lines = |seed| Plan {
        exercises: vec![Exercise::generated(Kind::Line(LineTier::Arc), seed)],
        warm_up: 0,
    };
    let colours = Plan {
        exercises: vec![Exercise::generated(Kind::Scale(ScaleTone::Hue, 5), 3)],
        warm_up: 0,
    };
    let sheets = [
        Sheet::session(1, 1, 0, stage_one(7, &[])),
        Sheet::copy(2, (3, 0), item.clone()),
        Sheet::drill(3, 0, lines(4)),
        Sheet::session(4, 2, 0, lines(5)),
        Sheet::session(5, 1, 0, stage_one(8, &[])),
        Sheet::drill(6, 0, colours),
        Sheet::drill(7, 0, lines(6)),
        Sheet::copy(8, (9, 0), item),
    ];
    let numbers: Vec<u32> = sheets.iter().map(|s| number(&sheets, s, 0)).collect();
    assert_eq!(numbers, [1, 1, 1, 1, 2, 1, 2, 2]);
    let picked = sheets.iter().map(|s| s.lesson().map(|l| l.picked));
    let picked: Vec<Option<Picked>> = picked.collect();
    assert_eq!(
        picked[0],
        Some(Picked::Full {
            stage: 1,
            warm_up: 0
        })
    );
    assert_eq!(picked[2], Some(Picked::Chosen));
    assert_eq!(sheets[2].drilled(), Some(Family::Line));
    assert_eq!(sheets[3].drilled(), None);
}

#[test]
fn the_summary_sets_combinations_and_scales_beside_the_earlier_ones() {
    let plan = Plan {
        exercises: vec![
            Exercise::generated(Kind::Areas(Arrangement::Strata), 1),
            Exercise::generated(Kind::Scale(ScaleTone::Grey, 9), 2),
        ],
        warm_up: 0,
    };
    let mut before = Sheet::session(1, 2, 0, plan.clone());
    let areas = |delta, strong| {
        vec![
            (Metric::AreasDelta, delta),
            (Metric::AreasMadeStrong, strong),
        ]
    };
    checked(&mut before, 0, areas(0.08, 2.0));
    let mut this = Sheet::session(2, 2, 0, plan);
    checked(&mut this, 0, areas(0.05, 0.0));
    let scale = vec![(Metric::ScaleOff, 0.04), (Metric::ScaleReversed, 1.0)];
    checked(&mut this, 1, scale);
    let kinds = summary(&this.lessons[0], &[&before.lessons[0]]);
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
    // The value scales met (S97): compared ones only, sessions and drills alike, copies never.
    assert_eq!(scales_met(&[before.clone()]), 0, "its scale never compared");
    assert_eq!(scales_met(&[before.clone(), this.clone()]), 1);
    let scales = Plan {
        exercises: vec![Exercise::generated(Kind::Scale(ScaleTone::Hue, 5), 3)],
        warm_up: 0,
    };
    let mut drill = Sheet::drill(3, 0, scales);
    checked(&mut drill, 0, Vec::new());
    assert_eq!(scales_met(&[before, this, drill]), 2);
}

/// Exercise `index` of `sheet` compared for the first time, with `metrics`.
fn checked(sheet: &mut Sheet, index: usize, metrics: Vec<(Metric, f64)>) {
    let laid = sheet.laid_mut(index).expect("an exercise");
    laid.compared(First::of(1_791_201_600, 40, metrics));
}

#[test]
fn a_session_knows_its_places_and_results() {
    let mut session = Sheet::session(3, 1, 1_791_201_600, stage_one(99, &[]));
    assert_eq!((session.stage(), session.at()), (Some(1), 0));
    assert_eq!(session.warm_up(), 0);
    assert_eq!(session.place(0), (1, 6));
    assert_eq!(session.place(5), (6, 6));
    assert_eq!(session.place(6), (1, 4), "combined lines");
    assert_eq!(session.place(9), (4, 4));
    assert_eq!(session.place(10), (1, 6), "colours");
    assert_eq!(session.place(15), (6, 6));
    assert_eq!(session.place(16), (0, 0));
    assert!(!session.warming_up(0));
    checked(&mut session, 4, Vec::new());
    assert!(session.first(4).is_some() && session.first(3).is_none());
    let laid = session.laid(4).expect("compared");
    assert_eq!(laid.compares, 1);
    let first = laid.first.clone().expect("its first Compare");
    assert!(!first.corrects(39) && first.corrects(40) && first.corrects(41));
    let old = First {
        corrected_from: None,
        ..first
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
    let session = Sheet::session(4, 1, 0, stage_one(7, &met));
    assert_eq!(session.warm_up(), 4);
    assert_eq!(session.exercises().count(), 20);
    assert!(session.warming_up(3) && !session.warming_up(4));
    assert_eq!(session.place(0), (1, 4));
    assert_eq!(session.place(3), (4, 4), "a colour in the warm-up");
    assert_eq!(session.place(4), (1, 6), "the mix's first line");
    assert_eq!(session.place(14), (1, 6), "the mix's first colour");
}

/// The kind of `sheet`'s exercise `index`.
fn kind(sheet: &Sheet, index: usize) -> Kind {
    sheet.laid(index).expect("an exercise").exercise.kind
}

#[test]
fn the_kinds_met_are_those_compared_each_once() {
    let mut first = Sheet::session(1, 1, 0, stage_one(7, &[]));
    for index in [0, 7, 10] {
        checked(&mut first, index, Vec::new());
    }
    let mut second = Sheet::session(2, 1, 0, stage_one(8, &met(std::slice::from_ref(&first))));
    checked(&mut second, 4, Vec::new());
    let kinds = met(&[first.clone(), second.clone()]);
    let expected = vec![kind(&first, 0), kind(&first, 10), kind(&first, 7)];
    assert_eq!(
        kinds, expected,
        "lines, colours, combined lines by their codes; once each"
    );
    assert!(matches!(kinds[2], Kind::Combined(Combination::Crossing)));
    assert_eq!(kind(&second, 4), kind(&first, 0), "the mix again");
    assert!(met(&[]).is_empty());
}

#[test]
fn the_summary_counts_each_family_and_sets_colours_beside_the_earlier_ones() {
    let mut before = Sheet::session(1, 1, 0, stage_one(7, &[]));
    checked(&mut before, 0, Vec::new());
    let colour = vec![
        (Metric::FirstDelta, 0.08),
        (Metric::FinalDelta, 0.05),
        (Metric::Corrections, 3.0),
    ];
    checked(&mut before, 10, colour);
    let mut this = Sheet::session(2, 1, 100, stage_one(8, &[]));
    for index in [0, 2, 7] {
        checked(&mut this, index, Vec::new());
    }
    let colour = vec![
        (Metric::FirstDelta, 0.04),
        (Metric::FinalDelta, 0.01),
        (Metric::FinalLightness, -0.01),
    ];
    checked(&mut this, 11, colour);
    let kinds = summary(&this.lessons[0], &[&before.lessons[0]]);
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
    let lightness = mine.of(Axis::Lightness).expect("lightness");
    assert!((lightness - 0.8).abs() < 1e-9, "{lightness}");
    assert_eq!(earlier.of(Axis::Lightness), None);
    assert!(earlier.of(Axis::FirstToFinal).is_some());
    let first = summary(&this.lessons[0], &[]);
    assert!(first[2].judgement.expect("a judgement")[1].is_empty());
}
