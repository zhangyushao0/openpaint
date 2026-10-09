//! Brushes: the built-in files parse into the six brushes by job, broken files are refused with
//! typed errors, the toolbox keeps each brush's size and opacity, and a stroke keeps the recipe
//! it was drawn with when a built-in brush changes later.

use openpaint_app::{
    BrushChoice, BrushIntent, BrushLibrary, BrushSetting, ButtonMap, Contact, ContactPhase, Device,
    EditorState, InputSample, Intent, MIN_OPACITY, RecipeError, Session, StreamLine, StrokeIntent,
    Tool, Toolbox,
};
use openpaint_core::{BrushRecipe, CanvasSize, Deposit, PaperTexture, Tip};

const PENCIL: &str = include_str!("../brushes/pencil.toml");

fn built_in() -> BrushLibrary {
    BrushLibrary::built_in().expect("built-in brushes")
}

fn parse(text: &str) -> Result<BrushLibrary, RecipeError> {
    BrushLibrary::from_files(&[("test", text)], "test")
}

fn field(name: &str) -> String {
    name.to_owned()
}

#[test]
fn the_built_in_brushes_are_listed_by_job() {
    let library = built_in();
    let names: Vec<_> = library.brushes().iter().map(|brush| brush.name()).collect();
    assert_eq!(
        names,
        [
            "Pencil",
            "Ink Pen",
            "Inker",
            "Monoline",
            "Hard Round",
            "Soft Brush",
            "Textured Round",
            "Flat",
            "Opaque Paint",
            "Speckle"
        ]
    );
    let [pencil, ink, _, _, _, soft, ..] = library.brushes() else {
        panic!("ten brushes");
    };
    assert_eq!(
        **ink.recipe(),
        BrushRecipe::plain(),
        "the round brush before recipes"
    );
    assert_eq!((ink.size().default, ink.opacity()), (12.0, 1.0));
    // draweave's tuned pencil: graphite building up on the sketch paper.
    let Deposit::Accumulate(graphite) = pencil.recipe().deposit() else {
        panic!("the pencil accumulates");
    };
    assert_eq!(
        (graphite.tip(), graphite.flow(), graphite.max_density()),
        (Tip::Flat, 3.1, 1.0)
    );
    let paper = graphite.paper().expect("on paper");
    assert_eq!(paper.texture(), PaperTexture::Sketch);
    assert_eq!(paper.contact().points(), [[0.0, 0.4], [1.0, 0.95]]);
    assert_eq!(
        [paper.scale(), paper.depth(), paper.dust()],
        [4.5, 0.3, 0.3]
    );
    assert_eq!(pencil.recipe().size().pressure().at(0.0), 0.3);
    let flow = |p: f32| graphite.flow() * pencil.recipe().opacity().at(p, 0.0, 0.0);
    assert!((flow(0.0) - 0.9).abs() < 1e-6 && (flow(1.0) - 3.1).abs() < 1e-6);
    assert!(!pencil.recipe().uses_speed(), "no speed");
    let Deposit::Accumulate(airbrush) = soft.recipe().deposit() else {
        panic!("the soft brush accumulates");
    };
    assert_eq!((airbrush.tip(), airbrush.paper()), (Tip::Bell, None));
    assert_eq!(
        *ink.recipe().deposit(),
        Deposit::Cover,
        "ink never darkens itself"
    );
    let toolbox = Toolbox::new(library.clone());
    assert_eq!(
        toolbox.projection().current,
        ink.key().clone(),
        "the default brush"
    );
    assert_eq!(toolbox.style().expect("style").size(), 12.0);
}

