//! Value masses and whole copies in colour blocks on a practice sheet (S96): a composition whose
//! masses read painted with the three greys only (Fill frame filling the frame, the picture's
//! shape, S99, in the grey chosen), compared by its copy read against the picture's answer on the
//! runtime's worker
//! (the answer's greys under the slider, the hatching where the copy differs, the share at the
//! answer's value kept); a whole copy painted in colour blocks on a layer under its division, the
//! division dimmed over them, a point of the blocks compared by the eyedropper; stage 4 from the
//! log with value masses and whole copies in turn.

use std::sync::Arc;

use openpaint_app::{
    BLOCKS_LAYER, BrushLibrary, CopyRead, DIVISION_DIMMED, DIVISION_LAYER, Overlay, Pen,
    PictureImage, PracticeIntent, PracticeLog, PracticePens, PracticePick, ReadAs, Session,
    SheetView, Title, Toolbox, Viewport, sheet_of, value_colour,
};
use openpaint_core::{Artwork, CanvasSize, Color, StrokeStyle};
use openpaint_practice::{
    Exercise, Family, ItemRef, Kind, Masses, Metric, Pack, PackItem, PictureKind, Plan,
    SessionRecord, Tier, Value, ValueMasses, value,
};

const VIEWPORT: Viewport = Viewport {
    origin: [0.0, 84.0],
    size: [2880.0, 1716.0],
    pixels_per_point: 1.75,
};
const NOW: u64 = 1_791_900_000;

/// A composition pack of two landscapes, 800 by 600, the first's masses kept, the second's not
/// (they did not read).
fn compositions() -> Pack {
    let item = |id: &str, tier: Tier, massed: bool| {
        let (size, pixels) = Masses::generate(9, 0.75).picture(800);
        PackItem {
            id: id.to_owned(),
            kind: PictureKind::Composition,
            tier,
            size,
            title: "Test".to_owned(),
            credit: None,
            outline: None,
            masses: massed.then(|| Arc::new(ValueMasses::of(size, &pixels))),
        }
    };
    Pack {
        id: "test-compositions".to_owned(),
        title: "Test compositions".to_owned(),
        items: vec![
            item("harbour", Tier::Simple, true),
            item("hills", Tier::Medium, false),
        ],
    }
}

fn opened(kind: Kind) -> Session {
    let packs = [compositions()];
    let exercise = Exercise {
        picture: Some(ItemRef {
            pack: "test-compositions".to_owned(),
            item: "harbour".to_owned(),
        }),
        ..Exercise::generated(kind, 0x5EED_0096)
    };
    let plan = Plan {
        exercises: vec![exercise],
        warm_up: 0,
    };
    let record = SessionRecord::begun(1, (4, 1), (5, 0), plan);
    let [width, height] = sheet_of(&record, &packs).size;
    let sheet = Artwork::new(CanvasSize::new(width, height).expect("size"));
    let view = SheetView::of(&record, &packs, ([width, height], &Title::untitled()));
    let library = BrushLibrary::built_in().expect("built-in brushes");
    let style = Toolbox::new(library.clone()).style().expect("style");
    let brush =
        StrokeStyle::new(Arc::clone(style.recipe()), 8.0, 1.0, Color::BLACK).expect("brush");
    let pens = PracticePens::of(&library).expect("pens");
    let tally = openpaint_practice::Tally::default();
    let mut session = Session::practising(
        sheet,
        (brush, pens),
        (record, Vec::new(), view, tally),
        &VIEWPORT,
    );
    practise(&mut session, PracticeIntent::Instructions(false));
    session
}

fn practise(session: &mut Session, intent: PracticeIntent) {
    session.practise(intent, NOW).expect("practised");
}

