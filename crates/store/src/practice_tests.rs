use openpaint_practice::{Combination, LineTier};

use super::*;

/// Bytes after the results' tail of a record holding no pictures and no time: the warm-up's
/// count, the pictures', the spans' and the layout (S99).
const END_BYTES: usize = 4 + 4 + 4 + 1;

fn session() -> SessionRecord {
    let met = [Kind::Line(LineTier::Arc), Kind::Combined(Combination::Near)];
    let mut session = SessionRecord::stage_one(7, 3, 0x5EED_0045, 1_791_201_600, &met);
    assert_eq!(session.warm_up, 4);
    session.canvas = 12;
    session.current = 11;
    session.finished = Some(1_791_202_000);
    let mut line = ExerciseResult::first(4, 1_791_201_700, 31, Vec::new());
    line.compares = 3;
    session.results.push(line);
    session.results.push(ExerciseResult::first(
        14,
        1_791_201_800,
        40,
        vec![(Metric::FinalDelta, 0.031_25), (Metric::Corrections, 2.0)],
    ));
    session
}

fn read(key: &[u8], bytes: &[u8]) -> Result<SessionRecord, PortError> {
    decode_session(key, bytes).map(|(session, _)| session)
}

#[test]
fn a_session_round_trips_and_damage_is_refused() {
    let session = session();
    let key = practice_key(7);
    let bytes = encode_session(&session);
    assert_eq!(
        decode_session(&key, &bytes),
        Ok((session.clone(), Owed::default()))
    );
    // As 1.7.0 to 1.16.0 wrote it: no layout last, its sheet owed laying anew (S99).
    let frames = Owed {
        frames: true,
        ..Owed::default()
    };
    let unlaid = &bytes[..bytes.len() - 1];
    assert_eq!(decode_session(&key, unlaid), Ok((session.clone(), frames)));
    assert!(decode_session(&key, &bytes[..bytes.len() - 2]).is_err());
    let mut other = bytes.clone();
    *other.last_mut().expect("its layout") = 2;
    assert!(decode_session(&key, &other).is_err(), "a layout unknown");
    assert!(
        decode_session(&key, &[bytes.as_slice(), &[0]].concat()).is_err(),
        "a byte after the tail"
    );
    assert!(
        decode_session(b"p", &bytes).is_err(),
        "a key without its id"
    );
    let mut unknown = bytes.clone();
    // The first exercise's kind, after the session's fixed fields and the count: none of 1.15.0's.
    unknown[1 + 4 + 8 * 4 + 4 + 4] = 18;
    assert!(decode_session(&key, &unknown).is_err());
}

#[test]
fn estimates_and_points_in_space_round_trip_with_their_errors() {
    use openpaint_practice::{Estimate, Plan, Trick};
    let kinds = [
        Kind::Flat(Estimate::Point),
        Kind::Flat(Estimate::Angle),
        Kind::Space(Trick::Mirror),
    ];
    let plan = Plan {
        exercises: (kinds.iter()).map(|k| Exercise::generated(*k, 8)).collect(),
        warm_up: 0,
    };
    let mut session = SessionRecord::begun(12, (3, 1), (15, 16), plan);
    let point = [(Metric::PointAcross, -0.031), (Metric::PointDown, 0.012)];
    (session.results).push(ExerciseResult::first(0, 17, 3, point.to_vec()));
    (session.results).push(ExerciseResult::first(
        1,
        18,
        5,
        vec![(Metric::AngleOff, 4.5)],
    ));
    (session.results).push(ExerciseResult::first(
        2,
        19,
        9,
        vec![(Metric::SpaceOff, 0.07)],
    ));
    let bytes = encode_session(&session);
    assert_eq!(read(&practice_key(12), &bytes), Ok(session));
}

#[test]
fn solids_round_trip_with_their_error() {
    use openpaint_practice::{Plan, Solid};
    let plan = Plan {
        exercises: (Solid::ALL.iter())
            .map(|s| Exercise::generated(Kind::Solid(*s), 9))
            .collect(),
        warm_up: 0,
    };
    let mut session = SessionRecord::begun(13, (3, 2), (15, 16), plan);
    (session.results).push(ExerciseResult::first(
        2,
        17,
        4,
        vec![(Metric::SolidOff, 0.012)],
    ));
    let bytes = encode_session(&session);
    assert_eq!(read(&practice_key(13), &bytes), Ok(session));
}

#[test]
fn objects_round_trip_with_their_error() {
    use openpaint_practice::{Carve, Plan};
    let plan = Plan {
        exercises: (Carve::ALL.iter())
            .map(|c| Exercise::generated(Kind::Object(*c), 9))
            .collect(),
        warm_up: 0,
    };
    let mut session = SessionRecord::begun(14, (4, 1), (15, 16), plan);
    (session.results).push(ExerciseResult::first(
        3,
        17,
        4,
        vec![(Metric::ObjectOff, 0.009)],
    ));
    let bytes = encode_session(&session);
    assert_eq!(read(&practice_key(14), &bytes), Ok(session));
}

