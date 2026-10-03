//! Fill intents: a request becomes a preview (never a history step), only the newest request
//! runs, a commit is one step that undo takes back, a cancel or another edit leaves nothing,
//! and a hidden or locked layer refuses with a notice.

use openpaint_app::{
    AppError, Availability, Contact, ContactPhase, DEFAULT_THRESHOLD, Device, EditorAction,
    FillFailed, FillIntent, FillRefusal, HistoryIntent, InputSample, Intent, LayerIntent, Session,
    StrokeIntent,
};
use openpaint_app::{BrushLibrary, Toolbox};
use openpaint_core::{
    Artwork, ArtworkCommand, CanvasSize, Color, LayerCommand, RegionMask, StrokeStyle,
};

const SIZE: [u32; 2] = [40, 20];

fn brush() -> StrokeStyle {
    let library = BrushLibrary::built_in().expect("built-in brushes");
    Toolbox::new(library).style().expect("style")
}

fn session() -> Session {
    Session::new(CanvasSize::new(SIZE[0], SIZE[1]).expect("size"), brush())
}

fn red() -> Color {
    Color::rgb(1.0, 0.0, 0.0).expect("red")
}

fn request(session: &mut Session, x: f32, threshold: f32) -> Result<(), AppError> {
    session.dispatch(Intent::Fill(FillIntent::Request {
        point: [x, 5.0],
        threshold,
        color: red(),
    }))
}

fn fill(session: &mut Session, intent: FillIntent) {
    session.dispatch(Intent::Fill(intent)).expect("fill intent");
}

/// The pixels left of `x`: what the worker would find for a request there.
fn region(x: usize) -> RegionMask {
    let mut inside = vec![false; (SIZE[0] * SIZE[1]) as usize];
    for row in inside.chunks_mut(SIZE[0] as usize) {
        row[..x].fill(true);
    }
    RegionMask::from_pixels(CanvasSize::new(SIZE[0], SIZE[1]).expect("size"), &inside)
        .expect("mask")
}

/// Runs the job the session hands out, if any, with `mask`; returns its sequence.
fn run(session: &mut Session, mask: RegionMask) -> Option<u64> {
    let job = session.fill_request()?;
    session.fill_finished(job.sequence, Ok(mask));
    Some(job.sequence)
}

fn fills(artwork: &Artwork) -> usize {
    artwork
        .layers()
        .iter()
        .map(|layer| layer.edits().len())
        .sum()
}

#[test]
fn a_preview_is_shown_but_not_kept_and_a_commit_is_one_undoable_step() {
    let mut session = session();
    assert_eq!(session.projection().fill().threshold, DEFAULT_THRESHOLD);
    request(&mut session, 3.0, 0.25).expect("request");
    let job = session.fill_request().expect("a job");
    assert_eq!((job.point, job.threshold), ([3, 5], 0.25));
    assert!(session.projection().fill().pending);
    assert!(session.fill_request().is_none(), "handed out once");
    session.fill_finished(job.sequence, Ok(region(10)));
    assert_eq!(fills(session.shown()), 1, "the preview shows");
    assert_eq!(fills(session.artwork()), 0, "but is not the artwork");
    assert_eq!(
        session.projection().availability(EditorAction::Undo),
        Availability::NothingLeft
    );
    fill(&mut session, FillIntent::Commit);
    assert_eq!(fills(session.artwork()), 1);
    assert!(session.shown().same_version(session.artwork()));
    assert!(!session.filling());
    session
        .dispatch(Intent::History(HistoryIntent::Undo))
        .expect("undo");
    assert_eq!(fills(session.artwork()), 0);
    session
        .dispatch(Intent::History(HistoryIntent::Redo))
        .expect("redo");
    assert_eq!(fills(session.artwork()), 1);
    assert_eq!(
        session.projection().fill().threshold,
        0.25,
        "kept for the next"
    );
}

