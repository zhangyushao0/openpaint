//! S83's practice polish in the app: undo and redo on a sheet reach only the exercise shown (a
//! Compare's record is never undone from a later exercise); a painted copy's reading that arrives
//! after its exercise was left is dropped; a colour copy whose picture was removed keeps its
//! frames and strokes and offers no Compare; the log keeps a copy's name, forgets it with the
//! copy, and the user's own pictures as their pack now holds them.

mod support;

use std::sync::Arc;

use openpaint_app::{
    BrushLibrary, Contact, ContactPhase, Device, HistoryIntent, InputSample, Intent, ProjectId,
    Session, StrokeIntent, Title, Toolbox, Viewport,
};
use openpaint_core::{Artwork, CanvasSize, Color, StrokeStyle};
use openpaint_practice::{
    Arrangement, Exercise, ItemRef, Kind, LineTier, PERSONAL, Pack, PackItem, PictureKind, Plan,
    Sheet, Tier,
};
use openpaint_practice_app::{
    PracticeIntent, PracticeLog, PracticePens, PracticeProjection, SheetView,
};
use support::OnSheet;

const VIEWPORT: Viewport = Viewport {
    origin: [0.0, 84.0],
    size: [2880.0, 1716.0],
    pixels_per_point: 1.75,
};
const NOW: u64 = 1_791_400_000;

/// The sheet of `record` with `packs` on a canvas of `size`, opened.
fn opened(record: Sheet, packs: &[Pack], size: [u32; 2]) -> Session {
    let sheet = Artwork::new(CanvasSize::new(size[0], size[1]).expect("size"));
    let view = SheetView::of(&record, packs, (size, &Title::untitled(), 1));
    let library = BrushLibrary::built_in().expect("built-in brushes");
    let style = Toolbox::new(library).style().expect("style");
    let brush =
        StrokeStyle::new(Arc::clone(style.recipe()), 8.0, 1.0, Color::BLACK).expect("brush");
    let pens = PracticePens::built_in().expect("pens");
    openpaint_practice_app::open_sheet(
        sheet,
        (brush, pens),
        (
            record,
            Vec::new(),
            view,
            openpaint_practice::Tally::default(),
        ),
        &VIEWPORT,
    )
}

/// A pen stroke through `points` (canvas pixels).
fn draw(session: &mut Session, points: &[[f32; 2]]) {
    let sample = |index: usize| InputSample {
        position: points[index],
        pressure: 0.8,
        tilt: 0.2,
        azimuth: 0.0,
        time_ms: 1000 + 8 * index as i64,
    };
    let last = points.len() - 1;
    let steps = [
        (ContactPhase::Down, vec![sample(0)]),
        (ContactPhase::Move, (1..last).map(sample).collect()),
        (ContactPhase::Up, vec![sample(last)]),
    ];
    for (phase, samples) in steps {
        let contact = Contact {
            id: 7,
            device: Device::Pen,
            phase,
            samples,
        };
        session
            .dispatch(Intent::Stroke(StrokeIntent::Contact(contact)))
            .expect("dispatch");
    }
}

fn strokes(session: &Session) -> usize {
    (session.artwork().layers().iter())
        .map(|layer| layer.strokes().iter().count())
        .sum()
}

/// A line drawn across the shown exercise's copy frame.
fn line_in_frame(session: &mut Session) {
    let practice = PracticeProjection::of(session).expect("a sheet");
    let frame = practice.sheet.pairs[practice.current.expect("shown")].copy;
    let [a, b] = [[0.3, 0.4], [0.7, 0.6]].map(|p| frame.to_canvas(p).map(|v| v as f32));
    draw(session, &[a, [(a[0] + b[0]) / 2.0, a[1] + 3.0], b]);
}

fn lines(count: usize) -> Sheet {
    let plan = Plan {
        exercises: (0..count)
            .map(|k| Exercise::generated(Kind::Line(LineTier::Arc), k as u64))
            .collect(),
        warm_up: 0,
    };
    Sheet::session(1, 1, 0, plan)
}

#[test]
fn undo_and_redo_reach_only_the_exercise_shown() {
    let record = lines(3);
    let size = openpaint_practice::Layout::of_sheet(&record).size;
    let mut session = opened(record, &[], size);
    line_in_frame(&mut session);
    line_in_frame(&mut session);
    session
        .practise(PracticeIntent::Compare(true), NOW)
        .expect("compared");
    session
        .practise(PracticeIntent::Next(VIEWPORT), NOW)
        .expect("next");
    assert_eq!(strokes(&session), 2);
    let undo = |session: &mut Session| session.dispatch(Intent::History(HistoryIntent::Undo));
    assert!(
        undo(&mut session).is_err(),
        "nothing of this exercise to undo"
    );
    line_in_frame(&mut session);
    assert_eq!(strokes(&session), 3);
    undo(&mut session).expect("its own stroke");
    assert_eq!(strokes(&session), 2, "the earlier exercise's strokes stay");
    assert!(
        undo(&mut session).is_err(),
        "the floor: the exercise's start"
    );
    let redo = session.dispatch(Intent::History(HistoryIntent::Redo));
    redo.expect("redo within the exercise");
    assert_eq!(strokes(&session), 3);
    // The first exercise's Compare stands in the record.
    let saved = session.sheet().save_request().expect("the record changed");
    assert_eq!(saved.laid(0).map(|l| l.compares), Some(1));
    // An undone stroke of this exercise is not redone into the next.
    undo(&mut session).expect("undone");
    session
        .practise(PracticeIntent::Next(VIEWPORT), NOW)
        .expect("next");
    let redo = session.dispatch(Intent::History(HistoryIntent::Redo));
    assert!(redo.is_err(), "nothing redone across exercises");
}

