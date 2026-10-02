//! Autosave through the session: which version is handed to the store and when, how a failed
//! save shows and is retried, and the notice after a failed open.

use openpaint_app::{
    AppError, Availability, Contact, ContactPhase, Device, EditorAction, HistoryIntent,
    InputSample, Intent, PortError, Session, StorageIntent, StorageNotice, StrokeIntent,
};
use openpaint_core::CanvasSize;

fn canvas() -> CanvasSize {
    CanvasSize::new(200, 100).expect("size")
}

fn pen(phase: ContactPhase, x: f32) -> Intent {
    let sample = InputSample {
        position: [x, 50.0],
        pressure: 0.5,
        tilt: 0.0,
        azimuth: 0.0,
        time_ms: 0,
    };
    Intent::Stroke(StrokeIntent::Contact(Contact {
        id: 1,
        device: Device::Pen,
        phase,
        samples: vec![sample],
    }))
}

fn draw(session: &mut Session, x: f32) {
    for (phase, dx) in [(ContactPhase::Down, 0.0), (ContactPhase::Up, 10.0)] {
        session.dispatch(pen(phase, x + dx)).expect("stroke");
    }
}

fn failure() -> Result<(), PortError> {
    Err(PortError::Device {
        detail: "disk full".into(),
    })
}

fn retry(session: &mut Session) -> Result<(), AppError> {
    session.dispatch(Intent::Storage(StorageIntent::RetrySave))
}

#[test]
fn each_finished_stroke_is_handed_to_the_store_once() {
    let mut session = Session::new(canvas()).expect("session");
    assert!(
        session.save_request().is_none(),
        "a blank artwork has nothing to lose"
    );
    session
        .dispatch(pen(ContactPhase::Down, 10.0))
        .expect("down");
    assert!(
        session.save_request().is_none(),
        "the stroke is not finished"
    );
    session.dispatch(pen(ContactPhase::Up, 20.0)).expect("up");
    let first = session.save_request().expect("the finished stroke");
    assert!(first.same_version(session.artwork()));
    assert!(session.saving());
    draw(&mut session, 40.0);
    assert!(session.save_request().is_none(), "one save at a time");
    session.save_finished(&first, &Ok(()));
    let second = session.save_request().expect("the newer version follows");
    assert!(second.same_version(session.artwork()));
    session.save_finished(&second, &Ok(()));
    assert!(session.save_request().is_none(), "everything is stored");
    assert_eq!(session.projection().storage(), StorageNotice::None);
}

#[test]
fn undo_back_to_the_stored_version_needs_no_save() {
    let mut session = Session::new(canvas()).expect("session");
    draw(&mut session, 10.0);
    let stored = session.save_request().expect("stroke");
    session.save_finished(&stored, &Ok(()));
    draw(&mut session, 40.0);
    session
        .dispatch(Intent::History(HistoryIntent::Undo))
        .expect("undo");
    assert!(session.save_request().is_none());
}

#[test]
fn a_failed_save_shows_waits_and_is_retried_on_request() {
    let mut session = Session::new(canvas()).expect("session");
    let projection = session.projection();
    assert_eq!(
        projection.availability(EditorAction::RetrySave),
        Availability::NothingLeft
    );
    assert!(retry(&mut session).is_err(), "nothing to retry");
    draw(&mut session, 10.0);
    let version = session.save_request().expect("stroke");
    session.save_finished(&version, &failure());
    let projection = session.projection();
    assert_eq!(
        projection.storage(),
        StorageNotice::SaveFailed { retrying: false }
    );
    assert_eq!(
        projection.availability(EditorAction::RetrySave),
        Availability::Available
    );
    assert!(
        session.save_request().is_none(),
        "a failed version is not retried by itself every frame"
    );
    retry(&mut session).expect("retry");
    let again = session.save_request().expect("the retry");
    assert!(again.same_version(&version));
    let projection = session.projection();
    assert_eq!(
        projection.storage(),
        StorageNotice::SaveFailed { retrying: true }
    );
    assert_eq!(
        projection.availability(EditorAction::RetrySave),
        Availability::Running
    );
    session.save_finished(&again, &Ok(()));
    assert_eq!(session.projection().storage(), StorageNotice::None);
}

#[test]
fn the_next_stroke_after_a_failure_tries_again() {
    let mut session = Session::new(canvas()).expect("session");
    draw(&mut session, 10.0);
    let version = session.save_request().expect("stroke");
    session.save_finished(&version, &failure());
    draw(&mut session, 40.0);
    let next = session.save_request().expect("the new version");
    assert!(next.same_version(session.artwork()));
    session.save_finished(&next, &Ok(()));
    assert_eq!(session.projection().storage(), StorageNotice::None);
}

#[test]
fn leaving_tries_a_failed_version_once_more() {
    let mut session = Session::new(canvas()).expect("session");
    draw(&mut session, 10.0);
    let version = session.save_request().expect("stroke");
    session.save_finished(&version, &failure());
    let leaving = session.leave_request().expect("the failed version again");
    assert!(leaving.same_version(&version));
    assert!(session.leave_request().is_none(), "one save at a time");
}

#[test]
fn a_late_result_for_another_version_changes_nothing() {
    let mut session = Session::new(canvas()).expect("session");
    draw(&mut session, 10.0);
    let stale = session.artwork().clone();
    session.save_finished(&stale, &failure());
    assert_eq!(session.projection().storage(), StorageNotice::None);
}

#[test]
fn a_failed_open_is_told_until_dismissed() {
    let mut session = Session::after_failed_open(canvas()).expect("session");
    assert_eq!(session.projection().storage(), StorageNotice::OpenFailed);
    assert!(session.artwork().layers()[0].strokes().is_empty());
    session
        .dispatch(Intent::Storage(StorageIntent::Dismiss))
        .expect("dismiss");
    assert_eq!(session.projection().storage(), StorageNotice::None);
    let again = session.dispatch(Intent::Storage(StorageIntent::Dismiss));
    assert!(matches!(again, Err(AppError::Unavailable { .. })));
}

#[test]
fn an_opened_artwork_counts_as_stored() {
    let mut drawn = Session::new(canvas()).expect("session");
    draw(&mut drawn, 10.0);
    let mut session = Session::open(drawn.artwork().clone()).expect("open");
    assert!(session.save_request().is_none());
    assert!(!session.history().can_undo(), "history is not stored");
}