#[test]
fn divisions_whole_copies_and_value_masses_round_trip_with_their_pictures_and_numbers() {
    use openpaint_practice::{ItemRef, Level, Plan, Tier};
    let picture = |item: &str| ItemRef {
        pack: "openpaint-compositions".to_owned(),
        item: item.to_owned(),
    };
    let exercise = |kind: Kind, item: &str| Exercise {
        picture: Some(picture(item)),
        ..Exercise::generated(kind, 9)
    };
    let plan = Plan {
        exercises: vec![
            exercise(Kind::Division(Tier::Simple), "harbour"),
            exercise(Kind::WholeCopy(Tier::Medium), "hills"),
            exercise(Kind::ValueMasses(Tier::Medium), "valley"),
        ],
        warm_up: 0,
    };
    let mut session = SessionRecord::begun(15, (4, 2), (15, 16), plan);
    let counted = vec![
        (Metric::DivisionLines(Level::Main), 4.0),
        (Metric::DivisionLength(Level::Main), 0.81),
        (Metric::DivisionLines(Level::Second), 6.0),
        (Metric::DivisionLength(Level::Second), 0.27),
        (Metric::DivisionLines(Level::Third), 0.0),
    ];
    (session.results).push(ExerciseResult::first(0, 17, 4, counted));
    let matched = vec![(Metric::MassesMatched, 0.83)];
    (session.results).push(ExerciseResult::first(2, 18, 9, matched));
    let bytes = encode_session(&session);
    assert_eq!(read(&practice_key(15), &bytes), Ok(session));
}

#[test]
fn a_record_keeps_its_time_and_one_of_1_6_2_reads_without_it() {
    let mut session = session();
    session.time = vec![
        Span {
            from: 1_791_201_600,
            to: 1_791_202_000,
        },
        Span {
            from: 1_791_210_000,
            to: 1_791_210_090,
        },
    ];
    let key = practice_key(7);
    let bytes = encode_session(&session);
    assert_eq!(
        decode_session(&key, &bytes),
        Ok((session.clone(), Owed::default()))
    );
    // The spans before the layout: their count, then each one's start and end.
    let end = bytes.len() - 1;
    let spans = end - 2 * SPAN_BYTES;
    assert_eq!(bytes[spans - 4..spans], 2u32.to_le_bytes());
    let mut backwards = bytes.clone();
    backwards[end - 8..end].copy_from_slice(&1_791_209_999u64.to_le_bytes());
    assert!(
        decode_session(&key, &backwards).is_err(),
        "a span ending first"
    );
    // As 1.6.2 wrote it: the warm-up's count last, no pictures' count, no time.
    let old = &bytes[..spans - 4 - 4];
    let (read, owed) = decode_session(&key, old).expect("read");
    assert!(owed.time && !owed.scales);
    assert_eq!(read.time, []);
    assert_eq!(read.results, session.results);
}

#[test]
fn a_stage_two_record_keeps_its_pictures_and_one_of_1_6_2_with_pictures_reads_untimed() {
    let mut session = session();
    session.stage = 2;
    session.exercises[1] = Exercise {
        picture: Some(ItemRef {
            pack: "openpaint-shapes".to_owned(),
            item: "teapot-bail".to_owned(),
        }),
        ..Exercise::generated(Kind::Shape(openpaint_practice::Tier::Simple), 9)
    };
    session.exercises[2].picture = Some(ItemRef {
        pack: "personal".to_owned(),
        item: "mine-3".to_owned(),
    });
    session.exercises[2].kind = Kind::Shape(openpaint_practice::Tier::Medium);
    let bytes = encode_session(&session);
    assert_eq!(read(&practice_key(7), &bytes), Ok(session.clone()));
    assert!(decode_session(&practice_key(7), &bytes[..bytes.len() - 2]).is_err());
    // As 1.4.0 to 1.6.2 wrote it: the pictures last.
    let old = &bytes[..bytes.len() - 5];
    assert_eq!(
        decode_session(&practice_key(7), old),
        Ok((
            session.clone(),
            Owed {
                time: true,
                scales: false,
                frames: true
            }
        ))
    );
    let mut far = bytes.clone();
    // The first picture's exercise, past the warm-up's and the pictures' counts.
    let plain = encode_session(&SessionRecord {
        exercises: session
            .exercises
            .iter()
            .map(|e| Exercise::generated(e.kind, e.seed))
            .collect(),
        ..session.clone()
    });
    let at = plain.len() - 9 + 4;
    far[at..at + 4].copy_from_slice(&99u32.to_le_bytes());
    assert!(decode_session(&practice_key(7), &far).is_err());
}

