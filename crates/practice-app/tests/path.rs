//! The path in the flow (S122): what the log shows of the path before anything is practised and
//! as the records and packs change, the lessons it starts (`Start`), and the first lesson as the
//! bar, its scribble and its end show it.

use std::sync::Arc;

use openpaint_app::{BrushLibrary, ProjectId, Session, Title, Toolbox, Viewport};
use openpaint_core::{Artwork, CanvasSize, Color, StrokeStyle};
use openpaint_practice::{
    Given, ItemRef, Kind, Layout, Needs, Outcome, Pack, PackItem, Picked, PictureKind, Sheet,
    Skill, SkillLevel, Tier,
};
use openpaint_practice_app::{
    PracticeIntent, PracticeLog, PracticePens, PracticeProjection, Start, open_sheet, practise,
};

const VIEWPORT: Viewport = Viewport {
    origin: [0.0, 84.0],
    size: [2880.0, 1716.0],
    pixels_per_point: 1.75,
};
/// 2026-10-05 12:00 UTC.
const NOON: u64 = 1_791_201_600;

fn lines() -> Skill {
    Skill::of_key("line").expect("the lines' skill")
}

/// `record` open as `log` opens it, on a sheet laid ahead for it.
fn opened(record: &Sheet, log: &PracticeLog) -> Session {
    let size = Layout::laid_ahead(record);
    let sheet = Artwork::new(CanvasSize::new(size[0], size[1]).expect("size"));
    let library = BrushLibrary::built_in().expect("built-in brushes");
    let style = Toolbox::new(library).style().expect("style");
    let brush =
        StrokeStyle::new(Arc::clone(style.recipe()), 8.0, 1.0, Color::BLACK).expect("brush");
    let pens = PracticePens::built_in().expect("pens");
    let printed = Given::of_sheet(record, &[]);
    let opening = log.practising(record, &printed, (size, &Title::untitled()));
    open_sheet(sheet, (brush, pens), opening, &VIEWPORT)
}

fn shown(session: &Session) -> PracticeProjection {
    PracticeProjection::of(session).expect("a sheet")
}

#[test]
fn before_anything_is_practised_the_path_offers_the_first_lesson() {
    let log = PracticeLog::new(Vec::new(), Vec::new(), Vec::new());
    let path = log.path();
    assert_eq!(path.units.len(), 8);
    assert!(path.units[0].open && !path.units[1].open);
    let first: Vec<(Skill, u8)> = (path.units[0].skills.iter())
        .map(|skill| (skill.skill, skill.dots))
        .collect();
    assert_eq!(first.len(), 3);
    assert_eq!(first[0], (lines(), 0));
    assert!(path.units[5].skills.is_empty(), "a unit not built yet");
    // Without a pack a single shape lacks its pictures.
    assert!(path.units[1].skills[0].lacks.is_some());
    assert!(path.first && path.next.is_none() && path.review.is_none());
    assert_eq!((path.unit, path.open), (1, None));
    assert!(log.learner().fresh());
}

#[test]
fn a_lesson_starts_as_the_first_a_skill_s_at_a_level_or_a_unit_s_full_one() {
    let log = PracticeLog::new(Vec::new(), Vec::new(), Vec::new());
    let picked = |start: Start| {
        let (sheet, title) = log.start(start, 5, NOON)?;
        let lesson = &sheet.lessons[0];
        let levels: Vec<u8> = lesson.exercises.iter().map(|laid| laid.level).collect();
        Some((lesson.picked, lesson.skill(), levels, title))
    };
    // Nothing practised: Start starts the first lesson, a scribble, a line and an arc.
    let (first, skill, levels, title) = picked(Start::Next).expect("the first lesson");
    assert_eq!((first, skill, levels), (Picked::First, None, vec![0; 3]));
    assert_eq!(title.as_str(), "Lesson 1");
    assert_eq!(
        picked(Start::First).map(|first| first.0),
        Some(Picked::First)
    );
    // A skill tapped: its lesson at the learner's level; at a level given, at that one.
    let (chosen, skill, levels, _) = picked(Start::Skill(lines())).expect("a lesson of lines");
    assert_eq!((chosen, skill), (Picked::Chosen, Some(lines())));
    assert!(!levels.is_empty() && levels.iter().all(|level| *level == 0));
    let at = Start::At(lines(), SkillLevel::Third);
    let (_, _, levels, _) = picked(at).expect("a lesson at a level");
    assert!(
        levels
            .iter()
            .all(|level| *level == SkillLevel::Third.code())
    );
    // A unit's full lesson; none for a unit not built, a review without a redo or a skill
    // whose pictures are not kept.
    let full = picked(Start::Full(1)).map(|full| full.0);
    assert!(matches!(full, Some(Picked::Full { stage: 1, .. })));
    let shapes = Skill::of_key("shape").expect("the shapes' skill");
    for none in [Start::Full(7), Start::Review, Start::Skill(shapes)] {
        assert!(picked(none).is_none(), "{none:?}");
    }
}

