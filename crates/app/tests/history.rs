//! History: undo and redo through the session, availability in the projection, and the byte
//! budget that trims the oldest steps.

use openpaint_app::{
    AppError, Availability, BrushLibrary, Contact, ContactPhase, Device, EditorAction, History,
    HistoryIntent, InputSample, Intent, Session, StrokeIntent, Toolbox,
};
use openpaint_core::{Artwork, CanvasPoint, CanvasSize, StrokeSample, StrokeSession, StrokeStyle};

fn brush() -> StrokeStyle {
    let library = BrushLibrary::built_in().expect("built-in brushes");
    Toolbox::new(library).style().expect("style")
}

fn session() -> Session {
    Session::new(CanvasSize::new(200, 100).expect("size"), brush())
}

fn pen(phase: ContactPhase, x: f32, time_ms: i64) -> Intent {
    let sample = InputSample {
        position: [x, 50.0],
        pressure: 0.5,
        tilt: 0.0,
        azimuth: 0.0,
        time_ms,
    };
    Intent::Stroke(StrokeIntent::Contact(Contact {
        id: 1,
        device: Device::Pen,
        phase,
        samples: vec![sample],
    }))
}

fn draw(session: &mut Session, x: f32) {
    for (phase, dx) in [
        (ContactPhase::Down, 0.0),
        (ContactPhase::Move, 5.0),
        (ContactPhase::Up, 10.0),
    ] {
        session.dispatch(pen(phase, x + dx, 0)).expect("stroke");
    }
}

fn step(session: &mut Session, intent: HistoryIntent) -> Result<(), AppError> {
    session.dispatch(Intent::History(intent))
}

fn availability(session: &Session) -> (Availability, Availability) {
    let projection = session.projection();
    (
        projection.availability(EditorAction::Undo),
        projection.availability(EditorAction::Redo),
    )
}

#[test]
fn undo_removes_the_stroke_and_redo_brings_it_back() {
    let mut session = session();
    let blank = session.artwork().clone();
    use Availability::{Available, NothingLeft};
    assert_eq!(availability(&session), (NothingLeft, NothingLeft));
    draw(&mut session, 10.0);
    let drawn = session.artwork().clone();
    assert_eq!(availability(&session), (Available, NothingLeft));
    step(&mut session, HistoryIntent::Undo).expect("undo");
    assert!(session.artwork().same_version(&blank));
    assert_eq!(availability(&session), (NothingLeft, Available));
    step(&mut session, HistoryIntent::Redo).expect("redo");
    assert!(session.artwork().same_version(&drawn));
    assert_eq!(availability(&session), (Available, NothingLeft));
}

#[test]
fn twenty_round_trips_leave_the_artwork_equal() {
    let mut session = session();
    for x in [10.0, 60.0, 110.0] {
        draw(&mut session, x);
    }
    let drawn = session.artwork().clone();
    for _ in 0..20 {
        for _ in 0..3 {
            step(&mut session, HistoryIntent::Undo).expect("undo");
        }
        for _ in 0..3 {
            step(&mut session, HistoryIntent::Redo).expect("redo");
        }
    }
    assert!(session.artwork().same_version(&drawn));
    assert_eq!(session.artwork(), &drawn);
    assert_eq!(session.history().undo_steps(), 3);
    assert_eq!(session.history().redo_steps(), 0);
}

#[test]
fn nothing_left_is_refused_and_changes_nothing() {
    let mut session = session();
    let blank = session.artwork().clone();
    let refused = step(&mut session, HistoryIntent::Undo).expect_err("nothing to undo");
    assert_eq!(
        refused,
        AppError::Unavailable {
            action: EditorAction::Undo,
            availability: Availability::NothingLeft
        }
    );
    assert!(step(&mut session, HistoryIntent::Redo).is_err());
    assert!(session.artwork().same_version(&blank));
}

#[test]
fn history_waits_while_a_stroke_is_drawn() {
    let mut session = session();
    draw(&mut session, 10.0);
    let first = session.artwork().clone();
    session
        .dispatch(pen(ContactPhase::Down, 50.0, 100))
        .expect("down");
    // Undo keeps its look (there is a step) but waits; redo has nothing either way.
    let drawing = Availability::StrokeInProgress;
    assert_eq!(availability(&session), (drawing, Availability::NothingLeft));
    assert!(drawing.is_offered() && !Availability::NothingLeft.is_offered());
    let refused = step(&mut session, HistoryIntent::Undo).expect_err("busy");
    assert!(refused.to_string().contains("stroke is in progress"));
    assert!(session.active().is_some(), "the stroke goes on");
    session
        .dispatch(pen(ContactPhase::Up, 60.0, 104))
        .expect("up");
    assert_eq!(session.history().undo_steps(), 2);
    step(&mut session, HistoryIntent::Undo).expect("undo");
    assert!(session.artwork().same_version(&first));
}

