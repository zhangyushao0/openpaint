//! Contrast thumbnails on a practice sheet (S154): a composition's shapes printed faint with the
//! ring at its focus, painted in the three greys from Middle on (Fill frame the overall tone),
//! compared by the thumbnail's own pixels read on the runtime's worker (its veil, the rules as
//! marks), no number kept, the eye's three asked once read; a lesson of them as the path starts it
//! once a pack holds a composition whose masses read, unit 8's skill.

mod support;

use std::sync::Arc;

use openpaint_app::{BrushLibrary, Toolbox, Viewport};
use openpaint_core::{Artwork, CanvasSize, Color, StrokeStyle};
use openpaint_practice::{
    Emphasis, Exercise, Figure, ItemRef, Judge, Kind, Level, LinePen, Masses, Needs, Outcome, Pack,
    PackItem, PictureKind, Skill, SkillLevel, ThumbnailCompared, ThumbnailReading, Tier, UNITS,
    Value, ValueMasses, Verdict, distance,
};
use openpaint_practice_app::{
    Judged, Pen, PictureImage, PracticeIntent, PracticeLog, PracticePens, SheetSession, Start,
};

const VIEWPORT: Viewport = Viewport {
    origin: [0.0, 84.0],
    size: [2880.0, 1716.0],
    pixels_per_point: 1.75,
};
const NOW: u64 = 1_791_900_000;

/// A composition pack of one landscape, 800 by 600, its masses kept.
fn compositions() -> Pack {
    let (size, pixels) = Masses::generate(9, 0.75).picture(800);
    Pack {
        id: "test-compositions".to_owned(),
        title: "Test compositions".to_owned(),
        items: vec![PackItem {
            id: "harbour".to_owned(),
            kind: PictureKind::Composition,
            tier: Tier::Simple,
            size,
            title: "Test".to_owned(),
            credit: None,
            outline: None,
            masses: Some(Arc::new(ValueMasses::of(size, &pixels))),
            areas: None,
        }],
    }
}

fn opened(kind: Kind) -> SheetSession {
    let packs = [compositions()];
    let exercise = Exercise {
        picture: Some(ItemRef {
            pack: "test-compositions".to_owned(),
            item: "harbour".to_owned(),
        }),
        ..Exercise::generated(kind, 0x5EED_0154)
    };
    let record = support::laid(vec![exercise], 0).framed(&packs);
    let [width, height] = openpaint_practice::Layout::of_sheet(&record).size;
    let sheet = Artwork::new(CanvasSize::new(width, height).expect("size"));
    let opening = support::opening(&record, (&packs, Vec::new()), [width, height]);
    let library = BrushLibrary::built_in().expect("built-in brushes");
    let style = Toolbox::new(library).style().expect("style");
    let brush =
        StrokeStyle::new(Arc::clone(style.recipe()), 8.0, 1.0, Color::BLACK).expect("brush");
    let pens = PracticePens::built_in().expect("pens");
    let mut session = SheetSession::open(sheet, (brush, pens), opening, &VIEWPORT);
    practise(&mut session, PracticeIntent::Instructions(false));
    session
}

fn practise(session: &mut SheetSession, intent: PracticeIntent) {
    session.practise(intent, NOW).expect("practised");
}

/// A thumbnail's frame, 400 by 300 pixels: dark against light round `centre` (frame units), the
/// middle grey elsewhere.
fn focused(centre: [f64; 2]) -> PictureImage {
    let pixels = (0..400 * 300)
        .flat_map(|at| {
            let p = [at % 400, at / 400].map(|v| (f64::from(v) + 0.5) / 400.0);
            let value = match distance(p, centre) {
                d if d <= 0.035 => Value::Dark,
                d if d <= 0.07 => Value::Light,
                _ => Value::Middle,
            };
            let [r, g, b] = value.grey();
            [r, g, b, 255]
        })
        .collect();
    PictureImage {
        size: [400, 300],
        pixels,
    }
}