#[test]
fn only_the_newest_request_runs_and_the_last_preview_stays_meanwhile() {
    let mut session = session();
    request(&mut session, 3.0, 0.2).expect("first");
    let first = session.fill_request().expect("first job");
    request(&mut session, 3.0, 0.3).expect("second");
    request(&mut session, 3.0, 0.4).expect("third");
    assert!(session.fill_request().is_none(), "one at a time");
    session.fill_finished(first.sequence, Ok(region(8)));
    assert_eq!(
        fills(session.shown()),
        1,
        "the first preview shows while the next runs"
    );
    fill(&mut session, FillIntent::Commit);
    assert_eq!(
        fills(session.artwork()),
        0,
        "the newest is not computed yet"
    );
    let newest = session.fill_request().expect("the newest job");
    assert_eq!(newest.threshold, 0.4, "the second never ran");
    assert_eq!(fills(session.shown()), 1, "still the old preview");
    session.fill_finished(newest.sequence, Ok(region(12)));
    let kept = &session.artwork().layers()[0].edits()[0];
    let kept = kept.fill().expect("a fill");
    assert_eq!(kept.mask().size(), [12, 20], "the newest fill was kept");
    assert!(session.fill_request().is_none());
}

#[test]
fn a_cancel_or_another_edit_leaves_the_artwork_as_it_was() {
    let mut session = session();
    let before = session.artwork().clone();
    request(&mut session, 3.0, 0.5).expect("request");
    let job = session.fill_request().expect("job");
    fill(&mut session, FillIntent::Cancel);
    session.fill_finished(job.sequence, Ok(region(10)));
    assert!(session.shown().same_version(&before));
    assert!(session.artwork().same_version(&before));
    request(&mut session, 3.0, 0.5).expect("request");
    run(&mut session, region(10)).expect("ran");
    session
        .dispatch(Intent::Layer(LayerIntent::Add))
        .expect("add");
    assert_eq!(
        fills(session.shown()),
        0,
        "a layer change drops the preview"
    );
    fill(&mut session, FillIntent::Commit);
    assert_eq!(fills(session.artwork()), 0);
}

#[test]
fn a_failed_fill_says_so_and_a_later_request_tries_again() {
    let mut session = session();
    request(&mut session, 3.0, 0.5).expect("request");
    fill(&mut session, FillIntent::Commit);
    let job = session.fill_request().expect("job");
    session.fill_finished(job.sequence, Err(FillFailed));
    assert!(session.projection().fill().failed);
    assert!(!session.filling(), "a commit that failed ends the fill");
    request(&mut session, 3.0, 0.5).expect("again");
    assert!(!session.projection().fill().failed);
}

#[test]
fn a_hidden_or_locked_layer_refuses_with_a_notice() {
    let mut session = session();
    let layer = session.artwork().active();
    session
        .dispatch(Intent::Layer(LayerIntent::SetVisible {
            layer,
            visible: false,
        }))
        .expect("hide");
    let refused = request(&mut session, 3.0, 0.5);
    assert_eq!(refused, Err(AppError::FillRefused(FillRefusal::Hidden)));
    assert_eq!(
        session.projection().fill().refused,
        Some(FillRefusal::Hidden)
    );
    assert!(session.fill_request().is_none());
    session
        .dispatch(Intent::Layer(LayerIntent::SetVisible {
            layer,
            visible: true,
        }))
        .expect("show");
    assert_eq!(
        session.projection().fill().refused,
        None,
        "acting clears it"
    );
    let lock = ArtworkCommand::Layer(LayerCommand::SetLocked {
        layer,
        locked: true,
    });
    let locked = session.artwork().apply(lock).expect("lock").artwork;
    let mut session = Session::open(locked, brush());
    let refused = request(&mut session, 3.0, 0.5);
    assert_eq!(refused, Err(AppError::FillRefused(FillRefusal::Locked)));
}

#[test]
fn no_fill_while_a_stroke_is_drawn_or_off_the_canvas() {
    let mut session = session();
    let sample = InputSample {
        position: [5.0, 5.0],
        pressure: 0.5,
        tilt: 0.0,
        azimuth: 0.0,
        time_ms: 0,
    };
    let down = Contact {
        id: 1,
        device: Device::Pen,
        phase: ContactPhase::Down,
        samples: vec![sample],
    };
    session
        .dispatch(Intent::Stroke(StrokeIntent::Contact(down)))
        .expect("down");
    assert_eq!(
        session.projection().availability(EditorAction::Fill),
        Availability::StrokeInProgress
    );
    assert!(matches!(
        request(&mut session, 3.0, 0.5),
        Err(AppError::Unavailable { .. })
    ));
    let mut session = self::session();
    assert!(
        request(&mut session, 40.0, 0.5).is_err(),
        "past the right edge"
    );
    assert!(request(&mut session, 3.0, 1.5).is_err());
}
