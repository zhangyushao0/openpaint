use openpaint_practice::{
    Arrangement, BoxView, Carve, Combination, Estimate, Guide, Horizon, ItemRef, Lean, LineTier,
    Plan, ScaleTone, Solid, Tendency, Tier, Trick, stage_one,
};

use super::*;

const NOON: u64 = 1_791_201_600;

/// A first session part done: its warm-up, two exercises compared (one judged by the app, one by
/// the eye), a picture laid in a frame of its shape, time on it.
fn session() -> Sheet {
    let met = [Kind::Line(LineTier::Arc), Kind::Combined(Combination::Near)];
    let mut plan = stage_one(0x5EED_0045, &met);
    plan.exercises[1] = Exercise {
        picture: Some(ItemRef {
            pack: "openpaint-shapes".to_owned(),
            item: "teapot-bail".to_owned(),
        }),
        ..Exercise::generated(Kind::Shape(Tier::Simple), 9)
    };
    let mut sheet = Sheet::session(7, 1, NOON, plan);
    assert_eq!(sheet.warm_up(), 4);
    sheet.canvas = 12;
    sheet.spans = vec![
        Span {
            from: NOON,
            to: NOON + 400,
        },
        Span {
            from: NOON + 9000,
            to: NOON + 9090,
        },
    ];
    let lesson = sheet.lesson_mut().expect("its lesson");
    lesson.at = 15;
    lesson.finished = Some(NOON + 9090);
    let shape = sheet.laid_mut(1).expect("a shape");
    shape.frame = [600, 800];
    shape.level = 2;
    let line = sheet.laid_mut(4).expect("a line");
    line.compared(First::of(NOON + 100, 31, Vec::new()));
    line.compares = 3;
    line.result = Some((Outcome::Near, Judge::Eye));
    line.seconds = 95;
    let colour = sheet.laid_mut(14).expect("a colour");
    let metrics = vec![(Metric::FinalDelta, 0.031_25), (Metric::Corrections, 2.0)];
    colour.compared(First::of(NOON + 200, 40, metrics));
    colour.result = Some((Outcome::Clean, Judge::App));
    colour.first.as_mut().expect("compared").corrected_from = None;
    sheet
}

/// `sheet`'s bytes read back under its key.
fn round_trip(sheet: &Sheet) -> Result<Sheet, PortError> {
    decode_sheet(&sheet_key(sheet.id), &encode_sheet(sheet))
}

#[test]
fn a_sheet_round_trips_and_damage_is_refused() {
    let sheet = session();
    assert_eq!(round_trip(&sheet).as_ref(), Ok(&sheet));
    let key = sheet_key(7);
    let bytes = encode_sheet(&sheet);
    assert_eq!(bytes[0], SHEET, "the tag first");
    assert!(
        decode_sheet(&key, &bytes[..bytes.len() - 1]).is_err(),
        "cut short"
    );
    assert!(
        decode_sheet(&key, &[bytes.as_slice(), &[0]].concat()).is_err(),
        "a byte after its last exercise"
    );
    assert!(decode_sheet(b"p", &bytes).is_err(), "a key without its id");
    let mut stage = bytes.clone();
    stage[0] = 1;
    assert!(decode_sheet(&key, &stage).is_err(), "a record of 1.21.6");
    let mut purpose = bytes.clone();
    purpose[1 + 8 + 8] = 4;
    assert!(decode_sheet(&key, &purpose).is_err(), "a purpose unknown");
    // The spans: their count after the purpose, then the first one's start and end.
    let spans = 1 + 8 + 8 + 1;
    assert_eq!(bytes[spans..spans + 4], 2u32.to_le_bytes());
    let mut backwards = bytes.clone();
    let end = spans + 4 + 8;
    backwards[end..end + 8].copy_from_slice(&(NOON - 1).to_le_bytes());
    assert!(
        decode_sheet(&key, &backwards).is_err(),
        "a span ending first"
    );
    // The lesson: its count after the spans, then started, finished and how it was picked.
    let lessons = spans + 4 + 2 * 16;
    let mut none = bytes[..lessons].to_vec();
    none.extend_from_slice(&0u32.to_le_bytes());
    assert!(decode_sheet(&key, &none).is_err(), "no lesson");
    let picked = lessons + 4 + 8 + 8;
    assert_eq!(
        bytes[picked..picked + 3],
        [3, 1, 4],
        "a stage-1 session, four warming up"
    );
    let mut unpicked = bytes.clone();
    unpicked[picked] = 1;
    assert!(
        decode_sheet(&key, &unpicked).is_err(),
        "picked of a way not written yet"
    );
    // The first exercise: its kind after the lesson's exercise in hand and count.
    let kind = picked + 3 + 2 + 4;
    let mut unknown = bytes.clone();
    unknown[kind] = 20;
    assert!(
        decode_sheet(&key, &unknown).is_err(),
        "a kind none of 1.21.6's"
    );
}

