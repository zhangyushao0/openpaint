//! Texture on a practice sheet (S154j): the strip's picture in the original's frame, both frames of
//! its shape, twice as wide as high; the darkest pen alone, no fill; Compare reads the copy on the
//! worker, lays the answer's shadows at the slider, rings each shadow missed and outlines each mark
//! over light, then waits for the learner's eye (D123): nothing kept, the result chosen once.

use std::sync::Arc;

use openpaint_app::{
    BrushLibrary, Contact, ContactPhase, Device, InputSample, Intent, StrokeIntent, Title, Toolbox,
    Viewport,
};
use openpaint_core::{Artwork, CanvasSize, Color, StrokeStyle};
use openpaint_practice::{
    DARKEST, Figure, Frame, Given, Judge, Kind, Layout, Outcome, Pen, Point, Sheet, SheetMark,
    Skill, SkillLevel, Source, TextureCompared, TextureReading, TextureTask, Verdict,
};
use openpaint_practice_app::{
    Judged, PictureImage, PracticeIntent, PracticeLog, PracticePens, SheetSession, Start,
};

const VIEWPORT: Viewport = Viewport {
    origin: [0.0, 84.0],
    size: [2880.0, 1716.0],
    pixels_per_point: 1.75,
};
/// 2026-10-05 12:00 UTC.
const NOON: u64 = 1_791_201_600;

/// A first-level lesson of texture, open on a sheet laid ahead.
fn opened() -> SheetSession {
    let log = PracticeLog::new(Vec::new(), Vec::new(), Vec::new());
    let skill = Skill::of_key("texture").expect("the skill");
    let start = Start::At(skill, SkillLevel::First);
    let (record, _) = log.start(start, 7, NOON).expect("a lesson");
    let record = Sheet {
        canvas: 40,
        ..record
    };
    let size = Layout::laid_ahead(&record);
    let sheet = Artwork::new(CanvasSize::new(size[0], size[1]).expect("size"));
    let library = BrushLibrary::built_in().expect("built-in brushes");
    let style = Toolbox::new(library).style().expect("style");
    let brush =
        StrokeStyle::new(Arc::clone(style.recipe()), 8.0, 1.0, Color::BLACK).expect("brush");
    let pens = PracticePens::built_in().expect("pens");
    let printed = Given::of_sheet(&record, log.packs());
    let opening = log.practising(&record, &printed, (size, &Title::untitled()));
    SheetSession::open(sheet, (brush, pens), opening, &VIEWPORT)
}

