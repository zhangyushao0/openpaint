//! Drawing from life (S151) in the app: begun only against a photo of one's own, its frames the
//! photo's shape at a lesson's size, its photo's frame covered and never picked while it is drawn
//! with the drawing pens; Compare showing the photo and laying it over the drawing in place and
//! Aligned at the slider's strength, no result asked or kept; no lesson, dot or seal counting it,
//! the options listing it apart from the colour copies, the practice book keeping it with the
//! first lesson's drawing as its "before"; its photo no shape lesson's picture (S158).

mod support;

use std::sync::Arc;

use openpaint_app::{
    BrushLibrary, Contact, ContactPhase, Device, InputSample, Intent, StrokeIntent, Toolbox,
    Viewport,
};
use openpaint_core::{Artwork, CanvasSize, Color, StrokeStyle};
use openpaint_practice::{
    CardItem, Exercise, Figure, Given, ItemRef, Kind, Layout, LineShown, ObservedCompared, Outcome,
    Outline, PERSONAL, Pack, PackItem, PictureKind, Place, Purpose, Seal, Sheet, Skill, SkillLevel,
    Span, Tier, Verdict, schedule,
};
use openpaint_practice_app::{
    Pen, PictureImage, PracticeIntent, PracticeLog, PracticePens, PracticePick, SHOWN, SheetRead,
    SheetSession, Start,
};

const VIEWPORT: Viewport = Viewport {
    origin: [0.0, 84.0],
    size: [2880.0, 1716.0],
    pixels_per_point: 1.75,
};
/// 2026-10-05 12:00 UTC.
const NOON: u64 = 1_791_201_600;

fn photo() -> ItemRef {
    ItemRef {
        pack: PERSONAL.to_owned(),
        item: "mine-1".to_owned(),
    }
}

/// A pack of one picture 800 by 600, `pack`'s item `item`, its silhouette a box in its middle.
fn pack(pack: &str, item: &str) -> Pack {
    let silhouette = vec![
        [200.0, 150.0],
        [600.0, 150.0],
        [600.0, 450.0],
        [200.0, 450.0],
    ];
    Pack {
        id: pack.to_owned(),
        title: "Pictures".to_owned(),
        items: vec![PackItem {
            id: item.to_owned(),
            kind: PictureKind::Shape,
            tier: Tier::Simple,
            size: [800, 600],
            title: "Picture 1".to_owned(),
            credit: None,
            outline: Some(Outline {
                contours: vec![silhouette],
            }),
            masses: None,
            areas: None,
        }],
    }
}

/// The official shapes and the user's own photo.
fn packs() -> Vec<Pack> {
    vec![pack("test-shapes", "lamp"), pack(PERSONAL, "mine-1")]
}

/// The first lesson (sheet 1), finished, practised two minutes.
fn first_lesson() -> Sheet {
    let mut sheet = Sheet::begun(1, (Purpose::Day, NOON), schedule::first(7));
    sheet.canvas = 3;
    sheet.spans = vec![Span {
        from: NOON,
        to: NOON + 120,
    }];
    sheet.lessons[0].finished = Some(NOON + 120);
    sheet
}

/// `record` opened as a log keeping the packs and `others` opens it, its How put away.
fn opened(record: &Sheet, others: Vec<Sheet>) -> SheetSession {
    let size = Layout::of_sheet(record).size;
    let sheet = Artwork::new(CanvasSize::new(size[0], size[1]).expect("size"));
    let library = BrushLibrary::built_in().expect("built-in brushes");
    let style = Toolbox::new(library).style().expect("style");
    let brush =
        StrokeStyle::new(Arc::clone(style.recipe()), 8.0, 1.0, Color::BLACK).expect("brush");
    let pens = PracticePens::built_in().expect("pens");
    let opening = support::opening(record, (&packs(), others), size);
    let mut session = SheetSession::open(sheet, (brush, pens), opening, &VIEWPORT);
    practise(&mut session, PracticeIntent::Instructions(false));
    session
}

fn practise(session: &mut SheetSession, intent: PracticeIntent) {
    session.practise(intent, NOON + 3700).expect("practised");
}

/// A pen stroke round `corners` of the shown exercise's copy frame (frame units).
fn draw(session: &mut SheetSession, corners: &[[f64; 2]]) {
    let practice = session.projection();
    let frame = practice.sheet.pairs[practice.current.expect("shown")].copy;
    let sample = |index: usize| InputSample {
        position: frame.to_canvas(corners[index]).map(|v| v as f32),
        pressure: 0.8,
        tilt: 0.2,
        azimuth: 0.0,
        time_ms: 1000 + 8 * index as i64,
    };
    let last = corners.len() - 1;
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
        let stroke = Intent::Stroke(StrokeIntent::Contact(contact));
        session.dispatch(stroke).expect("dispatch");
    }
}

