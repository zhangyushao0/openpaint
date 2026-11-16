//! Colour under light on a practice sheet (S154j): the form, the pin and where the families meet
//! printed in one frame, the colours to mix from beside it, each named and picked whole; Compare
//! reads the copy's regions on the worker, lays the model's render as one answer, outlines what
//! breaks a rule, and waits for the learner's eye (D123): nothing kept, the result chosen once.

use std::sync::Arc;

use openpaint_app::{BrushLibrary, Title, Toolbox, Viewport};
use openpaint_core::{Artwork, CanvasSize, Color, StrokeStyle};
use openpaint_practice::{
    CardItem, ColourTask, Figure, Given, Judge, Kind, Layout, LitCompared, LitReading, Outcome,
    Palette, Region, Rule, Sheet, SheetMark, Skill, SkillLevel, Source, Verdict, beside,
};
use openpaint_practice_app::{
    Judged, PictureImage, PracticeIntent, PracticeLog, PracticePens, PracticePick, SheetSession,
    Start,
};

const VIEWPORT: Viewport = Viewport {
    origin: [0.0, 84.0],
    size: [2880.0, 1716.0],
    pixels_per_point: 1.75,
};
/// 2026-10-05 12:00 UTC.
const NOON: u64 = 1_791_201_600;

/// A lesson of colour under light at `level`, open on a sheet laid ahead.
fn opened(level: SkillLevel) -> SheetSession {
    let log = PracticeLog::new(Vec::new(), Vec::new(), Vec::new());
    let skill = Skill::of_key("colour-under-light").expect("the skill");
    let (record, _) = (log.start(Start::At(skill, level), 7, NOON)).expect("a lesson");
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

fn practise(session: &mut SheetSession, intent: PracticeIntent) {
    session.practise(intent, NOON + 60).expect("practised");
}

/// The task in hand.
fn task(session: &SheetSession) -> ColourTask {
    let practice = session.projection();
    let original = practice.original.as_deref().expect("in hand");
    original
        .downcast::<ColourTask>()
        .expect("colour under light")
        .clone()
}

/// Filled, compared, and the copy read as `paint` paints it at the size the worker reads.
fn compared(session: &mut SheetSession, paint: impl Fn(&ColourTask, u32) -> Vec<u8>) {
    let task = task(session);
    practise(session, PracticeIntent::Fill(Color::BLACK));
    practise(session, PracticeIntent::Compare(true));
    let job = session
        .sheet_mut()
        .read_request()
        .expect("the copy asked for");
    assert!(job.reading.downcast::<LitReading>().is_some());
    assert_eq!(job.size[0], job.size[1], "one square frame");
    let pixels = PictureImage {
        size: job.size,
        pixels: paint(&task, job.size[0]),
    };
    session.sheet_mut().read(job.sequence, job.run(&pixels));
}

/// The comparison shown.
fn comparison(session: &SheetSession) -> LitCompared {
    let practice = session.projection();
    let compared = practice.compared.as_ref().expect("compared");
    compared.downcast::<LitCompared>().expect("read").clone()
}

fn outlined(overlay: &[SheetMark]) -> usize {
    (overlay.iter())
        .filter(|mark| matches!(mark.drawn, Figure::Haloed { red: true, .. }))
        .count()
}

#[test]
fn the_colours_lie_beside_the_frame_named_and_picked_whole() {
    let mut session = opened(SkillLevel::First);
    let practice = session.projection();
    let Some(Kind::LitColour(lit)) = practice.kind else {
        panic!("colour under light: {:?}", practice.kind)
    };
    assert_eq!(lit.palette, Palette::WarmSun);
    let swatches: Vec<([f64; 2], [u8; 3], &str)> = (practice.sheet.givens[0].iter())
        .filter_map(|mark| match mark.drawn {
            Figure::Beside {
                at,
                colour,
                name: Some(name),
            } => Some((at, colour, name)),
            _ => None,
        })
        .collect();
    let names: Vec<&str> = swatches.iter().map(|(_, _, name)| *name).collect();
    assert_eq!(names, ["local-colour", "sunlight", "skylight"]);
    let colours = task(&session).colours;
    assert_eq!(swatches[1].1, colours.light);
    let pair = practice.sheet.pairs[0];
    for (at, colour, _) in swatches {
        let on = beside(&pair.copy, at).map(|v| v as f32);
        let [r, g, b] = colour.map(|c| f32::from(c) / 255.0);
        let picked = Color::rgb(r, g, b).expect("a colour");
        assert_eq!(session.sheet_mut().pick(on), PracticePick::Colour(picked));
    }
    // The one frame: the form and the pin printed, where the families meet dashed.
    let dashed = (practice.sheet.givens[0].iter())
        .filter(|mark| matches!(mark.drawn, Figure::Line { dashed: true, .. }))
        .count();
    assert!(dashed > 0);
}

#[test]
fn compare_lays_one_answer_and_waits_for_the_eye() {
    let mut session = opened(SkillLevel::Second);
    compared(&mut session, |task, side| task.flat(&[], side));
    let read = comparison(&session).read.expect("read");
    assert_eq!(read.broken, [], "the model painted breaks no rule");
    let practice = session.projection();
    let waiting = Judged {
        verdict: Verdict::Eye,
        result: None,
    };
    assert_eq!(practice.judged, Some(waiting), "the learner's to judge");
    // The card: the disc of the colours given, Grey, one answer and its slider; no reason line.
    let CardItem::Mixed {
        given,
        copy,
        marked,
    } = &practice.card[0]
    else {
        panic!("the disc first: {:?}", practice.card)
    };
    assert_eq!((given.len(), copy.len(), marked.len()), (3, 4, 0));
    assert_eq!(practice.card[1], CardItem::Grey);
    assert!(matches!(&practice.card[2], CardItem::Says(say) if say.key == "one-answer"));
    assert_eq!(practice.card[3], CardItem::Slider);
    let overlay = practice.overlay.clone().expect("the answer laid");
    let laid = |mark: &SheetMark| {
        matches!(
            mark.drawn,
            Figure::Picture {
                source: Source::Painted,
                ..
            }
        )
    };
    assert!(overlay.iter().any(laid));
    assert_eq!(outlined(&overlay), 0);
    // Nothing kept: the learner's choice, once.
    let record = session.sheet().record().clone();
    assert!(record.first(0).expect("compared").metrics.is_empty());
    practise(&mut session, PracticeIntent::Judge(Outcome::Near));
    let judged = session.projection().judged.expect("judged");
    assert_eq!(judged.result, Some((Outcome::Near, Judge::Eye)));
}

#[test]
fn a_shadow_the_local_darkened_is_outlined_on_the_copy_and_ringed_on_the_disc() {
    let mut session = opened(SkillLevel::Second);
    compared(&mut session, |task, side| {
        let wrong = task.wrong(Rule::ShadowColour).expect("a wrong copy");
        task.flat(&wrong, side)
    });
    let read = comparison(&session).read.expect("read");
    assert_eq!(read.broken, [(Rule::ShadowColour, Region::Shadow)]);
    let practice = session.projection();
    let shadow = (read.colours.iter()).position(|(region, _)| *region == Region::Shadow);
    let CardItem::Mixed { marked, .. } = &practice.card[0] else {
        panic!("the disc first")
    };
    assert_eq!(marked, &shadow.into_iter().collect::<Vec<_>>());
    assert_eq!(outlined(&practice.overlay.clone().expect("laid")), 1);
    // The Grey view: the copy and the render in grey, the shadow's colour no lightness finding.
    practise(&mut session, PracticeIntent::Grey(true));
    let practice = session.projection();
    assert!(practice.grey && practice.read_pictures.is_some());
    let overlay = practice.overlay.clone().expect("laid");
    let read_laid = |at: u8| {
        (overlay.iter()).any(
            |mark| matches!(mark.drawn, Figure::Picture { source: Source::Read(k), .. } if k == at),
        )
    };
    assert!(read_laid(0) && read_laid(1));
    assert_eq!(outlined(&overlay), 0);
    // A stroke after Compare takes it away.
    practise(&mut session, PracticeIntent::Fill(Color::WHITE));
    assert!(session.projection().compared.is_none());
}
