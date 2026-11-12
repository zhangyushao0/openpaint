//! The practice pens (S100), after V大's brushes: Big, Small and value masses' greys are the hard
//! tip, a round hard edge at full opacity whose pressure sets only its size, a little; Soft is the
//! airbrush as it was; Sketch and Draw the pencil and ink pen as they were. No pen is the
//! toolbox's brush, so no edit of the user's reaches it (the hard tip is Hard Paint's file, S104:
//! `hard_paint.rs`). Each colour kind offers only what its work needs: a value scale none (its
//! cells are filled). That a light stroke lays the colour picked exactly on the canvas is the
//! runtime's GPU test.

use std::sync::Arc;

use openpaint_app::{
    BrushKey, BrushLibrary, Contact, ContactPhase, Device, InputSample, Intent, Session,
    StrokeIntent, Title, Toolbox, Viewport,
};
use openpaint_core::{Artwork, CanvasSize, Color, Deposit, StrokeStyle};
use openpaint_practice::{
    Arrangement, Exercise, Given, Kind, LineTier, Pack, Picked, Planned, Purpose, ScaleTone, Sheet,
    SkillLevel,
};
use openpaint_practice_app::{Pen, PracticeIntent, PracticeLog, PracticePens, PracticeProjection};

const VIEWPORT: Viewport = Viewport {
    origin: [0.0, 84.0],
    size: [2880.0, 1716.0],
    pixels_per_point: 1.75,
};
const NOW: u64 = 1_792_000_000;
const HARD: [Pen; 5] = [Pen::Big, Pen::Small, Pen::Dark, Pen::Middle, Pen::Light];

fn pens() -> PracticePens {
    PracticePens::built_in().expect("pens")
}

/// The built-in toolbox file `key`'s recipe.
fn built_in(key: &str) -> StrokeStyle {
    let library = BrushLibrary::built_in().expect("built-in brushes");
    let brush = library.file(&BrushKey::new(key)).expect("a built-in");
    StrokeStyle::new(Arc::clone(brush.recipe()), 10.0, 1.0, Color::BLACK).expect("style")
}

#[test]
fn big_small_and_the_greys_lay_full_opacity_at_any_pressure_speed_and_tilt() {
    let pens = pens();
    for pen in HARD {
        let style = pens.style(pen);
        let recipe = style.recipe();
        assert_eq!(style.opacity(), 1.0, "{pen:?}");
        assert!(
            recipe.softness() == 0.0 && recipe.grain().strength() == 0.0,
            "{pen:?}: hard, no grain"
        );
        assert!(matches!(recipe.deposit(), Deposit::Cover), "{pen:?}");
        assert!(!recipe.shape().is_stamped(), "{pen:?}: round, no texture");
        for step in 0..=20 {
            let pressure = step as f32 / 20.0;
            for (speed, tilt) in [(0.0, 0.0), (3.0, 0.6), (12.0, 1.3)] {
                let opacity = recipe.opacity().at(pressure, speed, tilt);
                assert_eq!(opacity, 1.0, "{pen:?} at {pressure}, {speed}, {tilt}");
                // Pressure sets its size, a little; speed and tilt nothing.
                let size = recipe.size().at(pressure, speed, tilt);
                assert_eq!(size, recipe.size().at(pressure, 0.0, 0.0), "{pen:?}");
                assert!((0.6..=1.0).contains(&size), "{pen:?}: {size} at {pressure}");
            }
        }
    }
    let sizes = [Pen::Big, Pen::Small, Pen::Dark].map(|pen| pens.style(pen).size());
    assert_eq!(
        sizes,
        [64.0, 20.0, 24.0],
        "Big and Small as before, the greys a pen's width"
    );
}