#[test]
fn a_new_stroke_after_undo_drops_what_could_be_redone() {
    let mut session = session();
    draw(&mut session, 10.0);
    draw(&mut session, 60.0);
    step(&mut session, HistoryIntent::Undo).expect("undo");
    draw(&mut session, 110.0);
    assert_eq!(availability(&session).1, Availability::NothingLeft);
    assert_eq!(session.history().undo_steps(), 2);
}

#[test]
fn finger_contacts_and_cancelled_strokes_record_nothing() {
    let mut session = session();
    session
        .dispatch(pen(ContactPhase::Down, 10.0, 0))
        .expect("down");
    session
        .dispatch(pen(ContactPhase::Cancel, 10.0, 4))
        .expect("cancel");
    assert_eq!(session.history().undo_steps(), 0);
}

// Budget trimming, on `History` directly.

fn sample(x: f32, time_ms: u32) -> StrokeSample {
    StrokeSample::new(
        CanvasPoint::new(x, 10.0).expect("point"),
        0.5,
        0.0,
        0.0,
        time_ms,
    )
    .expect("sample")
}

/// The artwork with one more stroke of `samples` samples.
fn stroke(artwork: &Artwork, samples: u32) -> Artwork {
    let mut session: StrokeSession = artwork
        .begin_stroke(brush(), sample(0.0, 0))
        .expect("begin");
    let rest: Vec<StrokeSample> = (1..samples).map(|i| sample(i as f32, i)).collect();
    session.extend(&rest).expect("extend");
    session.finish().artwork
}

const STEP_SAMPLES: u32 = 1000;

fn step_bytes() -> usize {
    let blank = Artwork::new(CanvasSize::new(64, 64).expect("size"));
    stroke(&blank, STEP_SAMPLES).bytes_since(&blank)
}

/// A history of `budget` bytes after `steps` strokes, and the versions it went through.
fn recorded(budget: usize, steps: usize) -> (History, Vec<Artwork>) {
    let mut versions = vec![Artwork::new(CanvasSize::new(64, 64).expect("size"))];
    let mut history = History::new(versions[0].clone(), budget);
    for _ in 0..steps {
        let next = stroke(versions.last().expect("version"), STEP_SAMPLES);
        assert!(history.record(next.clone()));
        versions.push(next);
    }
    (history, versions)
}

#[test]
fn the_budget_trims_the_oldest_steps_first() {
    let per_step = step_bytes();
    let (mut history, versions) = recorded(per_step * 5 + per_step / 2, 12);
    assert_eq!(history.undo_steps(), 5);
    assert!(history.total_bytes() <= per_step * 5 + per_step / 2);
    while history.undo().is_some() {}
    assert!(
        history.current().same_version(&versions[7]),
        "the oldest kept version is the one five steps back"
    );
}

#[test]
fn within_the_budget_nothing_is_trimmed() {
    let (history, _) = recorded(step_bytes() * 100, 12);
    assert_eq!(history.undo_steps(), 12);
    assert!(history.total_bytes() >= step_bytes() * 12);
}

#[test]
fn the_newest_step_stays_even_above_the_budget() {
    let (mut history, versions) = recorded(10, 4);
    assert_eq!(history.undo_steps(), 1);
    assert!(history.undo().expect("undo").same_version(&versions[3]));
    assert!(history.undo().is_none());
}

#[test]
fn recording_after_undo_frees_the_redo_steps() {
    let per_step = step_bytes();
    let (mut history, versions) = recorded(per_step * 100, 4);
    let full = history.total_bytes();
    history.undo();
    history.undo();
    assert_eq!(history.total_bytes(), full, "undo keeps what can be redone");
    let branch = stroke(&versions[2], 10);
    assert!(history.record(branch.clone()));
    assert!(
        !history.record(branch),
        "the current version records nothing"
    );
    assert!(!history.can_redo());
    assert_eq!(history.undo_steps(), 3);
    assert!(history.total_bytes() < full - per_step);
}
