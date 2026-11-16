//! A lesson of a skill on its sheet (S122): where the eye judges, the user chooses a result after
//! Compare, once, and Next without one leaves none; where the app judges, its result stands; a
//! lesson's end shows each exercise's result, the skill's dots before it and after it, and the
//! lesson One more starts, the path's next once the log keeps the lesson; a redo waits on the path
//! and the review lays it again.

use std::sync::Arc;

use openpaint_app::{
    BrushLibrary, Contact, ContactPhase, Device, InputSample, Intent, StrokeIntent, Title, Toolbox,
    Viewport,
};
use openpaint_core::{Artwork, CanvasSize, Color, StrokeStyle};
use openpaint_practice::{
    Given, Judge, Layout, Outcome, Picked, Planned, Purpose, Sheet, Skill, SkillLevel, Verdict,
    schedule,
};
use openpaint_practice_app::{
    OneMore, PracticeIntent, PracticeLog, PracticePens, PracticeProjection, SheetSession, Start,
};

const VIEWPORT: Viewport = Viewport {
    origin: [0.0, 84.0],
    size: [2880.0, 1716.0],
    pixels_per_point: 1.75,
};
/// 2026-10-05 12:00 UTC.
const NOON: u64 = 1_791_201_600;

fn skill(key: &str) -> Skill {
    Skill::of_key(key).expect("a skill of that key")
}

/// The lesson of `key` at the first level `log` starts, on sheet `canvas`.
fn lesson_of(log: &PracticeLog, key: &str, canvas: u64) -> Sheet {
    let start = Start::At(skill(key), SkillLevel::First);
    let (sheet, _) = log.start(start, 7, NOON).expect("a lesson");
    Sheet { canvas, ..sheet }
}

/// `record` open as `log` opens it, on a sheet laid ahead for it.
fn opened(record: &Sheet, log: &PracticeLog) -> SheetSession {
    let size = Layout::laid_ahead(record);
    let sheet = Artwork::new(CanvasSize::new(size[0], size[1]).expect("size"));
    let library = BrushLibrary::built_in().expect("built-in brushes");
    let style = Toolbox::new(library).style().expect("style");
    let brush =
        StrokeStyle::new(Arc::clone(style.recipe()), 8.0, 1.0, Color::BLACK).expect("brush");
    let pens = PracticePens::built_in().expect("pens");
    let printed = Given::of_sheet(record, log.packs());
    let opening = log.practising(record, &printed, (size, &Title::untitled()));
    SheetSession::open(sheet, (brush, pens), opening, &VIEWPORT)
}

fn shown(session: &SheetSession) -> PracticeProjection {
    session.projection()
}

fn record_of(session: &SheetSession) -> Sheet {
    session.sheet().record().clone()
}

/// A pen stroke across the shown exercise's copy frame, then Compare.
fn compared(session: &mut SheetSession) {
    let practice = shown(session);
    let frame = practice.sheet.pairs[practice.current.expect("shown")].copy;
    let points: Vec<[f32; 2]> = (0..=20)
        .map(|k| {
            frame
                .to_canvas([0.3 + 0.02 * f64::from(k), 0.5])
                .map(|v| v as f32)
        })
        .collect();
    let last = points.len() - 1;
    let sample = |index: usize| InputSample {
        position: points[index],
        pressure: 0.6,
        tilt: 0.2,
        azimuth: 0.0,
        time_ms: 1000 + 8 * index as i64,
    };
    for (phase, samples) in [
        (ContactPhase::Down, vec![sample(0)]),
        (ContactPhase::Move, (1..last).map(sample).collect()),
        (ContactPhase::Up, vec![sample(last)]),
    ] {
        let contact = Contact {
            id: 7,
            device: Device::Pen,
            phase,
            samples,
        };
        let intent = Intent::Stroke(StrokeIntent::Contact(contact));
        session.dispatch(intent).expect("dispatch");
    }
    session
        .practise(PracticeIntent::Compare(true), NOON + 60)
        .expect("compared");
}

fn next(session: &mut SheetSession) {
    session
        .practise(PracticeIntent::Next(VIEWPORT), NOON + 120)
        .expect("next");
}