/// The frame's pixels (`frame` pixels, white paper) as the answer `masses` would be painted where
/// the picture lies (`place`: its top left and size, frame units: one to the frame's longer side).
fn painted(masses: &ValueMasses, [origin, size]: [[f64; 2]; 2], frame: [u32; 2]) -> PictureImage {
    let unit = f64::from(frame[0].max(frame[1]));
    let pixels = (0..frame[0] * frame[1])
        .flat_map(|at| {
            let p = [at % frame[0], at / frame[0]].map(|v| (f64::from(v) + 0.5) / unit);
            let share = [(p[0] - origin[0]) / size[0], (p[1] - origin[1]) / size[1]];
            let inside = share.iter().all(|s| (0.0..1.0).contains(s));
            let [r, g, b] = if inside {
                masses.at(share).grey()
            } else {
                [255; 3]
            };
            [r, g, b, 255]
        })
        .collect();
    PictureImage {
        size: frame,
        pixels,
    }
}

#[test]
fn value_masses_are_painted_in_three_greys_and_read_against_the_answer() {
    let mut session = opened(Kind::ValueMasses(Tier::Simple));
    let practice = session.projection().practice().cloned().expect("a sheet");
    assert_eq!(practice.family, Some(Family::ValueMasses));
    assert_eq!(practice.pen.map(|(pen, _)| pen), Some(Pen::Dark));
    assert!(practice.sheet.painted[0].is_some(), "the answer's greys");
    for refused in [Pen::Sketch, Pen::Draw, Pen::Main, Pen::Big] {
        assert!(session.practise(PracticeIntent::Pen(refused), NOW).is_err());
    }
    // Fill frame with Middle: the picture's place filled in its grey, whatever colour is asked.
    practise(&mut session, PracticeIntent::Pen(Pen::Middle));
    practise(&mut session, PracticeIntent::Fill(Color::BLACK));
    let fill = (session.artwork().layers().iter())
        .flat_map(|layer| layer.edits().iter())
        .find_map(|edit| edit.fill())
        .expect("a fill");
    let grey = value_colour(Value::Middle).expect("a grey");
    assert_eq!(fill.color(), grey);
    // The frame is the picture's shape (S99): Fill frame fills it whole.
    let frame = practice.sheet.pairs[0].copy;
    assert_eq!(frame.size, [400.0, 300.0]);
    let [x, y] = fill.mask().origin().map(f64::from);
    let [w, h] = fill.mask().size().map(f64::from);
    for (filled, edge) in [
        (x, frame.origin[0]),
        (y, frame.origin[1]),
        (w, 400.0),
        (h, 300.0),
    ] {
        assert!((filled - edge).abs() <= 1.0, "{filled} {edge}");
    }
    // Compare asks the copy of the worker, the frame read against the answer over it whole.
    practise(&mut session, PracticeIntent::Compare(true));
    let job = session.practice_read_request().expect("the copy asked for");
    assert_eq!(job.size, [400, 300]);
    let ReadAs::Masses(answer, place) = job.read.clone() else {
        panic!("{:?}", job.read);
    };
    assert_eq!(place, [[0.0, 0.0], [1.0, 0.75]]);
    let (read, metrics) = job.run(&painted(&answer, place, job.size)).expect("read");
    let CopyRead::Masses(check) = &read else {
        panic!("{read:?}");
    };
    assert!(check.hatching.is_empty() && check.matched > 0.99);
    session.practice_read(job.sequence, Some((read.clone(), metrics)));
    match session
        .projection()
        .practice()
        .and_then(|p| p.overlay.clone())
    {
        Some(Overlay::Masses { shown, read, .. }) => {
            assert!((shown - 0.5).abs() < 1e-6);
            assert!(read.is_some());
        }
        other => panic!("{other:?}"),
    }
    let record = session.practice_save_request().expect("compared");
    let kept = value(
        &record.result(0).expect("compared").metrics,
        Metric::MassesMatched,
    );
    assert!(kept.is_some_and(|share| share > 0.99), "{kept:?}");
}

/// The layer named `name`: its place among the layers, its opacity and whether it is drawn on.
fn layer(session: &Session, name: &str) -> (usize, f32, bool) {
    let artwork = session.artwork();
    let at = (artwork.layers().iter())
        .position(|layer| layer.name().as_str() == name)
        .unwrap_or_else(|| panic!("no layer {name}"));
    let layer = &artwork.layers()[at];
    (at, layer.props().opacity(), artwork.active() == layer.id())
}

