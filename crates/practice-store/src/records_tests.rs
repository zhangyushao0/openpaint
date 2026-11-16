use openpaint_practice::{
    BoxView, Carve, Estimate, Guide, Horizon, ItemRef, Lean, LineTier, Planned, ScaleTone, Skill,
    SkillLevel, Solid, Tendency, Tier, Tone, Trick,
};

use super::*;

const NOON: u64 = 1_791_201_600;

/// `exercises` as a lesson picked `picked`, each at the first level.
fn planned(picked: Picked, exercises: Vec<Exercise>) -> Planned {
    Planned {
        picked,
        exercises: (exercises.into_iter())
            .map(|exercise| (exercise, SkillLevel::First))
            .collect(),
    }
}

/// A skill's long lesson part done: two exercises compared (one judged by the app,
/// one by the eye), a picture laid in a frame of its shape, time on it.
fn long_lesson() -> Sheet {
    let mut exercises: Vec<Exercise> = (0..16)
        .map(|seed| Exercise::generated(Kind::Line(LineTier::Arc), seed))
        .collect();
    exercises[1] = Exercise {
        picture: Some(ItemRef {
            pack: "openpaint-shapes".to_owned(),
            item: "teapot-bail".to_owned(),
        }),
        ..Exercise::generated(Kind::Shape(Tier::Simple), 9)
    };
    let tone = Tone::of_code(1).expect("a tone");
    exercises[14] = Exercise::generated(Kind::Colour(tone), 14);
    let mut sheet = Sheet::begun(7, (Purpose::Day, NOON), planned(Picked::Chosen, exercises));
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
    colour.first.as_mut().expect("compared").corrected_from = 0;
    sheet
}

/// `sheet`'s bytes read back under its key.
fn round_trip(sheet: &Sheet) -> Result<Sheet, PortError> {
    decode_sheet(&sheet_key(sheet.id), &encode_sheet(sheet))
}

#[test]
fn a_sheet_round_trips_and_damage_is_refused() {
    let sheet = long_lesson();
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
    let mut untagged = bytes.clone();
    untagged[0] = 1;
    assert!(decode_sheet(&key, &untagged).is_err(), "another tag");
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
    assert_eq!(bytes[picked], 2, "a skill's lesson");
    for way in [1, 3, 6] {
        let mut unpicked = bytes.clone();
        unpicked[picked] = way;
        assert!(
            decode_sheet(&key, &unpicked).is_err(),
            "picked in a way this version does not write: {way}"
        );
    }
    // The first exercise: its kind after the lesson's exercise in hand and count.
    let kind = picked + 1 + 2 + 4;
    for code in [5, 24] {
        let mut unknown = bytes.clone();
        unknown[kind] = code;
        assert!(
            decode_sheet(&key, &unknown).is_err(),
            "a kind this version does not write: {code}"
        );
    }
}

#[test]
fn the_first_lesson_round_trips_with_its_free_scribble_and_a_result_by_eye() {
    // S122's additions, each additive: picked 4, kind (20, 0); the eye's judge (2) and the
    // level were written before it.
    let planned = openpaint_practice::schedule::first(5);
    let mut sheet = Sheet::begun(8, (Purpose::Day, NOON), planned);
    let line = sheet.laid_mut(1).expect("the straight line");
    line.level = 2;
    line.compared(First::of(NOON, 3, Vec::new()));
    line.judged(Outcome::Near);
    assert_eq!(line.result, Some((Outcome::Near, Judge::Eye)));
    assert_eq!(round_trip(&sheet), Ok(sheet.clone()));
    let bytes = encode_sheet(&sheet);
    let picked = 1 + 8 + 8 + 1 + 4 + 4 + 8 + 8;
    assert_eq!(bytes[picked], 4, "the first lesson");
    let kind = picked + 1 + 2 + 4;
    assert_eq!(bytes[kind..kind + 2], [20, 0], "a free scribble");
}