/// The photo's silhouette in frame units (an 800-pixel photo: a pixel is 1/800), `dx` across.
fn silhouette(dx: f64) -> Vec<[f64; 2]> {
    let corners = [
        [200.0, 150.0],
        [600.0, 150.0],
        [600.0, 450.0],
        [200.0, 450.0],
        [200.0, 150.0],
    ];
    corners
        .map(|[x, y]: [f64; 2]| [x / 800.0 + dx, y / 800.0])
        .to_vec()
}

/// The photo laid over the copy while compared: where it lies and how strongly.
fn laid(session: &SheetSession) -> Option<([[f64; 2]; 4], f32)> {
    (session.projection().overlay.iter().flatten()).find_map(|mark| match &mark.drawn {
        Figure::Picture {
            place: Place::Corners(corners),
            shown,
            ..
        } => Some((*corners, *shown)),
        _ => None,
    })
}

#[test]
fn only_a_photo_of_one_s_own_begins_a_drawing_from_life_drawn_beside_its_covered_photo() {
    let log = PracticeLog::new(Vec::new(), Vec::new(), packs());
    let official = ItemRef {
        pack: "test-shapes".to_owned(),
        item: "lamp".to_owned(),
    };
    assert!(log.begin_from_life(&official, 9, NOON).is_none());
    let (record, title) = log
        .begin_from_life(&photo(), 9, NOON)
        .expect("a photo of mine");
    assert_eq!(title.as_str(), "Drawing from life 1");
    assert!(record.is_copy() && record.drawn_from_life());
    // The photo's shape at a lesson's frame: the drawing pens' own size.
    let pair = Layout::of_sheet(&record).pairs[0];
    assert_eq!([pair.original.size, pair.copy.size], [[400.0, 300.0]; 2]);
    let mut session = opened(&record, Vec::new());
    let shown = session.projection();
    assert!(shown.covered, "the photo out of sight");
    assert!(
        shown.copy && !shown.all_tools,
        "drawn with the exercise's own pens"
    );
    assert_eq!(shown.pen.map(|(pen, _)| pen), Some(Pen::Draw));
    assert!(!shown.compare.is_offered(), "nothing drawn yet");
    let middle = pair.original.middle().map(|v| v as f32);
    assert_eq!(session.sheet_mut().pick(middle), PracticePick::Refused);
    assert_eq!(session.projection().notice, None, "refused silently");
    practise(&mut session, PracticeIntent::AllTools(true));
    practise(&mut session, PracticeIntent::AllTools(false));
}

#[test]
fn compare_shows_the_photo_and_lays_it_in_place_and_aligned_keeping_no_result() {
    let log = PracticeLog::new(Vec::new(), Vec::new(), packs());
    let (record, _) = log
        .begin_from_life(&photo(), 9, NOON)
        .expect("a photo of mine");
    let mut session = opened(&record, Vec::new());
    let item = session
        .sheet_mut()
        .picture_request()
        .expect("the photo read");
    let image = PictureImage {
        size: [800, 600],
        pixels: [90, 120, 60, 255].repeat(800 * 600),
    };
    session.sheet_mut().picture_read(item, Some(image));
    draw(&mut session, &silhouette(0.05));
    practise(&mut session, PracticeIntent::Compare(true));
    let shown = session.projection();
    assert!(!shown.covered, "the photo shows from the first Compare on");
    let compared = shown.compared.as_ref().expect("compared");
    let observed = compared.downcast::<ObservedCompared>().expect("the photo");
    assert!(observed.outlined && shown.aligned_offered);
    assert_eq!(shown.card[..2], [CardItem::Overlaid, CardItem::Slider]);
    assert_eq!(laid(&session), Some((observed.corners, SHOWN)), "in place");
    assert_eq!(
        shown.judged.map(|judged| judged.verdict),
        Some(Verdict::Nobody)
    );
    let judging = session.practise(PracticeIntent::Judge(Outcome::Clean), NOON + 3700);
    assert!(judging.is_err(), "no result to choose");
    practise(&mut session, PracticeIntent::Shown(0.3));
    assert_eq!(laid(&session).map(|(_, shown)| shown), Some(0.3));
    practise(&mut session, PracticeIntent::Aligned(true));
    let job = session.sheet_mut().fit_request().expect("its fit asked");
    session.sheet_mut().fitted(job.sequence, job.run());
    let (corners, _) = laid(&session).expect("laid aligned");
    let moved = corners[0][0] - observed.corners[0][0];
    assert!((moved - 0.05).abs() < 0.01, "onto the drawing: {moved}");
    let saved = session
        .sheet_mut()
        .save_request()
        .expect("its Compare kept");
    let laid = saved.laid(0).expect("the drawing");
    assert!(laid.first.is_some() && laid.result.is_none(), "no result");
}