#[test]
fn broken_files_are_refused_with_typed_errors() {
    let cases = [
        ("name = ", RecipeError::Syntax { line: 1, column: 8 }),
        (
            "[size]\ndefault = 1\nmin = 1\nmax = 2\n[opacity]\ndefault = 1",
            RecipeError::Missing {
                field: field("name"),
            },
        ),
        (
            "name = 3",
            RecipeError::WrongType {
                field: field("name"),
                expected: "text",
            },
        ),
        (
            "name = \"A\"\ncolour = 1",
            RecipeError::Unknown {
                field: field("colour"),
            },
        ),
    ];
    for (text, expected) in cases {
        assert_eq!(parse(text).err(), Some(expected), "{text}");
    }
    let brush = |size: &str, extra: &str| {
        format!("name = \"A\"\n[size]\n{size}\n[opacity]\ndefault = 0.5\n{extra}")
    };
    let sizes = "default = 4\nmin = 1\nmax = 10";
    assert!(parse(&brush(sizes, "")).is_ok());
    let wrong = [
        (brush("default = 40\nmin = 1\nmax = 10", ""), "size"),
        (
            brush(sizes, "pressure = [[0.5, 0.2], [0.4, 1.0]]"),
            "opacity.pressure",
        ),
        (
            brush(sizes, "speed = [[0.0, 1.0], [99.0, 0.5]]"),
            "opacity.speed",
        ),
        (brush(sizes, "[grain]\nstrength = 2.0"), "grain.strength"),
        (brush(sizes, "[shape]\nsoftness = -1"), "shape.softness"),
    ];
    for (text, at) in wrong {
        let expected = RecipeError::OutOfRange { field: field(at) };
        assert_eq!(parse(&text).err(), Some(expected), "{text}");
    }
    let pairs = brush(sizes, "pressure = [1, 2]");
    assert!(matches!(
        parse(&pairs),
        Err(RecipeError::WrongType { field, .. }) if field == "opacity.pressure"
    ));
    let missing = BrushLibrary::from_files(&[("pencil", PENCIL)], "ink");
    assert!(matches!(missing, Err(RecipeError::Missing { .. })));
}

#[test]
fn broken_deposits_and_papers_are_refused_with_typed_errors() {
    let brush = |extra: &str| {
        format!(
            "name = \"A\"\n[size]\ndefault = 4\nmin = 1\nmax = 10\n[opacity]\ndefault = 0.5\n{extra}"
        )
    };
    let wrong = [
        ("[accumulate]\ntip = \"round\"\nflow = 1", "accumulate.tip"),
        ("[accumulate]\ntip = \"bell\"\nflow = 99", "accumulate.flow"),
        (
            "[shape]\nsoftness = 0.5\n[accumulate]\ntip = \"bell\"\nflow = 1",
            "shape.softness",
        ),
        (
            "[accumulate]\ntip = \"flat\"\nflow = 1\n[paper]\ntexture = \"sketch\"\n\
             contact = [[0.0, 0.5]]\nscale = 900\ndepth = 0.3\ndust = 0.3",
            "paper.scale",
        ),
    ];
    for (extra, at) in wrong {
        let expected = RecipeError::OutOfRange { field: field(at) };
        assert_eq!(parse(&brush(extra)).err(), Some(expected), "{extra}");
    }
    let paper_alone = brush(
        "[paper]\ntexture = \"sketch\"\ncontact = [[0.0, 0.5]]\nscale = 4\ndepth = 0.3\ndust = 0",
    );
    let expected = RecipeError::Missing {
        field: field("accumulate"),
    };
    assert_eq!(parse(&paper_alone).err(), Some(expected));
}

#[test]
fn every_brush_keeps_its_own_size_and_opacity() {
    let library = built_in();
    let [pencil, ink, ..] = library.brushes() else {
        panic!("the built-in brushes");
    };
    let mut toolbox = Toolbox::new(library.clone());
    toolbox.dispatch(&BrushIntent::Size {
        position: 1.0,
        settled: false,
    });
    assert_eq!(toolbox.save_request(), None, "a held slider is not stored");
    toolbox.dispatch(&BrushIntent::Size {
        position: 1.0,
        settled: true,
    });
    assert_eq!(toolbox.projection().size, ink.size().max);
    assert!(toolbox.save_request().is_some());
    assert_eq!(toolbox.save_request(), None, "handed out once");
    toolbox.dispatch(&BrushIntent::Choose(pencil.key().clone()));
    let shown = toolbox.projection();
    assert_eq!((shown.size, shown.opacity), (pencil.size().default, 1.0));
    toolbox.dispatch(&BrushIntent::Opacity {
        opacity: 0.0,
        settled: true,
    });
    assert_eq!(toolbox.projection().opacity, MIN_OPACITY);
    toolbox.dispatch(&BrushIntent::StreamLine {
        amount: 1.5,
        settled: true,
    });
    assert_eq!(toolbox.streamline().amount(), 1.0, "clamped");
    toolbox.dispatch(&BrushIntent::Choose(ink.key().clone()));
    assert_eq!(toolbox.projection().size, ink.size().max, "kept while away");
    assert_eq!(
        toolbox.streamline(),
        StreamLine::OFF,
        "every brush starts off"
    );
    toolbox.dispatch(&BrushIntent::Choose(pencil.key().clone()));
    assert_eq!(toolbox.projection().streamline.amount(), 1.0, "its own");
    toolbox.dispatch(&BrushIntent::Choose(ink.key().clone()));
    let state = toolbox.state();
    let mut reopened = Toolbox::new(library);
    assert!(reopened.restore(&state));
    assert_eq!(reopened.projection(), toolbox.projection());
    assert_eq!(reopened.state(), state);
}