#[test]
fn a_whole_copy_in_colour_blocks_paints_under_its_division_and_compares_a_point() {
    let mut session = opened(Kind::WholeCopy(Tier::Simple));
    assert_eq!(session.artwork().layers().len(), 3, "no blocks' layer yet");
    // Big: the blocks' layer made under the division, drawn on; the division dimmed over it.
    practise(&mut session, PracticeIntent::Pen(Pen::Big));
    let (blocks, _, drawn_on) = layer(&session, BLOCKS_LAYER);
    let (division, dimmed, _) = layer(&session, DIVISION_LAYER);
    assert!(drawn_on && blocks + 1 == division);
    assert_eq!(dimmed, DIVISION_DIMMED);
    // Fill frame fills the picture's place on it in the colour picked.
    let sky = Color::rgb(0.5, 0.7, 0.9).expect("a colour");
    practise(&mut session, PracticeIntent::Fill(sky));
    let artwork = session.artwork();
    let filled = artwork.layers()[blocks]
        .edits()
        .iter()
        .find_map(|e| e.fill());
    assert_eq!(filled.map(|fill| fill.color()), Some(sky));
    // Compared: a point of the blocks compared by the eyedropper (the picture read for the
    // screen); one left bare compares none.
    let (size, pixels) = Masses::generate(9, 0.75).picture(800);
    let item = session
        .practice_picture_request()
        .expect("its picture asked for");
    session.practice_picture_read(item, Some(PictureImage { size, pixels }));
    practise(&mut session, PracticeIntent::Compare(true));
    let practice = session.projection().practice().cloned().expect("a sheet");
    let frame = practice.sheet.pairs[0].copy;
    let picture = practice.sheet.pictures[0].clone().expect("its picture");
    let at = picture.fit.to_frame([400.0, 300.0]);
    let point = frame.to_canvas(at).map(|v| v as f32);
    assert!(matches!(
        session.practice_pick(point),
        PracticePick::Probe { .. }
    ));
    session.practice_probed([128, 180, 230]);
    let probe = session.projection().practice().and_then(|p| p.probe);
    assert!(probe.is_some_and(|probe| probe.copy == [128, 180, 230]));
    let _ = session.practice_pick(point);
    session.practice_probed([255; 3]);
    assert!(
        session
            .projection()
            .practice()
            .and_then(|p| p.probe)
            .is_none()
    );
    // Draw: the line drawing's layer drawn on, the division still dimmed.
    practise(&mut session, PracticeIntent::Compare(false));
    practise(&mut session, PracticeIntent::Pen(Pen::Draw));
    assert_eq!(layer(&session, DIVISION_LAYER).1, DIVISION_DIMMED);
    assert!(layer(&session, openpaint_app::LINES_LAYER).2);
}

#[test]
fn stage_four_holds_value_masses_and_whole_copies_in_turn_and_drills_them() {
    let log = PracticeLog::new(Vec::new(), Vec::new(), vec![compositions()]);
    assert!(log.today().masses);
    let (record, _) = log.begin((4, None), 0x5EED_0096, NOW);
    let families: Vec<Family> = record.exercises.iter().map(|e| e.kind.family()).collect();
    assert_eq!(&families[3..], [Family::Division, Family::ValueMasses]);
    // Only the picture whose masses read is offered for value masses.
    let masses = record.exercises[4]
        .picture
        .as_ref()
        .map(|item| item.item.as_str());
    assert_eq!(masses, Some("harbour"));
    let (drill, _) = (log.begin_drill(Family::ValueMasses, 7, NOW)).expect("a value masses drill");
    assert_eq!(drill.exercises.len(), 3);
    assert!(
        drill
            .exercises
            .iter()
            .all(|e| e.kind.family() == Family::ValueMasses)
    );
    let bare = PracticeLog::new(Vec::new(), Vec::new(), Vec::new());
    assert!(!bare.today().masses);
    assert!(bare.begin_drill(Family::ValueMasses, 7, NOW).is_none());
}
