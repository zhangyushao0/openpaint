//! Drills in the app (S91): the practice options' tiles from the log (pencil until a kind is
//! compared anywhere, a drill left unfinished to continue), a drill begun of one kind and numbered
//! among its kind's, never counted as a session (the Today card, the stage, the next session's
//! number and the summary's earlier sessions untouched), its How first every time it opens, and
//! its finish without a road.

use std::sync::Arc;

use openpaint_app::{
    BrushLibrary, DrillShown, PracticeIntent, PracticeLog, PracticePens, ProjectId, Session,
    SheetView, Title, Toolbox, Viewport,
};
use openpaint_core::{Artwork, CanvasSize, Color, StrokeStyle};
use openpaint_practice::{
    DRILL, ExerciseResult, Family, Kind, STAGES_READY, SessionRecord, Sheet, Tally, Trick, kinds_of,
};

const VIEWPORT: Viewport = Viewport {
    origin: [0.0, 84.0],
    size: [2880.0, 1716.0],
    pixels_per_point: 1.75,
};
/// 2026-10-05 12:00 UTC.
const NOON: u64 = 1_791_201_600;

/// `record` kept with a sheet (project `40 + id`), its first exercise compared.
fn compared(mut record: SessionRecord) -> SessionRecord {
    record.canvas = 40 + record.id;
    record
        .results
        .push(ExerciseResult::first(0, NOON + 60, 9, Vec::new()));
    record
}

fn tile(log: &PracticeLog, family: Family) -> DrillShown {
    let today = log.today();
    let found = today.drills.iter().find(|tile| tile.family == family);
    *found.expect("a tile")
}

#[test]
fn the_tiles_show_each_ready_stage_s_kinds_inked_once_compared_and_a_drill_to_continue() {
    let mut log = PracticeLog::new(Vec::new(), Vec::new(), Vec::new());
    let today = log.today();
    let all: Vec<Family> = (1..=STAGES_READY)
        .flat_map(|stage| kinds_of(stage).iter().copied())
        .collect();
    assert_eq!(
        today.drills.iter().map(|t| t.family).collect::<Vec<_>>(),
        all
    );
    assert!(today.drills.iter().all(|t| !t.compared && t.open.is_none()));
    assert_eq!(tile(&log, Family::Centre).stage, 3);
    // A centre drill begun, one compared, left unfinished: inked and continued from its tile.
    let (record, title) = log.begin_drill(Family::Centre, 7, NOON).expect("a drill");
    assert_eq!(title.as_str(), "Drill 1");
    assert_eq!((record.stage, record.number, record.id), (DRILL, 1, 1));
    assert_eq!(record.exercises.len(), 3);
    assert!(
        record
            .exercises
            .iter()
            .all(|e| e.kind == Kind::Space(Trick::Centre))
    );
    log.keep(compared(record));
    let centre = tile(&log, Family::Centre);
    assert!(centre.compared);
    assert_eq!(centre.open, Some(ProjectId::new(41)));
    assert!(!tile(&log, Family::Halves).compared);
    // Never a session: the Today card, the stage and the next session as before.
    let today = log.today();
    assert_eq!((today.stage, today.done, today.open), (1, 0, None));
    let (session, _) = log.begin((1, None), 9, NOON + 600);
    assert_eq!((session.stage, session.number, session.id), (1, 1, 2));
    // A session comparing a line inks Line; the next centre drill is the second.
    log.keep(compared(session));
    assert!(tile(&log, Family::Line).compared);
    let (second, _) = log
        .begin_drill(Family::Centre, 8, NOON + 900)
        .expect("a drill");
    assert_eq!((second.number, second.id), (2, 3));
    // Shapes need a picture: no shapes drill without a pack.
    assert_eq!(log.begin_drill(Family::Shape, 9, NOON), None);
    assert_eq!(log.begin_drill(Family::Study, 9, NOON), None);
}

#[test]
fn a_drill_s_summary_sets_it_beside_earlier_drills_of_its_kind_only() {
    let mut log = PracticeLog::new(Vec::new(), Vec::new(), Vec::new());
    let finished = |mut record: SessionRecord| {
        record.finished = Some(NOON + 1200);
        compared(record)
    };
    let (first, _) = log.begin_drill(Family::Colour, 1, NOON).expect("a drill");
    log.keep(finished(first));
    let (session, _) = log.begin((1, None), 2, NOON);
    log.keep(finished(session));
    let (line, _) = log.begin_drill(Family::Line, 3, NOON).expect("a drill");
    log.keep(finished(line));
    let (second, _) = log.begin_drill(Family::Colour, 4, NOON).expect("a drill");
    let before: Vec<u64> = log.before(&second).iter().map(|r| r.id).collect();
    assert_eq!(before, [1], "the colour drill only");
    let (next, _) = log.begin((1, None), 5, NOON);
    let before: Vec<u64> = log.before(&next).iter().map(|r| r.id).collect();
    assert_eq!(before, [2], "the session only");
    assert_eq!(log.today().done, 1);
}

fn practising(record: SessionRecord, before: Vec<SessionRecord>) -> Session {
    let [width, height] = Sheet::of(&vec![None; record.exercises.len()]).size;
    let sheet = Artwork::new(CanvasSize::new(width, height).expect("size"));
    let library = BrushLibrary::built_in().expect("built-in brushes");
    let style = Toolbox::new(library.clone()).style().expect("style");
    let brush =
        StrokeStyle::new(Arc::clone(style.recipe()), 8.0, 1.0, Color::BLACK).expect("brush");
    let pens = PracticePens::of(&library).expect("pens");
    let view = SheetView::of(&record, &[], ([width, height], &Title::untitled()));
    let opening = (record, before, view, Tally::default());
    Session::practising(sheet, (brush, pens), opening, &VIEWPORT)
}

#[test]
fn a_drill_shows_its_how_first_every_time_and_finishes_without_a_road() {
    let log = PracticeLog::new(Vec::new(), Vec::new(), Vec::new());
    let (record, _) = log.begin_drill(Family::Halves, 3, NOON).expect("a drill");
    // Compared before in an earlier drill: a session would not show the How by itself.
    let earlier = compared(SessionRecord {
        id: 9,
        ..record.clone()
    });
    let mut session = practising(record, vec![earlier]);
    let shown = |session: &Session| session.projection().practice().cloned().expect("a sheet");
    let first = shown(&session);
    assert_eq!(first.instructions, Some(Family::Halves), "its How first");
    assert_eq!((first.drill, first.place), (Some(Family::Halves), (1, 3)));
    assert!(!first.warming_up);
    assert_eq!(session.practice_drill(), Some(Family::Halves));
    session
        .practise(PracticeIntent::Instructions(false), NOON + 10)
        .expect("closed");
    session
        .practise(PracticeIntent::Next(VIEWPORT), NOON + 20)
        .expect("next");
    assert_eq!(shown(&session).instructions, None, "once a drill");
    assert_eq!(shown(&session).place, (2, 3));
    for _ in 0..2 {
        session
            .practise(PracticeIntent::Next(VIEWPORT), NOON + 30)
            .expect("next");
    }
    let done = shown(&session);
    assert!(done.current.is_none() && done.summary.is_some());
    let finished = done.finished.expect("a finish");
    assert_eq!(finished.road, None, "no session's dot");
    assert!(session.practise(PracticeIntent::OneMore, NOON + 40).is_ok());
    // The same drill opened again shows its How first again.
    let (again, _) = log.begin_drill(Family::Halves, 3, NOON).expect("a drill");
    let reopened = practising(again, Vec::new());
    assert_eq!(shown(&reopened).instructions, Some(Family::Halves));
}