#[test]
fn a_reading_that_arrives_after_its_exercise_was_left_is_dropped() {
    let plan = Plan {
        exercises: vec![
            Exercise::generated(Kind::Areas(Arrangement::Patches), 0x5EED_0082),
            Exercise::generated(Kind::Areas(Arrangement::Strata), 0x5EED_0084),
        ],
        warm_up: 0,
    };
    let record = Sheet::session(1, 2, 0, plan);
    let size = openpaint_practice::Layout::of_sheet(&record).size;
    let mut session = opened(record, &[], size);
    let fill = Color::rgb(0.5, 0.4, 0.3).expect("colour");
    session
        .practise(PracticeIntent::Fill(fill), NOW)
        .expect("filled");
    session
        .practise(PracticeIntent::Compare(true), NOW)
        .expect("compared");
    let late = session.sheet().read_request().expect("its copy asked for");
    session
        .practise(PracticeIntent::Next(VIEWPORT), NOW)
        .expect("next");
    session
        .practise(PracticeIntent::Fill(fill), NOW)
        .expect("filled");
    session
        .practise(PracticeIntent::Compare(true), NOW)
        .expect("compared");
    let pixels = openpaint_practice_app::PictureImage {
        size: late.size,
        pixels: [128, 102, 77, 255].repeat((late.size[0] * late.size[1]) as usize),
    };
    session.sheet().read(late.sequence, late.run(&pixels));
    let record = session.sheet().save_request().expect("the record changed");
    let second = record.first(1).expect("the second compared");
    let unread = record.laid(1).and_then(|laid| laid.result);
    assert_eq!(unread, None, "no verdict before its pixels are read");
    assert!(
        second.metrics.is_empty(),
        "the first exercise's reading is not the second's"
    );
    let current = session
        .sheet()
        .read_request()
        .expect("the second's copy asked for");
    session.sheet().read(current.sequence, current.run(&pixels));
    let record = session.sheet().save_request().expect("its numbers kept");
    assert!(!record.first(1).expect("compared").metrics.is_empty());
    let judged = record.laid(1).and_then(|laid| laid.result);
    assert!(
        matches!(judged, Some((_, openpaint_practice::Judge::App))),
        "judged by the app once read: {judged:?}"
    );
}

fn pack(id: &str, item: &str) -> Pack {
    Pack {
        id: id.to_owned(),
        title: "Pictures".to_owned(),
        items: vec![PackItem {
            id: item.to_owned(),
            kind: PictureKind::Shape,
            tier: Tier::Simple,
            size: [800, 600],
            title: "Picture 1".to_owned(),
            credit: None,
            outline: None,
            masses: None,
            areas: None,
        }],
    }
}

#[test]
fn a_copy_whose_picture_was_removed_keeps_its_frames_and_strokes_and_offers_no_compare() {
    let mine = ItemRef {
        pack: PERSONAL.to_owned(),
        item: "mine-1".to_owned(),
    };
    let packs = [pack(PERSONAL, "mine-1")];
    let log = PracticeLog::new(Vec::new(), Vec::new(), packs.to_vec());
    let (mut record, _) = log.begin_copy(&mine, 1, 2).expect("a copy of my picture");
    record.canvas = 9;
    let record = record.framed(&packs);
    let size = openpaint_practice::Layout::of_sheet(&record).size;
    let with = opened(record.clone(), &packs, size);
    let frames = PracticeProjection::of(&with)
        .expect("a sheet")
        .sheet
        .pairs
        .clone();
    // The picture removed: the same frames from the canvas, no picture, no Compare.
    let mut without = opened(record, &[], size);
    let practice = PracticeProjection::of(&without).expect("a sheet");
    assert_eq!(practice.sheet.pairs, frames);
    assert_eq!(practice.sheet.pictures, vec![None]);
    line_in_frame(&mut without);
    assert!(
        !PracticeProjection::of(&without)
            .expect("sheet")
            .compare
            .is_offered()
    );
    assert!(
        without
            .practise(PracticeIntent::Compare(true), NOW)
            .is_err()
    );
    assert_eq!(strokes(&without), 1, "the strokes stay");
}

#[test]
fn the_log_keeps_a_copy_s_name_and_the_user_s_own_pictures_as_their_pack_holds_them() {
    let mine = ItemRef {
        pack: PERSONAL.to_owned(),
        item: "mine-1".to_owned(),
    };
    let official = pack("test-shapes", "lamp");
    let mut log = PracticeLog::new(
        Vec::new(),
        Vec::new(),
        vec![official, pack(PERSONAL, "mine-1")],
    );
    let (mut copy, title) = log.begin_copy(&mine, 1, 2).expect("a copy");
    copy.canvas = 12;
    log.keep(copy.clone());
    log.name(ProjectId::new(12), title);
    let today = log.today();
    assert_eq!(today.copies[0].name.as_str(), "Colour copy 1");
    let own: Vec<&ItemRef> = today.pictures_of(None).map(|p| &p.item).collect();
    assert_eq!(own, [&mine]);
    assert_eq!(
        today.picture(&mine).map(|p| (p.copies, p.sessions)),
        Some((1, 0))
    );
    log.name(ProjectId::new(12), Title::new("Lamp study").expect("title"));
    assert_eq!(log.today().copies[0].name.as_str(), "Lamp study");
    // The user's pictures all removed: the copy stays, its picture gone.
    log.set_personal(None);
    let today = log.today();
    assert_eq!(today.pictures_of(None).count(), 0);
    assert_eq!(today.copies[0].picture, None);
    assert_eq!(
        today.pictures_of(Some(Tier::Simple)).count(),
        1,
        "the official pack stays"
    );
    log.forget(copy.id);
    assert!(log.today().copies.is_empty());
}