#[test]
fn a_line_is_judged_by_eye_once_and_next_without_a_choice_leaves_no_result() {
    let log = PracticeLog::new(Vec::new(), Vec::new(), Vec::new());
    let record = lesson_of(&log, "line", 40);
    let mut session = opened(&record, &log);
    let judge = |outcome| PracticeIntent::Judge(outcome);
    assert!(
        session.practise(judge(Outcome::Clean), NOON).is_err(),
        "before Compare"
    );
    compared(&mut session);
    let judged = shown(&session).judged.expect("judged while compared");
    assert_eq!((judged.verdict, judged.result), (Verdict::Eye, None));
    session
        .practise(judge(Outcome::Near), NOON + 70)
        .expect("chosen");
    let judged = shown(&session).judged.expect("judged while compared");
    assert_eq!(judged.result, Some((Outcome::Near, Judge::Eye)));
    assert!(
        session.practise(judge(Outcome::Clean), NOON + 80).is_err(),
        "once"
    );
    let saved = session.sheet_mut().save_request();
    let result = saved.and_then(|sheet| sheet.laid(0).and_then(|laid| laid.result));
    assert_eq!(
        result,
        Some((Outcome::Near, Judge::Eye)),
        "kept with the record"
    );
    // The second compared and left with Next unjudged: nothing asked, nothing kept.
    next(&mut session);
    compared(&mut session);
    next(&mut session);
    let record = record_of(&session);
    assert_eq!(record.laid(1).and_then(|laid| laid.result), None);
    // The bar's place: in the lesson, its level the first.
    let lesson = shown(&session).lesson.expect("the lesson in hand");
    let held = record.lessons[0].exercises.len();
    assert_eq!(
        (lesson.level, lesson.row.skill),
        (SkillLevel::First, Some(skill("line")))
    );
    if held > 2 {
        assert_eq!(lesson.place, (3, held));
    }
}

#[test]
fn where_the_app_judges_its_result_stands_and_no_choice_is_taken() {
    let log = PracticeLog::new(Vec::new(), Vec::new(), Vec::new());
    let record = lesson_of(&log, "colour", 40);
    let mut session = opened(&record, &log);
    let Some([r, g, b]) = shown(&session).original.as_deref().and_then(|o| o.swatch()) else {
        panic!("a colour")
    };
    let c = |v: u8| f32::from(v) / 255.0;
    let exact = Color::rgb(c(r), c(g), c(b)).expect("colour");
    session
        .practise(PracticeIntent::Fill(exact), NOON)
        .expect("filled");
    session
        .practise(PracticeIntent::Compare(true), NOON + 10)
        .expect("compared");
    let judged = shown(&session).judged.expect("judged");
    assert_eq!(judged.verdict, Verdict::App(Outcome::Clean));
    assert_eq!(judged.result, Some((Outcome::Clean, Judge::App)));
    let judge = PracticeIntent::Judge(Outcome::Redo);
    assert!(
        session.practise(judge, NOON + 20).is_err(),
        "the app's stands"
    );
}

#[test]
fn the_end_shows_each_result_the_dot_earned_and_the_lesson_one_more_starts() {
    // Five clean lines judged before: one more clean one fills the window and earns the dot.
    let mut before = lesson_of(&PracticeLog::default(), "line", 30);
    let held = before.lessons[0].exercises.len().min(5);
    let mut five: Vec<_> = before.lessons[0].exercises[..held].to_vec();
    while five.len() < 5 {
        five.push(five[0].clone());
    }
    for laid in &mut five {
        laid.result = Some((Outcome::Clean, Judge::App));
    }
    before.lessons[0].exercises = five;
    before.lessons[0].finished = Some(NOON - 600);
    let mut log = PracticeLog::new(vec![before], Vec::new(), Vec::new());
    assert_eq!(log.learner().dots(skill("line")), 0);
    let record = lesson_of(&log, "line", 40);
    let mut session = opened(&record, &log);
    compared(&mut session);
    session
        .practise(PracticeIntent::Judge(Outcome::Clean), NOON + 70)
        .expect("chosen");
    assert!(shown(&session).end.is_none(), "not done yet");
    assert!(
        shown(&session).marked.is_empty(),
        "marked once the lesson ends"
    );
    while shown(&session).current.is_some() {
        next(&mut session);
    }
    // The sheet marks the result under its pair, and keeps it there under the lessons after.
    assert_eq!(shown(&session).marked, [(0, Outcome::Clean)]);
    let end = shown(&session).end.expect("the lesson's end");
    assert_eq!(end.results.len(), record.lessons[0].exercises.len());
    assert_eq!(end.results[0], (0, Some(Outcome::Clean)));
    assert!(end.results[1..].iter().all(|(_, result)| result.is_none()));
    assert_eq!(end.skill, Some((skill("line"), 0, 1)), "a level reached");
    let Some(OneMore::Skill(offered)) = end.next else {
        panic!("the lesson One more starts");
    };
    log.keep(record_of(&session));
    assert_eq!(
        log.path().next,
        Some(offered),
        "the path's next, as One more starts it"
    );
    let (more, _) = log.start(Start::Next, 9, NOON + 300).expect("One more");
    assert_eq!(more.lessons[0].skill(), Some(offered.skill));
}