#[test]
fn a_stored_state_that_does_not_fit_falls_back_to_defaults() {
    let library = built_in();
    let mut toolbox = Toolbox::new(library);
    let fresh = toolbox.projection();
    let paint = BrushChoice {
        brush: "charcoal".into(),
        brushes: vec![
            BrushSetting {
                key: "ink-pen".into(),
                size: 5000.0,
                opacity: 0.5,
                streamline: 0.0,
            },
            BrushSetting {
                key: "charcoal".into(),
                size: 4.0,
                opacity: 0.5,
                streamline: 0.0,
            },
            BrushSetting {
                key: "pencil".into(),
                size: 4.0,
                opacity: 0.5,
                streamline: 2.0,
            },
        ],
    };
    let state = EditorState {
        tool: Tool::Brush,
        erase: paint.clone(),
        smudge: paint.clone(),
        paint,
        color: [0.0, 2.0, 0.0],
        recent: vec![[0.5, 0.5, f32::NAN]],
        buttons: ButtonMap::default(),
        swatches: vec![[2.0, 0.0, 0.0]],
        library: Vec::new(),
        sets: Default::default(),
        recent_brushes: Vec::new(),
    };
    assert!(!toolbox.restore(&state));
    assert_eq!(toolbox.projection(), fresh);
    // An empty brush key is the default brush, not a misfit.
    let defaults = toolbox.state();
    let unset = EditorState {
        erase: BrushChoice {
            brush: String::new(),
            brushes: Vec::new(),
        },
        ..defaults.clone()
    };
    assert!(toolbox.restore(&unset));
    assert_eq!(toolbox.state(), defaults);
}

fn pen(phase: ContactPhase, x: f32, time_ms: i64) -> Intent {
    let sample = InputSample {
        position: [x, 40.0],
        pressure: 0.7,
        tilt: 0.0,
        azimuth: 0.0,
        time_ms,
    };
    Intent::Stroke(StrokeIntent::Contact(Contact {
        id: 1,
        device: Device::Pen,
        phase,
        samples: vec![sample],
    }))
}

fn draw(session: &mut Session, x: f32) {
    for (phase, dx, time) in [
        (ContactPhase::Down, 0.0, 0),
        (ContactPhase::Move, 20.0, 8),
        (ContactPhase::Up, 40.0, 16),
    ] {
        session.dispatch(pen(phase, x + dx, time)).expect("pen");
    }
}

#[test]
fn a_changed_built_in_brush_leaves_strokes_drawn_before_as_they_were() {
    let before = BrushLibrary::from_files(&[("pencil", PENCIL)], "pencil").expect("pencil");
    let old = Toolbox::new(before).style().expect("style");
    let mut session = Session::new(CanvasSize::new(200, 100).expect("size"), old.clone());
    draw(&mut session, 10.0);
    // A later version of the app ships a pencil with another pressure curve and flow.
    let changed = PENCIL
        .replace("[[0.0, 0.3], [1.0, 1.0]]", "[[0.0, 1.0]]")
        .replace("flow = 3.1", "flow = 2.0");
    let after = BrushLibrary::from_files(&[("pencil", &changed)], "pencil").expect("pencil");
    let new = Toolbox::new(after).style().expect("style");
    assert_ne!(new.recipe(), old.recipe());
    let mut reopened = Session::open(session.artwork().clone(), new.clone());
    draw(&mut reopened, 100.0);
    let strokes: Vec<_> = reopened.artwork().layers()[0]
        .strokes()
        .iter()
        .cloned()
        .collect();
    assert_eq!(
        strokes[0].style().recipe(),
        old.recipe(),
        "drawn before: unchanged"
    );
    assert_eq!(
        strokes[1].style().recipe(),
        new.recipe(),
        "drawn after: the new pencil"
    );
}