#[test]
fn a_re_test_round_trips_with_its_unit_and_whether_it_is_the_end() {
    // S147's addition, additive: picked 5, then its unit and its end.
    let exercises = vec![Exercise::generated(Kind::Space(Trick::Centre), 2)];
    for end in [false, true] {
        let picked = Picked::Retest { unit: 3, end };
        let sheet = Sheet::begun(9, (Purpose::Day, NOON), planned(picked, exercises.clone()));
        assert_eq!(round_trip(&sheet), Ok(sheet.clone()));
        let bytes = encode_sheet(&sheet);
        let at = 1 + 8 + 8 + 1 + 4 + 4 + 8 + 8;
        assert_eq!(bytes[at..at + 3], [5, 3, u8::from(end)]);
        let mut damaged = bytes.clone();
        damaged[at + 2] = 2;
        assert!(
            decode_sheet(&sheet_key(9), &damaged).is_err(),
            "an end neither"
        );
    }
}

#[test]
fn a_first_compare_s_metric_and_a_result_of_codes_unknown_are_refused() {
    let colour = Exercise::generated(Kind::Colour(Tone::of_code(1).expect("a tone")), 4);
    let picked = Picked::Chosen;
    let mut sheet = Sheet::begun(3, (Purpose::Day, NOON), planned(picked, vec![colour]));
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
        (code, 21, "a metric unknown"),
        (code, 47, "another metric unknown"),
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
    let picked = Picked::Chosen;
    let mut sheet = Sheet::begun(15, (Purpose::Day, NOON), planned(picked, exercises));
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
fn a_skill_s_lesson_a_test_s_and_a_long_colour_copy_round_trip_as_what_they_are() {
    let exercises = (0..3)
        .map(|seed| Exercise::generated(Kind::Space(Trick::Thirds), seed))
        .collect();
    let mut lesson = Sheet::begun(14, (Purpose::Day, NOON), planned(Picked::Chosen, exercises));
    lesson.canvas = 30;
    lesson.spans.push(Span {
        from: NOON,
        to: NOON + 400,
    });
    let skill = Skill::of(Kind::Space(Trick::Thirds));
    let read = round_trip(&lesson).expect("read");
    assert_eq!(read.lesson().and_then(Lesson::skill), skill);
    assert_eq!(read, lesson);
    // A test's lesson (the device chore's, S121b): its purpose's byte 3, a lesson all the same.
    let test = Sheet {
        purpose: Purpose::Test,
        ..lesson
    };
    assert_eq!(encode_sheet(&test)[1 + 8 + 8], 3);
    let read = round_trip(&test).expect("read");
    assert_eq!(read.lesson().and_then(Lesson::skill), skill);
    assert_eq!(read, test);
    let picture = ItemRef {
        pack: "openpaint-shapes".to_owned(),
        item: "teapot-bail".to_owned(),
    };
    let mut copy = Sheet::copy(10, (5, NOON), picture);
    copy.laid_mut(0).expect("the copy").frame = [1200, 900];
    let read = round_trip(&copy).expect("read");
    assert!(read.is_copy() && read.lesson().and_then(Lesson::skill).is_none());
    assert_eq!(read, copy);
    // A drawing from life (S151): a copy's purpose, its exercise of kind 23, no new byte.
    let photo = ItemRef {
        pack: "personal".to_owned(),
        item: "mine-0badf00d".to_owned(),
    };
    let mut life = Sheet::from_life(11, (6, NOON), photo);
    life.laid_mut(0).expect("the drawing").frame = [1600, 1200];
    assert_eq!(encode_sheet(&life)[1 + 8 + 8], 2, "a copy's purpose");
    let read = round_trip(&life).expect("read");
    assert!(read.drawn_from_life() && !copy.drawn_from_life());
    assert_eq!(read, life);
}

#[test]
fn the_project_a_record_names_is_read_alone_whatever_follows() {
    let sheet = long_lesson();
    let mut bytes = encode_sheet(&sheet);
    assert_eq!(named_project(&bytes), Some(12));
    // A lesson picked in a way this version does not read: refused, its project still named.
    let picked = 1 + 8 + 8 + 1 + 4 + 2 * 16 + 4 + 8 + 8;
    bytes[picked] = 3;
    assert!(decode_sheet(&sheet_key(7), &bytes).is_err());
    assert_eq!(named_project(&bytes), Some(12));
    assert_eq!(named_project(&bytes[..9]), Some(12), "the head alone");
    assert_eq!(named_project(&bytes[..8]), None, "too short");
    let mut untagged = bytes.clone();
    untagged[0] = 1;
    assert_eq!(named_project(&untagged), None, "another tag");
    let unplaced = encode_sheet(&Sheet { canvas: 0, ..sheet });
    assert_eq!(named_project(&unplaced), None, "no project");
}