#[test]
fn a_thumbnail_is_printed_painted_in_three_greys_and_waits_for_the_eye_once_read() {
    let mut session = opened(Kind::Thumbnail(Emphasis::Own));
    let practice = session.projection();
    assert_eq!(practice.pen.map(|(pen, _)| pen), Some(Pen::Middle));
    for refused in [Pen::Sketch, Pen::Draw, Pen::Big] {
        assert!(session.practise(PracticeIntent::Pen(refused), NOW).is_err());
    }
    // Printed: its shapes faint, its ring in the strong pen's red; its values nowhere.
    let givens = &practice.sheet.givens[0];
    let pen = |wanted: LinePen| {
        (givens.iter())
            .filter(|mark| matches!(&mark.drawn, Figure::Line { pen, .. } if *pen == wanted))
            .count()
    };
    assert!(pen(LinePen::Faint) > 0, "its shapes");
    assert_eq!(pen(LinePen::Level(Level::Main)), 1, "one ring");
    assert!(
        practice.sheet.painted[0].is_none(),
        "no picture of its values"
    );
    assert!(!givens.iter().any(|mark| mark.drawn == Figure::Pictured));
    practise(&mut session, PracticeIntent::Fill(Color::BLACK));
    practise(&mut session, PracticeIntent::Compare(true));
    let job = session.sheet_mut().read_request().expect("asked for");
    assert_eq!(job.size, [400, 300]);
    let reading = job
        .reading
        .downcast::<ThumbnailReading>()
        .expect("a thumbnail's");
    let read = job.run(&focused(reading.rings[0].centre)).expect("read");
    let found = read.comparison.downcast::<ThumbnailCompared>();
    assert!(
        found
            .and_then(|c| c.read.as_ref())
            .is_some_and(|read| read.findings.is_empty())
    );
    assert_eq!(read.pictures.len(), 1, "its veil");
    session.sheet_mut().read(job.sequence, Some(read));
    let waiting = Judged {
        verdict: Verdict::Eye,
        result: None,
    };
    assert_eq!(
        session.projection().judged,
        Some(waiting),
        "the eye's three"
    );
    practise(&mut session, PracticeIntent::Judge(Outcome::Near));
    let kept = session.projection().judged.and_then(|judged| judged.result);
    assert_eq!(kept, Some((Outcome::Near, Judge::Eye)));
    let record = session.sheet_mut().save_request().expect("compared");
    let first = record.first(0).expect("compared");
    assert!(first.metrics.is_empty(), "no number kept");
}

#[test]
fn unit_8_teaches_contrast_thumbnails_laid_once_a_pack_holds_masses() {
    let skill = Skill::of_key("contrast-thumbnail").expect("its skill");
    // Unit 8's last skill, after organic arrows, contours and form intersections (S156).
    assert_eq!(UNITS[7].skills.last(), Some(&skill));
    let log = PracticeLog::new(Vec::new(), Vec::new(), vec![compositions()]);
    for (level, emphasis) in [
        (SkillLevel::First, Emphasis::Own),
        (SkillLevel::Second, Emphasis::Moved),
        (SkillLevel::Third, Emphasis::Two),
    ] {
        let (record, _) = (log.start(Start::At(skill, level), 7, NOW)).expect("a lesson");
        let kinds: Vec<Kind> = record.exercises().map(|laid| laid.exercise.kind).collect();
        assert_eq!(kinds, [Kind::Thumbnail(emphasis)], "{level:?}");
    }
    let bare = PracticeLog::new(Vec::new(), Vec::new(), Vec::new());
    let lacks = (bare.path().units.iter().flat_map(|unit| &unit.skills))
        .find(|tile| tile.skill == skill)
        .and_then(|tile| tile.lacks);
    assert_eq!(lacks, Some(Needs::Masses));
}