#[test]
fn a_first_compare_s_metric_and_a_result_of_codes_unknown_are_refused() {
    let mut sheet = Sheet::session(
        3,
        2,
        NOON,
        Plan {
            exercises: vec![Exercise::generated(
                Kind::Colour(openpaint_practice::Tone::of_code(1).expect("a tone")),
                4,
            )],
            warm_up: 0,
        },
    );
    let laid = sheet.laid_mut(0).expect("a colour");
    laid.compared(First::of(NOON, 9, vec![(Metric::FinalDelta, 0.5)]));
    laid.result = Some((Outcome::Redo, Judge::App));
    let bytes = encode_sheet(&sheet);
    assert_eq!(round_trip(&sheet), Ok(sheet.clone()));
    // From the end: seconds (u32), the judge, the outcome, the compares (u32), the metric's
    // value (f64) and code.
    let judge = bytes.len() - 4 - 1;
    let (outcome, code) = (judge - 1, judge - 1 - 4 - 8 - 1);
    assert_eq!((bytes[outcome], bytes[judge]), (3, 1));
    assert_eq!(bytes[code], Metric::FinalDelta.code());
    let key = sheet_key(3);
    for (at, byte, what) in [
        (code, 21, "a retired metric"),
        (outcome, 4, "an outcome unknown"),
        (judge, 3, "a judge unknown"),
    ] {
        let mut damaged = bytes.clone();
        damaged[at] = byte;
        assert!(decode_sheet(&key, &damaged).is_err(), "{what}");
    }
}

/// An exercise of most kinds, a picture's with its picture.
fn many_kinds() -> Vec<Exercise> {
    let picture = |item: &str| ItemRef {
        pack: "openpaint-compositions".to_owned(),
        item: item.to_owned(),
    };
    let pictured = |kind: Kind, item: &str| Exercise {
        picture: Some(picture(item)),
        ..Exercise::generated(kind, 9)
    };
    let mut exercises: Vec<Exercise> = [
        Kind::Flat(Estimate::Point),
        Kind::Flat(Estimate::Angle),
        Kind::Space(Trick::Mirror),
        Kind::Box(BoxView::ThreePoint),
        Kind::FreeBoxes(Horizon::Low),
        Kind::Ellipse(Guide::Plane),
        Kind::Areas(Arrangement::Blocks),
        Kind::Scale(ScaleTone::Hue, 9),
    ]
    .into_iter()
    .map(|kind| Exercise::generated(kind, 8))
    .collect();
    exercises.extend(
        Solid::ALL
            .iter()
            .map(|s| Exercise::generated(Kind::Solid(*s), 9)),
    );
    exercises.extend(
        Carve::ALL
            .iter()
            .map(|c| Exercise::generated(Kind::Object(*c), 9)),
    );
    exercises.extend([
        pictured(Kind::Division(Tier::Simple), "harbour"),
        pictured(Kind::WholeCopy(Tier::Medium), "hills"),
        pictured(Kind::ValueMasses(Tier::Medium), "valley"),
        pictured(
            Kind::KeyShift(Tendency {
                lean: Lean::WarmPure,
                hue: 350,
            }),
            "rust",
        ),
    ]);
    exercises
}

#[test]
fn every_kind_s_exercises_round_trip_with_their_pictures_and_numbers() {
    let exercises = many_kinds();
    let count = exercises.len();
    let mut sheet = Sheet::session(
        15,
        4,
        NOON,
        Plan {
            exercises,
            warm_up: 0,
        },
    );
    for index in 0..count {
        let laid = sheet.laid_mut(index).expect("an exercise");
        laid.compared(First::of(
            NOON + index as u64,
            4,
            vec![(Metric::SpaceOff, 0.07)],
        ));
        laid.frame = [u16::try_from(index).expect("small"), 300];
    }
    assert_eq!(round_trip(&sheet), Ok(sheet));
}

#[test]
fn a_drill_a_test_s_and_a_long_colour_copy_round_trip_as_what_they_are() {
    let plan = Plan {
        exercises: (0..3)
            .map(|seed| Exercise::generated(Kind::Space(Trick::Thirds), seed))
            .collect(),
        warm_up: 0,
    };
    let mut drill = Sheet::drill(14, NOON, plan);
    drill.canvas = 30;
    drill.spans.push(Span {
        from: NOON,
        to: NOON + 400,
    });
    let read = round_trip(&drill).expect("read");
    assert_eq!(read.drilled(), Some(openpaint_practice::Family::Thirds));
    assert_eq!(read, drill);
    // A test's drill (the device chore's, S121b): its purpose's byte 3, a drill all the same.
    let test = Sheet {
        purpose: Purpose::Test,
        ..drill
    };
    assert_eq!(encode_sheet(&test)[1 + 8 + 8], 3);
    let read = round_trip(&test).expect("read");
    assert_eq!(read.drilled(), Some(openpaint_practice::Family::Thirds));
    assert_eq!(read, test);
    let picture = ItemRef {
        pack: "openpaint-shapes".to_owned(),
        item: "teapot-bail".to_owned(),
    };
    let mut copy = Sheet::copy(10, (5, NOON), picture);
    copy.laid_mut(0).expect("the copy").frame = [1200, 900];
    let read = round_trip(&copy).expect("read");
    assert!(read.is_copy() && read.drilled().is_none() && read.stage().is_none());
    assert_eq!(read, copy);
}
