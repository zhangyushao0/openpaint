//! The Drawing Guide's symmetry through the session (S42): the copies show while the stroke is
//! drawn and are committed with it as separate stroke records in one undo step.

use openpaint_app::{
    BrushLibrary, Contact, ContactPhase, Device, GuideIntent, HistoryIntent, InputSample, Intent,
    Session, StrokeIntent, Symmetry, Toolbox,
};
use openpaint_core::{Artwork, CanvasSize};

fn session() -> Session {
    let library = BrushLibrary::built_in().expect("built-in brushes");
    let brush = Toolbox::new(library).style().expect("style");
    Session::new(CanvasSize::new(400, 300).expect("size"), brush)
}

fn guide(session: &mut Session, intent: GuideIntent) {
    session
        .dispatch(Intent::Stroke(StrokeIntent::Guide(intent)))
        .expect("guide");
}

fn pen(session: &mut Session, phase: ContactPhase, [x, y]: [f32; 2], time_ms: i64) {
    let sample = InputSample {
        position: [x, y],
        pressure: 0.5,
        tilt: 0.0,
        azimuth: 0.0,
        time_ms,
    };
    let contact = Contact {
        id: 7,
        device: Device::Pen,
        phase,
        samples: vec![sample],
    };
    session
        .dispatch(Intent::Stroke(StrokeIntent::Contact(contact)))
        .expect("contact");
}

fn stroke(session: &mut Session) {
    pen(session, ContactPhase::Down, [220.0, 40.0], 0);
    pen(session, ContactPhase::Move, [260.0, 70.0], 8);
    pen(session, ContactPhase::Up, [300.0, 90.0], 16);
}

/// Every stroke's sample positions on the active layer, oldest stroke first.
fn paths(artwork: &Artwork) -> Vec<Vec<[f32; 2]>> {
    let layer = artwork.layer(artwork.active()).expect("layer");
    layer
        .strokes()
        .iter()
        .map(|stroke| {
            stroke
                .samples()
                .iter()
                .map(|s| [s.position().x(), s.position().y()])
                .collect()
        })
        .collect()
}

#[test]
fn a_vertical_guide_commits_the_stroke_and_its_mirror_as_one_step() {
    let mut session = session();
    let blank = session.artwork().clone();
    guide(&mut session, GuideIntent::Symmetry(Symmetry::Vertical));
    assert!(session.projection().guide().on);
    pen(&mut session, ContactPhase::Down, [220.0, 40.0], 0);
    pen(&mut session, ContactPhase::Move, [260.0, 70.0], 8);
    // While drawing, the canvas shows the copy under the stroke, which is not in history.
    assert_eq!(paths(session.shown()), [vec![[180.0, 40.0], [140.0, 70.0]]]);
    assert!(session.stroke_reshown());
    assert!(session.artwork().same_version(&blank));
    pen(&mut session, ContactPhase::Up, [300.0, 90.0], 16);
    let drawn = vec![[220.0, 40.0], [260.0, 70.0], [300.0, 90.0]];
    let mirror = vec![[180.0, 40.0], [140.0, 70.0], [100.0, 90.0]];
    assert_eq!(
        paths(session.artwork()),
        [mirror, drawn],
        "the copy, then the stroke"
    );
    assert!(session.shown().same_version(session.artwork()));
    assert_eq!(session.history().undo_steps(), 1);
    session
        .dispatch(Intent::History(HistoryIntent::Undo))
        .expect("undo");
    assert!(
        session.artwork().same_version(&blank),
        "one undo takes both"
    );
}

#[test]
fn quadrant_and_radial_make_four_and_eight_strokes_about_a_moved_centre() {
    for (symmetry, count) in [
        (Symmetry::Horizontal, 2),
        (Symmetry::Quadrant, 4),
        (Symmetry::Radial, 8),
    ] {
        let mut session = session();
        guide(&mut session, GuideIntent::Symmetry(symmetry));
        guide(&mut session, GuideIntent::Centre([250.0, 100.0]));
        stroke(&mut session);
        let paths = paths(session.artwork());
        assert_eq!(paths.len(), count, "{symmetry:?}");
        // Every copy keeps the stroke's distances from the centre.
        let from = |p: [f32; 2]| (p[0] - 250.0).hypot(p[1] - 100.0);
        for path in &paths {
            for (copy, drawn) in path.iter().zip(&paths[count - 1]) {
                assert!((from(*copy) - from(*drawn)).abs() < 1e-3, "{symmetry:?}");
            }
        }
    }
}

#[test]
fn the_guide_off_draws_one_stroke_and_a_cancel_leaves_nothing() {
    let mut session = session();
    guide(&mut session, GuideIntent::Symmetry(Symmetry::Quadrant));
    guide(&mut session, GuideIntent::Show(false));
    stroke(&mut session);
    assert_eq!(paths(session.artwork()).len(), 1);
    guide(&mut session, GuideIntent::Show(true));
    let before = session.artwork().clone();
    pen(&mut session, ContactPhase::Down, [20.0, 20.0], 100);
    pen(&mut session, ContactPhase::Move, [60.0, 20.0], 108);
    assert_eq!(
        paths(session.shown()).len(),
        4,
        "the stroke before and three copies"
    );
    pen(&mut session, ContactPhase::Cancel, [60.0, 20.0], 116);
    assert!(session.shown().same_version(&before));
    assert!(session.artwork().same_version(&before));
}
