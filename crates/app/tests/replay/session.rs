//! The session's part in the replay: strokes stamped on the UTC clock the runtime sets (their
//! mirrored copies alike), and Actions' offer (nothing to replay without a record, an edit
//! replays, refused while a stroke is drawn, none on a mode's sheet).

use openpaint_app::{
    Availability, BrushLibrary, Contact, ContactPhase, Device, EditorAction, GuideIntent,
    InputSample, Intent, ReplayIntent, ReplayShown, Session, StrokeIntent, Symmetry, Toolbox,
};
use openpaint_core::StrokeStyle;

use super::{blank, fill};

fn pen(session: &mut Session, phase: ContactPhase, x: f32, time_ms: i64) {
    let samples = vec![InputSample {
        position: [x, 20.0],
        pressure: 0.7,
        tilt: 0.0,
        azimuth: 0.0,
        time_ms,
    }];
    let contact = Contact {
        id: 1,
        device: Device::Pen,
        phase,
        samples,
    };
    let intent = Intent::Stroke(StrokeIntent::Contact(contact));
    session.dispatch(intent).expect("pen");
}

fn brush() -> StrokeStyle {
    let toolbox = Toolbox::new(BrushLibrary::built_in().expect("brushes"));
    toolbox.style().expect("style")
}

fn offered(session: &Session) -> Availability {
    session.projection().availability(EditorAction::Replay)
}

#[test]
fn the_session_stamps_strokes_on_the_utc_clock_and_offers_the_replay() {
    let mut session = Session::open(blank(), brush());
    assert_eq!(offered(&session), Availability::NothingLeft, "no record");
    let open = Intent::Replay(ReplayIntent::Open);
    assert!(session.dispatch(open.clone()).is_err());
    pen(&mut session, ContactPhase::Down, 10.0, 5_000);
    pen(&mut session, ContactPhase::Up, 20.0, 5_016);
    let first = session.artwork().layers()[0].strokes().last();
    assert_eq!(first.expect("drawn").began_ms(), None, "no clock yet");
    session.set_utc_clock(1_000_000);
    let mirrored = GuideIntent::Symmetry(Symmetry::Vertical);
    let guide = Intent::Stroke(StrokeIntent::Guide(mirrored));
    session.dispatch(guide).expect("guide");
    pen(&mut session, ContactPhase::Down, 10.0, 6_000);
    assert_eq!(offered(&session), Availability::StrokeInProgress);
    pen(&mut session, ContactPhase::Up, 20.0, 6_016);
    let strokes = session.artwork().layers()[0].strokes();
    let starts: Vec<_> = strokes.iter().map(|stroke| stroke.began_ms()).collect();
    let stamped = Some(1_006_000);
    assert_eq!(starts, [None, stamped, stamped], "its copy alike");
    assert_eq!(offered(&session), Availability::Available);
    let shown = session.projection().replay_shown().clone();
    let ReplayShown::Offered { strokes, .. } = shown else {
        panic!("offered");
    };
    assert_eq!(strokes, 3);
    session.dispatch(open).expect("opens");
    let sheet = session.projection().with_replay(ReplayShown::None);
    let sheet = sheet.availability(EditorAction::Replay);
    assert_eq!(
        sheet,
        Availability::NothingLeft,
        "a mode's sheet offers none"
    );
    let filled = Session::open(fill(&blank()), brush());
    assert_eq!(offered(&filled), Availability::Available, "an edit replays");
}