#[test]
fn soft_is_the_airbrush_and_sketch_and_draw_the_pencil_and_ink_pen_as_they_were() {
    let pens = pens();
    // The same recipes as the toolbox's files of 1.17.0, so strokes keep drawing as they did.
    for (pen, key) in [
        (Pen::Soft, "soft-brush"),
        (Pen::Sketch, "pencil"),
        (Pen::Draw, "ink-pen"),
        (Pen::Correct, "ink-pen"),
        (Pen::Main, "ink-pen"),
    ] {
        assert_eq!(pens.style(pen).recipe(), built_in(key).recipe(), "{pen:?}");
    }
    let soft = pens.style(Pen::Soft);
    assert_eq!((soft.size(), soft.opacity()), (100.0, 0.8));
    assert_ne!(
        pens.style(Pen::Big).recipe(),
        built_in("opaque-paint").recipe()
    );
    assert_ne!(
        pens.style(Pen::Big).recipe(),
        built_in("hard-round").recipe()
    );
}

/// A toolbox whose every brush the user edited: each built-in's file with its pressure curves
/// turned (a recipe no practice pen has).
fn edited_toolbox() -> Toolbox {
    let edited = |name: &str| {
        format!(
            "name = \"{name}\"\n[size]\ndefault = 30\nmin = 1\nmax = 300\n\
             pressure = [[0.0, 0.1], [1.0, 0.5]]\n[opacity]\ndefault = 0.5\n\
             pressure = [[0.0, 0.2], [1.0, 0.4]]\n"
        )
    };
    let keys = [
        "pencil",
        "ink-pen",
        "soft-brush",
        "opaque-paint",
        "hard-round",
        "hard-paint",
    ];
    let texts: Vec<(String, String)> = (keys.iter())
        .map(|key| ((*key).to_owned(), edited(key)))
        .collect();
    let files: Vec<(&str, &str)> = (texts.iter())
        .map(|(key, text)| (key.as_str(), text.as_str()))
        .collect();
    Toolbox::new(BrushLibrary::from_files(&files, "hard-paint").expect("edited brushes"))
}

/// A sheet of `exercises` (and `packs`) opened with `toolbox`'s brush, its How closed.
fn opened(record: Sheet, packs: &[Pack], toolbox: &Toolbox) -> Session {
    let record = record.framed(packs);
    let [width, height] = openpaint_practice::Layout::of_sheet(&record).size;
    let sheet = Artwork::new(CanvasSize::new(width, height).expect("size"));
    let log = PracticeLog::new(Vec::new(), Vec::new(), packs.to_vec());
    let printed = Given::of_sheet(&record, packs);
    let opening = log.practising(&record, &printed, ([width, height], &Title::untitled()));
    let brush = toolbox.style().expect("style");
    let mut session =
        openpaint_practice_app::open_sheet(sheet, (brush, pens()), opening, &VIEWPORT);
    practise(&mut session, PracticeIntent::Instructions(false));
    session
}

fn session_of(kinds: &[Kind], toolbox: &Toolbox) -> Session {
    let exercises = (kinds.iter())
        .map(|kind| (Exercise::generated(*kind, 0x5EED_0100), SkillLevel::First))
        .collect();
    let picked = Picked::Full {
        stage: 1,
        warm_up: 0,
    };
    let planned = Planned { picked, exercises };
    opened(Sheet::begun(1, (Purpose::Day, 0), planned), &[], toolbox)
}

fn practise(session: &mut Session, intent: PracticeIntent) {
    openpaint_practice_app::practise(session, intent, NOW).expect("practised");
}

/// A pen stroke through `points` (canvas pixels) at `pressure`.
fn draw(session: &mut Session, points: &[[f32; 2]], pressure: f32) {
    let sample = |index: usize| InputSample {
        position: points[index],
        pressure,
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
            id: 9,
            device: Device::Pen,
            phase,
            samples,
        };
        session
            .dispatch(Intent::Stroke(StrokeIntent::Contact(contact)))
            .expect("dispatch");
    }
}