/// A pen stroke along `points` (frame units of `frame`).
fn draw(session: &mut SheetSession, frame: &Frame, points: &[Point]) {
    let path: Vec<[f32; 2]> = (openpaint_practice::resample(points, 0.002).into_iter())
        .map(|p| frame.to_canvas(p).map(|v| v as f32))
        .collect();
    let sample = |index: usize| InputSample {
        position: path[index],
        pressure: 0.7,
        tilt: 0.2,
        azimuth: 0.0,
        time_ms: 1000 + 4 * index as i64,
    };
    let last = path.len() - 1;
    for (phase, samples) in [
        (ContactPhase::Down, vec![sample(0)]),
        (ContactPhase::Move, (1..last).map(sample).collect()),
        (ContactPhase::Up, vec![sample(last)]),
    ] {
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

fn practise(session: &mut SheetSession, intent: PracticeIntent) {
    session.practise(intent, NOON + 60).expect("practised");
}

#[test]
fn a_strip_lies_in_frames_of_its_shape_painted_with_the_darkest_alone() {
    let mut session = opened();
    let practice = session.projection();
    assert!(matches!(practice.kind, Some(Kind::Texture(_))));
    let pair = practice.sheet.pairs[0];
    assert_eq!(pair.copy.size, [400.0, 200.0], "twice as wide as high");
    assert_eq!(pair.original.size, pair.copy.size);
    assert_eq!(practice.pen.map(|(pen, _)| pen), Some(Pen::Darkest));
    let tools = practice
        .kind
        .map(|kind| kind.facts().tools)
        .expect("its tools");
    assert_eq!((tools.pens, tools.fill), (&[Pen::Darkest][..], None));
    for other in [Pen::Dark, Pen::Light, Pen::Draw] {
        assert!(session.practise(PracticeIntent::Pen(other), NOON).is_err());
    }
    // The strip's picture in the original's frame.
    let pictured = (practice.sheet.givens[0].iter()).any(|mark| {
        matches!(
            mark.drawn,
            Figure::Picture {
                source: Source::Painted,
                ..
            }
        )
    });
    assert!(pictured);
}

#[test]
fn compare_rings_a_shadow_missed_outlines_a_mark_over_light_and_waits_for_the_eye() {
    let mut session = opened();
    let practice = session.projection();
    let frame = practice.sheet.pairs[0].copy;
    let original = practice.original.as_deref().expect("in hand");
    let task = original
        .downcast::<TextureTask>()
        .expect("a texture")
        .clone();
    draw(&mut session, &frame, &[[0.2, 0.2], [0.3, 0.22]]);
    practise(&mut session, PracticeIntent::Compare(true));
    let job = session
        .sheet_mut()
        .read_request()
        .expect("the copy asked for");
    assert!(job.reading.downcast::<TextureReading>().is_some());
    let image = one_missed_one_over(&task, job.size);
    session.sheet_mut().read(job.sequence, job.run(&image));
    let practice = session.projection();
    let compared = (practice.compared.as_ref())
        .and_then(|c| c.downcast::<TextureCompared>())
        .expect("read")
        .clone();
    let check = compared.check.expect("read");
    assert_eq!((check.missed.len(), check.marks_over), (1, 1));
    let overlay = practice.overlay.clone().expect("laid");
    let count = |of: &dyn Fn(&SheetMark) -> bool| overlay.iter().filter(|m| of(m)).count();
    assert_eq!(count(&|m| matches!(m.drawn, Figure::Ring(_))), 1);
    assert_eq!(
        count(&|m| matches!(m.drawn, Figure::Haloed { red: true, .. })),
        1
    );
    let answer_laid = |m: &SheetMark| {
        matches!(
            m.drawn,
            Figure::Picture {
                source: Source::Read(0),
                ..
            }
        )
    };
    assert_eq!(count(&answer_laid), 1);
    let waiting = Judged {
        verdict: Verdict::Eye,
        result: None,
    };
    assert_eq!(practice.judged, Some(waiting));
    let record = session.sheet().record().clone();
    assert!(record.first(0).expect("compared").metrics.is_empty());
    practise(&mut session, PracticeIntent::Judge(Outcome::Redo));
    let judged = session.projection().judged.expect("judged");
    assert_eq!(judged.result, Some((Outcome::Redo, Judge::Eye)));
}

/// A copy of `size` with every shadow `task` asks for but the first, and a dab where the light
/// falls.
fn one_missed_one_over(task: &TextureTask, size: [u32; 2]) -> PictureImage {
    let shadows = task.shadows(size);
    let [w, h] = shadows.size;
    let answer = shadows.answer();
    let skipped = &shadows.blobs[0];
    let lit = lit_square(&shadows.shadow, [w, h]);
    let pixels: Vec<u8> = (0..w * h)
        .flat_map(|at| {
            let (x, y) = (at % w, at / w);
            let dab = (lit[0]..lit[0] + 14).contains(&x) && (lit[1]..lit[1] + 14).contains(&y);
            let dark = (answer[at] && !skipped.contains(&at)) || dab;
            let [r, g, b] = if dark { DARKEST } else { [255; 3] };
            [r, g, b, 255]
        })
        .collect();
    PictureImage { size, pixels }
}

/// The top left of a 14-pixel square of `shadow` (a grid `[w, h]`) with no shadow within 20
/// pixels of it, inside the grid by 20.
fn lit_square(shadow: &[bool], [w, h]: [usize; 2]) -> [usize; 2] {
    let clear = |x: usize, y: usize| {
        (y - 20..y + 34).all(|yy| (x - 20..x + 34).all(|xx| !shadow[yy * w + xx]))
    };
    (20..h - 34)
        .flat_map(|y| (20..w - 34).map(move |x| (x, y)))
        .find(|(x, y)| clear(*x, *y))
        .map(|(x, y)| [x, y])
        .expect("a lit place")
}