#[test]
fn week_two_s_colour_work_and_a_long_colour_copy_round_trip() {
    use openpaint_practice::{Arrangement, Plan, ScaleTone};
    let plan = Plan {
        exercises: vec![
            Exercise::generated(Kind::Areas(Arrangement::Blocks), 3),
            Exercise::generated(Kind::Scale(ScaleTone::Hue, 9), 4),
        ],
        warm_up: 0,
    };
    let mut session = SessionRecord::begun(9, (2, 4), (11, 12), plan);
    let areas = [(Metric::AreasDelta, 0.04), (Metric::AreasMadeStrong, 1.0)];
    let scale = [(Metric::ScaleOff, 0.03), (Metric::ScaleReversed, 2.0)];
    (session.results).push(ExerciseResult::first(0, 13, 7, areas.to_vec()));
    (session.results).push(ExerciseResult::first(1, 14, 9, scale.to_vec()));
    let bytes = encode_session(&session);
    assert_eq!(read(&practice_key(9), &bytes), Ok(session));
    let picture = ItemRef {
        pack: "openpaint-shapes".to_owned(),
        item: "teapot-bail".to_owned(),
    };
    let mut copy = SessionRecord::project(10, 1, (5, 6), picture);
    copy.time.push(Span { from: 6, to: 900 });
    let read = read(&practice_key(10), &encode_session(&copy));
    assert!(read.as_ref().is_ok_and(SessionRecord::is_project));
    assert_eq!(read, Ok(copy));
}

#[test]
fn a_drill_round_trips_as_a_record_of_stage_drill_its_kind_its_exercises() {
    use openpaint_practice::{DRILL, Family, Plan, Trick};
    let plan = Plan {
        exercises: (0..3)
            .map(|seed| Exercise::generated(Kind::Space(Trick::Thirds), seed))
            .collect(),
        warm_up: 0,
    };
    let mut drill = SessionRecord::drill(14, 2, (21, 22), plan);
    drill.canvas = 30;
    (drill.results).push(ExerciseResult::first(
        1,
        23,
        6,
        vec![(Metric::SpaceOff, 0.04)],
    ));
    drill.time.push(Span { from: 22, to: 400 });
    let bytes = encode_session(&drill);
    // The stage byte first: no field is added for a drill.
    assert_eq!(bytes[0], DRILL);
    let read = read(&practice_key(14), &bytes).expect("read");
    assert_eq!(read.drilled(), Some(Family::Thirds));
    assert!(read.is_drill() && !read.is_session());
    assert_eq!(read, drill);
}

#[test]
fn boxes_and_ellipses_round_trip_with_no_numbers() {
    use openpaint_practice::{BoxView, Guide, Horizon, Plan};
    let kinds = [
        Kind::Box(BoxView::ThreePoint),
        Kind::FreeBoxes(Horizon::Low),
        Kind::Ellipse(Guide::Plane),
    ];
    let plan = Plan {
        exercises: (kinds.iter()).map(|k| Exercise::generated(*k, 5)).collect(),
        warm_up: 1,
    };
    let mut session = SessionRecord::begun(11, (2, 3), (13, 14), plan);
    (session.results).push(ExerciseResult::first(1, 15, 4, Vec::new()));
    let bytes = encode_session(&session);
    assert_eq!(read(&practice_key(11), &bytes), Ok(session));
}

#[test]
fn a_record_of_1_2_0_reads_without_a_warm_up() {
    // As 1.1.1 to 1.2.0 wrote it: the results' tail, then nothing.
    let session = session();
    let bytes = encode_session(&session);
    let old = &bytes[..bytes.len() - END_BYTES];
    let (read, owed) = decode_session(&practice_key(7), old).expect("read");
    assert_eq!((read.warm_up, owed.time), (0, true));
    assert_eq!(read.results, session.results);
    let mut empty = session.clone();
    empty.results.clear();
    let bytes = encode_session(&empty);
    let read = decode_session(&practice_key(7), &bytes[..bytes.len() - END_BYTES]);
    assert_eq!(read.map(|(r, _)| r.warm_up), Ok(0), "no results, no tail");
    assert_eq!(
        decode_session(&practice_key(7), &bytes),
        Ok((empty, Owed::default()))
    );
}

#[test]
fn a_record_of_1_1_0_reads_with_its_line_metrics_passed_over() {
    // As 1.1.0 wrote it: no tail, a line's result with the metrics of codes 1 to 8.
    let session = session();
    let bytes = encode_session(&session);
    let mut old = bytes[..bytes.len() - 2 * TAIL_BYTES - END_BYTES].to_vec();
    // The line's metric count: before the colour's result (its index, time, count and two
    // metrics).
    let metrics = old.len() - (4 + 8 + 4 + 2 * METRIC_BYTES) - 4;
    assert_eq!(
        old[metrics..metrics + 4],
        0u32.to_le_bytes(),
        "the line's count"
    );
    let mut retired = Writer::with_capacity(4 + 8 * METRIC_BYTES);
    retired.len(8);
    for code in 1..=8 {
        retired.u8(code);
        retired.f64(f64::from(code) / 100.0);
    }
    old.splice(metrics..metrics + 4, retired.finish());
    let read = read(&practice_key(7), &old).expect("read");
    let results: Vec<(u32, u32, Option<u64>, usize)> = read
        .results
        .iter()
        .map(|r| (r.index, r.compares, r.corrected_from, r.metrics.len()))
        .collect();
    assert_eq!(results, vec![(4, 1, None, 0), (14, 1, None, 2)]);
}
