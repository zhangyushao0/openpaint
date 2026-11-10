//! The session's mode slot (D112) with a mode of the test's own: the session asks it where a pen
//! may land, what strokes stay inside, which style the next stroke takes and which layers it
//! keeps, and lends it the artwork and the view; the composition root reaches it by its type; a
//! fill on it is found within its clip's box. And a mode's picker taking its turn with the
//! gallery's dialogs.

use std::sync::Arc;

use openpaint_app::{
    BrushLibrary, BytesSource, CanvasView, Contact, ContactPhase, Device, FileJob, FillIntent,
    Gallery, GalleryIntent, HistoryIntent, HostAnswer, HostRequest, InputSample, Intent,
    LayerIntent, Mode, PortError, Session, StrokeIntent, Toolbox,
};
use openpaint_core::{Artwork, CanvasSize, Color, RegionMask, StrokeStyle};

const FIT: CanvasView = CanvasView::FIT;

/// A mode holding a frame on the left half: pens land only there, strokes stay inside it, its
/// own pen draws at 12 pixels, and it keeps a second layer.
#[derive(Debug)]
struct Frame {
    clip: RegionMask,
    pen: StrokeStyle,
    landed: u32,
    laid: u32,
}

#[derive(Debug)]
struct Other;

impl Mode for Other {
    fn raw(&self) -> bool {
        false
    }
    fn lands(&mut self, _: [f32; 2]) -> bool {
        true
    }
    fn clip(&self) -> Option<&RegionMask> {
        None
    }
    fn style(&self, _: &StrokeStyle) -> Option<StrokeStyle> {
        None
    }
    fn amended(&self, _: &Artwork) -> Option<Artwork> {
        None
    }
    fn laid(&mut self) {}
}

impl Mode for Frame {
    fn raw(&self) -> bool {
        true
    }
    fn lands(&mut self, at: [f32; 2]) -> bool {
        self.landed += 1;
        at[0] < 100.0
    }
    fn clip(&self) -> Option<&RegionMask> {
        Some(&self.clip)
    }
    fn style(&self, _: &StrokeStyle) -> Option<StrokeStyle> {
        Some(self.pen.clone())
    }
    fn amended(&self, current: &Artwork) -> Option<Artwork> {
        let mut session = Session::open(current.clone(), self.pen.clone());
        let missing = current.layers().len() < 2;
        let add = Intent::Layer(LayerIntent::Add);
        (missing && session.dispatch(add).is_ok()).then(|| session.artwork().clone())
    }
    fn laid(&mut self) {
        self.laid += 1;
    }
}

fn brush() -> StrokeStyle {
    Toolbox::new(BrushLibrary::built_in().expect("brushes"))
        .style()
        .expect("style")
}

fn framed() -> Session {
    let canvas = CanvasSize::new(200, 100).expect("size");
    let left = [[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]];
    let brush = brush();
    let pen = StrokeStyle::new(Arc::clone(brush.recipe()), 12.0, 1.0, Color::BLACK).expect("pen");
    let frame = Frame {
        clip: RegionMask::polygon(canvas, &left).expect("frame"),
        pen,
        landed: 0,
        laid: 0,
    };
    let view = CanvasView::FIT;
    Session::with_mode(Artwork::new(canvas), brush, Box::new(frame), view)
}

fn stroke(session: &mut Session, x: f32) {
    let sample = |dx: f32, time_ms| InputSample {
        position: [x + dx, 50.0],
        pressure: 0.7,
        tilt: 0.0,
        azimuth: 0.0,
        time_ms,
    };
    for (phase, sample) in [
        (ContactPhase::Down, sample(0.0, 0)),
        (ContactPhase::Move, sample(20.0, 8)),
        (ContactPhase::Up, sample(40.0, 16)),
    ] {
        let contact = Contact {
            id: 1,
            device: Device::Pen,
            phase,
            samples: vec![sample],
        };
        let stroke = Intent::Stroke(StrokeIntent::Contact(contact));
        session.dispatch(stroke).expect("dispatch");
    }
}

fn strokes(session: &Session) -> usize {
    (session.artwork().layers().iter())
        .map(|layer| layer.strokes().len())
        .sum()
}