#[test]
fn no_lesson_dot_or_seal_counts_it_and_the_book_keeps_it_after_the_first_lesson_s_drawing() {
    let first = first_lesson();
    let mut log = PracticeLog::new(vec![first.clone()], Vec::new(), packs());
    let (lessons, learner) = (log.path().book.book.lessons, log.learner().clone());
    let (mut drawing, title) = log
        .begin_from_life(&photo(), 9, NOON + 3600)
        .expect("a photo");
    drawing.canvas = 5;
    drawing.spans = vec![Span {
        from: NOON + 3600,
        to: NOON + 4500,
    }];
    let id = drawing.id;
    log.keep(drawing);
    log.name(openpaint_app::ProjectId::new(5), title);
    assert_eq!(log.learner(), &learner, "no lesson, no dot");
    let path = log.path();
    let book = &path.book.book;
    assert_eq!(book.lessons, lessons);
    assert!(
        book.seals
            .iter()
            .all(|earned| earned.seal != Seal::FirstCopy)
    );
    assert!(
        book.days.iter().any(|day| day.records.contains(&id)),
        "kept by its day"
    );
    let drawn = |record: u64| LineShown {
        record,
        number: 1,
        index: 0,
    };
    assert_eq!(
        book.life,
        [drawn(first.id), drawn(id)],
        "the first lesson's its before"
    );
    assert!(path.copies[0].from_life);
    assert_eq!(path.copies[0].name.as_str(), "Drawing from life 1");
    let used = path
        .picture(&photo())
        .map(|p| (p.lessons, p.copies, p.drawings));
    assert_eq!(used, Some((0, 0, 1)));
    let official = ItemRef {
        pack: "test-shapes".to_owned(),
        item: "lamp".to_owned(),
    };
    let (_, copy) = log
        .begin_copy(&official, 4, NOON + 5000)
        .expect("a colour copy");
    assert_eq!(copy.as_str(), "Colour copy 1", "numbered among its own");
}

/// S158: the photo a drawing from life was drawn against is no shape lesson's picture (a photo of
/// one's table is no outline to copy); it stays one of My pictures, and a picture of one's own
/// never drawn from life still comes up.
#[test]
fn a_photo_drawn_from_life_is_never_a_shape_lesson_s_picture() {
    let start = Start::At(Skill::of_key("shape").expect("shapes"), SkillLevel::First);
    let picture = |sheet: &Sheet| sheet.laid(0).and_then(|laid| laid.exercise.picture.clone());
    let only = PracticeLog::new(Vec::new(), Vec::new(), vec![pack(PERSONAL, "mine-1")]);
    let (lesson, _) = only.start(start, 3, NOON).expect("my picture to copy");
    assert_eq!(picture(&lesson), Some(photo()));
    let (drawing, _) = only.begin_from_life(&photo(), 9, NOON).expect("a photo");
    let drawn = PracticeLog::new(
        vec![drawing.clone()],
        Vec::new(),
        vec![pack(PERSONAL, "mine-1")],
    );
    assert!(drawn.start(start, 3, NOON).is_none(), "no shape to copy");
    let shapes = (drawn.path().units.iter())
        .flat_map(|unit| unit.skills.clone())
        .find(|shown| shown.skill.key() == "shape");
    assert!(shapes.is_some_and(|shown| shown.lacks.is_some()));
    assert!(drawn.begin_copy(&photo(), 4, NOON).is_some(), "still mine");
    // Another picture of mine is a shape lesson's still.
    let mut mine = pack(PERSONAL, "mine-1");
    mine.items.extend(pack(PERSONAL, "mine-2").items);
    let both = PracticeLog::new(vec![drawing], Vec::new(), vec![mine]);
    let (lesson, _) = both.start(start, 3, NOON).expect("the other to copy");
    assert_eq!(
        picture(&lesson).map(|item| item.item),
        Some("mine-2".to_owned())
    );
}

#[test]
fn the_book_reads_a_drawing_of_what_lay_in_front_as_drawn_without_an_original() {
    // The first lesson's first exercise, never compared.
    let sheet = support::laid(vec![Exercise::generated(Kind::Free, 7)], NOON);
    let mut session = opened(&sheet, Vec::new());
    draw(&mut session, &[[0.3, 0.3], [0.5, 0.6], [0.7, 0.4]]);
    let printed = Given::of_sheet(&sheet, &[]);
    let read = SheetRead::of(session.session().artwork(), &sheet, &printed);
    let drawing = (read.lines.iter())
        .find(|line| line.index == 0)
        .expect("its drawing");
    assert!(drawing.original.is_empty() && !drawing.copy.is_empty());
    assert_eq!(drawing.extent, [1.0, 1.0]);
}