#[test]
fn one_more_after_a_skill_of_a_unit_ahead_of_the_learner_s_offers_that_skill_again() {
    // A box tapped in unit 2 while unit 1 is in hand: One more is boxes again, not Lines.
    let log = PracticeLog::new(Vec::new(), Vec::new(), Vec::new());
    let (record, _) = log
        .start(Start::Skill(skill("box")), 7, NOON)
        .expect("boxes");
    let record = Sheet {
        canvas: 40,
        ..record
    };
    let mut session = opened(&record, &log);
    while shown(&session).current.is_some() {
        next(&mut session);
    }
    let end = shown(&session).end.expect("the lesson's end");
    let Some(OneMore::Skill(offered)) = end.next else {
        panic!("the lesson One more starts");
    };
    assert_eq!(
        (offered.skill, offered.level),
        (skill("box"), SkillLevel::First)
    );
    assert_eq!(
        log.learner().unit(&openpaint_practice::Pictures::default()),
        1
    );
}

#[test]
fn a_redo_waits_on_the_path_and_the_review_lays_it_again_first() {
    // A colour lesson whose first exercise ended Again.
    let mut colour = lesson_of(&PracticeLog::default(), "colour", 30);
    colour.lessons[0].exercises[0].result = Some((Outcome::Redo, Judge::App));
    colour.lessons[0].finished = Some(NOON - 600);
    let again = colour.lessons[0].exercises[0].exercise.clone();
    let log = PracticeLog::new(vec![colour], Vec::new(), Vec::new());
    let path = log.path();
    let tile = (path.units.iter().flat_map(|unit| &unit.skills))
        .find(|tile| tile.skill == skill("colour"))
        .expect("the colours' tile");
    assert!(tile.redo);
    let review = path.review.expect("the review on the side");
    assert_eq!((review.skill, review.redo), (skill("colour"), true));
    assert_ne!(path.next.map(|next| next.skill), Some(skill("colour")));
    // The review lays the same kind, its variant too, and picture first.
    let (reviewed, _) = log.start(Start::Review, 11, NOON).expect("the review");
    let laid = &reviewed.lessons[0].exercises[0].exercise;
    assert_eq!((laid.kind, &laid.picture), (again.kind, &again.picture));
    // A lesson of the skill offers the redo; S122a's `chosen` lays it first.
    let pictures = openpaint_practice::Pictures::default();
    let standing = openpaint_practice::Standing {
        pictures: &pictures,
        learner: log.learner(),
    };
    let offer = schedule::offer(skill("colour"), &standing).expect("an offer");
    assert!(offer.redo);
}

#[test]
fn a_lesson_laid_by_hand_with_a_redo_first_starts_on_it() {
    // What `chosen` plans (S122a) is what the sheet lays: a redo first, at the skill's level.
    let log = PracticeLog::new(Vec::new(), Vec::new(), Vec::new());
    let lesson = lesson_of(&log, "colour", 0);
    let mut exercises: Vec<_> = (lesson.lessons[0].exercises.iter())
        .map(|laid| (laid.exercise.clone(), SkillLevel::First))
        .collect();
    let redo = exercises[exercises.len() - 1].clone();
    exercises.insert(0, redo.clone());
    let planned = Planned {
        picked: Picked::Chosen,
        exercises,
    };
    let record = Sheet {
        canvas: 40,
        ..Sheet::begun(1, (Purpose::Day, NOON), planned)
    };
    let session = opened(&record, &log);
    let practice = shown(&session);
    assert_eq!(practice.kind, Some(redo.0.kind));
    let lesson = practice.lesson.expect("the lesson in hand");
    assert_eq!(
        (lesson.row.skill, lesson.place.0),
        (Some(skill("colour")), 1)
    );
}