/// The last stroke's style on the sheet, if any.
fn last_stroke(session: &Session) -> Option<StrokeStyle> {
    let artwork = session.artwork();
    let strokes = artwork
        .layers()
        .iter()
        .flat_map(|layer| layer.strokes().iter());
    strokes.last().map(|stroke| stroke.style().clone())
}

/// The shown exercise's copy frame's middle, canvas pixels.
fn middle(session: &Session) -> [f32; 2] {
    let practice = PracticeProjection::of(session).expect("a sheet");
    let pair = practice.sheet.pairs[practice.current.expect("one shows")];
    pair.copy.to_canvas([0.5, 0.5]).map(|v| v as f32)
}

#[test]
fn every_practice_pen_is_independent_of_the_toolbox_s_edits() {
    let toolbox = edited_toolbox();
    let kinds = [Kind::Areas(Arrangement::Strata), Kind::Line(LineTier::Arc)];
    let mut session = session_of(&kinds, &toolbox);
    let picked = Color::rgb(0.8, 0.3, 0.1).expect("colour");
    let brush = toolbox.style().expect("style");
    let edited = StrokeStyle::new(Arc::clone(brush.recipe()), 30.0, 0.5, picked).expect("style");
    session.set_brush(edited.clone());
    let at = middle(&session);
    for pen in [Pen::Big, Pen::Small, Pen::Soft] {
        practise(&mut session, PracticeIntent::Pen(pen));
        draw(&mut session, &[at, [at[0] + 40.0, at[1]]], 0.02);
        let stroke = last_stroke(&session).expect("painted");
        let own = pens().style(pen).clone();
        assert_eq!(stroke.recipe(), own.recipe(), "{pen:?}: its own recipe");
        assert_ne!(
            stroke.recipe(),
            edited.recipe(),
            "{pen:?}: never the toolbox's"
        );
        let kept = (stroke.size(), stroke.opacity(), stroke.color());
        assert_eq!(kept, (own.size(), own.opacity(), picked), "{pen:?}");
    }
    practise(&mut session, PracticeIntent::Next(VIEWPORT));
    let at = middle(&session);
    for pen in [Pen::Sketch, Pen::Draw] {
        practise(&mut session, PracticeIntent::Pen(pen));
        draw(&mut session, &[at, [at[0] + 40.0, at[1] + 10.0]], 0.5);
        assert_eq!(last_stroke(&session).as_ref(), Some(pens().style(pen)));
    }
}

#[test]
fn a_value_scale_is_filled_its_pen_only_taking_a_cell_in_hand() {
    let mut session = session_of(
        &[Kind::Scale(ScaleTone::Grey, 5)],
        &Toolbox::new(BrushLibrary::built_in().expect("built-in brushes")),
    );
    let pen = |session: &Session| PracticeProjection::of(session).and_then(|p| p.pen);
    assert_eq!(pen(&session), None, "no pen offered");
    for refused in [Pen::Big, Pen::Small, Pen::Soft, Pen::Draw] {
        assert!(
            openpaint_practice_app::practise(&mut session, PracticeIntent::Pen(refused), NOW)
                .is_err()
        );
    }
    let cell = |session: &Session| PracticeProjection::of(session).and_then(|p| p.cell);
    assert_eq!(cell(&session), Some(1));
    // The middle cell touched: taken in hand, nothing laid, no notice.
    let at = middle(&session);
    draw(&mut session, &[at, [at[0], at[1] + 6.0]], 0.5);
    assert_eq!(cell(&session), Some(2));
    assert!(last_stroke(&session).is_none(), "nothing painted");
    assert_eq!(
        PracticeProjection::of(&session).and_then(|p| p.notice),
        None
    );
    // With all tools the toolbox's brush paints the cell it lands in.
    practise(&mut session, PracticeIntent::AllTools(true));
    draw(&mut session, &[at, [at[0], at[1] + 6.0]], 0.5);
    assert!(last_stroke(&session).is_some(), "the brush paints");
}