#[test]
fn the_lesson_in_hand_is_named_judged_and_ended() {
    let log = PracticeLog::new(Vec::new(), Vec::new(), Vec::new());
    let (first, _) = log.start(Start::Next, 5, NOON).expect("the first lesson");
    let first = Sheet {
        canvas: 40,
        ..first
    };
    let mut session = opened(&first, &log);
    let practice = shown(&session);
    let lesson = practice.lesson.expect("the lesson in hand");
    assert!(lesson.row.first && lesson.row.skill.is_none());
    assert_eq!((lesson.row.number, lesson.row.slot), (1, 0));
    assert_eq!((lesson.place, lesson.level), ((1, 3), SkillLevel::First));
    assert_eq!(
        (practice.kind, practice.how),
        (Some(Kind::Free), Some(Kind::Free))
    );
    assert_eq!(practice.sheet.kinds.len(), 3);
    assert_eq!(practice.sheet.rows, [lesson.row]);
    assert!(practice.judged.is_none() && practice.end.is_none());
    // A scribble has no Compare and no result; none is chosen while nothing is compared.
    assert!(!practice.compare.is_available());
    assert!(practise(&mut session, PracticeIntent::Compare(true), NOON + 5).is_err());
    let judge = PracticeIntent::Judge(Outcome::Clean);
    assert!(practise(&mut session, judge, NOON + 10).is_err());
    for place in 2..=3 {
        practise(&mut session, PracticeIntent::Next(VIEWPORT), NOON + 20).expect("next");
        let lesson = shown(&session).lesson.expect("the lesson in hand");
        assert_eq!(lesson.place, (place, 3));
    }
    practise(&mut session, PracticeIntent::Next(VIEWPORT), NOON + 30).expect("its end");
    let practice = shown(&session);
    let end = practice.end.expect("the lesson's end");
    assert_eq!(end.results, [(0, None), (1, None), (2, None)]);
    assert_eq!(end.skill, None, "the first lesson's");
    assert!(end.next.is_some(), "the lesson One more starts");
    assert!(practice.kind.is_none() && practice.lesson.is_some());
}

/// A pack of one shape's picture.
fn shapes() -> Pack {
    Pack {
        id: "test-shapes".to_owned(),
        title: "Test shapes".to_owned(),
        items: vec![PackItem {
            id: "teapot".to_owned(),
            kind: PictureKind::Shape,
            tier: Tier::Simple,
            size: [800, 600],
            title: "Teapot".to_owned(),
            credit: None,
            outline: None,
            masses: None,
            areas: None,
        }],
    }
}

#[test]
fn the_path_follows_the_log_as_lessons_and_packs_are_kept() {
    let mut log = PracticeLog::new(Vec::new(), Vec::new(), Vec::new());
    let shape = Skill::of_key("shape").expect("the shapes' skill");
    let tile = |log: &PracticeLog, skill: Skill| {
        let path = log.path();
        let found = (path.units.iter().flat_map(|unit| &unit.skills)).find(|t| t.skill == skill);
        found.copied().expect("its tile")
    };
    assert_eq!(tile(&log, shape).lacks, Some(Needs::Pictures));
    // The first lesson kept, unfinished: its sheet is what Start continues.
    let (first, _) = log.start(Start::Next, 5, NOON).expect("the first lesson");
    log.keep(Sheet {
        canvas: 40,
        ..first
    });
    let path = log.path();
    assert!(!path.first && !log.learner().fresh());
    assert_eq!(path.open, Some(ProjectId::new(40)));
    let next = path.next.expect("the next lesson");
    assert_eq!((next.level, next.redo), (SkillLevel::First, false));
    assert!(path.review.is_none(), "no redo waits");
    // Finished: nothing left open; the book names its row.
    let mut done = log.session(1).expect("kept").clone();
    done.lessons[0].finished = Some(NOON + 300);
    log.keep(done);
    let path = log.path();
    assert_eq!(path.open, None);
    let rows = &path.book.record(1).expect("in the book").rows;
    assert!(rows.len() == 1 && rows[0].first && rows[0].number == 1);
    // A pack kept: shapes lack nothing and a lesson of them lays its picture.
    log.set_packs(vec![shapes()]);
    assert_eq!(tile(&log, shape).lacks, None);
    let (lesson, title) = (log.start(Start::Skill(shape), 6, NOON)).expect("a lesson of shapes");
    assert_eq!(title.as_str(), "Lesson 2");
    let teapot = ItemRef {
        pack: "test-shapes".to_owned(),
        item: "teapot".to_owned(),
    };
    let laid = lesson.exercises().next().expect("an exercise");
    assert_eq!(laid.exercise.picture.as_ref(), Some(&teapot));
}