#[test]
fn a_mode_says_where_the_pen_lands_draws_with_its_own_pen_and_keeps_its_layers() {
    let mut session = framed();
    assert!(session.mode::<Frame>().is_some() && session.mode::<Other>().is_none());
    assert_eq!(
        session.artwork().layers().len(),
        2,
        "its layer kept as it opened"
    );
    assert_eq!(session.projection().pen(), Some(12.0), "its pen's outline");
    assert!(!session.history().can_undo(), "the layer is no step");
    stroke(&mut session, 150.0);
    assert_eq!(strokes(&session), 0, "a landing outside is refused");
    stroke(&mut session, 20.0);
    assert_eq!(strokes(&session), 1);
    let drawn = session
        .artwork()
        .layers()
        .iter()
        .find_map(|l| l.strokes().last());
    let drawn = drawn.expect("the stroke");
    assert_eq!(drawn.style().size(), 12.0, "the mode's pen, not the brush");
    assert!(drawn.clip().is_some(), "kept inside the frame");
    let frame = session.mode::<Frame>().expect("the mode");
    assert_eq!((frame.landed, frame.laid), (2, 0));
}

#[test]
fn a_fill_on_a_framed_sheet_is_found_within_the_frame_s_box_and_kept_inside_the_frame() {
    let mut session = framed();
    let drop = |point| {
        Intent::Fill(FillIntent::Request {
            point,
            threshold: 0.5,
            color: Color::BLACK,
        })
    };
    session.dispatch(drop([10.0, 20.0])).expect("inside");
    let job = session.fill_request().expect("a job");
    // The worker floods only this box, so a line across the frame keeps the flood on its side.
    assert_eq!(job.within, Some(([0, 0], [100, 100])));
    let canvas = session.artwork().canvas();
    session.fill_finished(job.sequence, Ok(RegionMask::whole(canvas)));
    session
        .dispatch(Intent::Fill(FillIntent::Commit))
        .expect("commit");
    let layer = (session.artwork().layers().iter()).find(|layer| !layer.edits().is_empty());
    let edit = &layer.expect("the filled layer").edits()[0];
    let fill = edit.fill().expect("a fill").mask();
    assert_eq!(
        (fill.origin(), fill.size()),
        ([0, 0], [100, 100]),
        "cut to it"
    );
    let mut plain = Session::with_mode(Artwork::new(canvas), brush(), Box::new(Other), FIT);
    plain.dispatch(drop([150.0, 20.0])).expect("no clip");
    let job = plain.fill_request().expect("a job");
    assert_eq!(job.within, None, "a mode without a clip floods the canvas");
}

#[test]
fn a_mode_lent_the_session_records_a_version_moves_the_view_and_cuts_the_history() {
    let mut session = framed();
    stroke(&mut session, 20.0);
    let lent = session.lend::<Frame, _>(|frame, lent| {
        frame.laid += 1;
        assert!(!lent.drawing() && lent.selection().is_none());
        let artwork = lent.artwork().clone();
        lent.record(artwork);
        *lent.view() = CanvasView::FIT;
        lent.artwork().layers().len()
    });
    assert_eq!(lent, Some(2));
    assert!(
        session.lend::<Other, _>(|_, _| ()).is_none(),
        "not that mode"
    );
    assert!(session.history().can_undo());
    session.lend::<Frame, _>(|_, lent| lent.keep_only_current());
    assert!(
        !session.history().can_undo(),
        "nothing before the version kept"
    );
    let undo = session.dispatch(Intent::History(HistoryIntent::Undo));
    assert!(undo.is_err());
    assert_eq!(session.mode_mut::<Frame>().map(|frame| frame.laid), Some(1));
}

#[test]
fn a_mode_s_picker_takes_its_turn_with_the_gallery_s_dialogs() {
    let mut gallery = Gallery::listed(Vec::new());
    gallery.choose(HostRequest::OpenFile, 7).expect("a picker");
    assert_eq!(gallery.host_request(), Some(HostRequest::OpenFile));
    assert!(
        gallery.dispatch(GalleryIntent::OpenFile).is_err(),
        "one at a time"
    );
    assert!(gallery.choose(HostRequest::OpenImage, 2).is_err());
    let answer = HostAnswer::Image(Box::new(BytesSource(vec![1, 2])));
    let job = gallery.host_answer(answer);
    assert!(matches!(
        job,
        Some(FileJob::Mode {
            tag: 7,
            source: Some(_)
        })
    ));
    assert!(
        gallery.choose(HostRequest::OpenImage, 2).is_err(),
        "while it is read"
    );
    gallery.file_done();
    gallery.choose(HostRequest::OpenImage, 2).expect("the next");
    let failed = gallery.host_answer(HostAnswer::Failed(PortError::Stopped));
    assert!(matches!(
        failed,
        Some(FileJob::Mode {
            tag: 2,
            source: None
        })
    ));
    gallery
        .dispatch(GalleryIntent::OpenFile)
        .expect("free again");
    gallery.opening();
    assert!(
        !gallery.availability().is_available(),
        "held while a mode opens"
    );
}
