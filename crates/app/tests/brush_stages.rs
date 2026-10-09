//! Brush stages from S44: the built-in Pencil responds to tilt only when laid over, the Inker
//! is the Ink Pen tapering at both ends, the Monoline keeps one width with StreamLine on at
//! first, Hard Round follows pressure in size and opacity; brush files' tilt, taper and
//! StreamLine are checked with typed errors.

use openpaint_app::{BrushIntent, BrushLibrary, RecipeError, Toolbox};
use openpaint_core::{BrushRecipe, Deposit, Dynamics, Taper};

fn built_in() -> BrushLibrary {
    BrushLibrary::built_in().expect("built-in brushes")
}

fn parse(text: &str) -> Result<BrushLibrary, RecipeError> {
    BrushLibrary::from_files(&[("test", text)], "test")
}

fn out_of_range(at: &str) -> RecipeError {
    RecipeError::OutOfRange {
        field: at.to_owned(),
    }
}

#[test]
fn the_pencil_tilts_the_inker_tapers_the_monoline_keeps_one_width() {
    let library = built_in();
    let [pencil, _, inker, monoline, hard, ..] = library.brushes() else {
        panic!("ten brushes");
    };
    // Laid on its side the pencil draws wider and lighter; held as for writing, as before.
    let tilt = |degrees: f32| {
        let at = |dynamics: &Dynamics| dynamics.tilt().at(degrees);
        [at(pencil.recipe().size()), at(pencil.recipe().opacity())]
    };
    assert_eq!(tilt(45.0), [1.0, 1.0]);
    assert_eq!(tilt(60.0), [1.0, 1.0]);
    let [wider, lighter] = tilt(67.5);
    assert!((wider - 2.0).abs() < 1e-6 && (lighter - 0.625).abs() < 1e-6);
    assert_eq!(pencil.recipe().taper(), Taper::NONE);
    // The Inker is the Ink Pen tapering at both ends.
    let taper = Taper::new(2.0, 3.0, 0.1).expect("taper");
    assert_eq!(**inker.recipe(), BrushRecipe::plain().with_taper(taper));
    assert_eq!((inker.size().default, inker.streamline()), (12.0, 0.0));
    // The Monoline: one width at any pressure, StreamLine 60 % at first.
    for p in [0.0, 0.2, 1.0] {
        assert_eq!(monoline.recipe().size().at(p, 9.0, 1.2), 1.0);
        assert_eq!(monoline.recipe().opacity().at(p, 9.0, 1.2), 1.0);
    }
    assert_eq!(
        (monoline.recipe().softness(), monoline.streamline()),
        (0.0, 0.6)
    );
    // Hard Round: pressure sets size and opacity, a hard covering edge.
    let at = |p: f32| [hard.recipe().size(), hard.recipe().opacity()].map(|d| d.at(p, 0.0, 0.0));
    assert_eq!((at(0.0), at(1.0)), ([0.25, 0.1], [1.0, 1.0]));
    assert_eq!(
        (hard.recipe().softness(), hard.recipe().deposit()),
        (0.0, &Deposit::Cover)
    );
    // The Monoline's StreamLine is every tool's at first.
    let mut toolbox = Toolbox::new(library.clone());
    toolbox.dispatch(&BrushIntent::Choose(monoline.key().clone()));
    assert_eq!(toolbox.streamline().amount(), 0.6);
}

#[test]
fn tilt_taper_and_streamline_are_checked_in_brush_files() {
    let brush = |size: &str, extra: &str| {
        format!(
            "name = \"A\"\n{extra}\n[size]\ndefault = 4\nmin = 1\nmax = 10\n{size}\n[opacity]\n\
             default = 0.5\n"
        )
    };
    let tilted = brush("tilt = [[40.0, 1.0], [80.0, 4.0]]", "");
    assert!(parse(&tilted).is_ok(), "a size may grow up to four times");
    let wrong = [
        (brush("tilt = [[40.0, 1.0], [95.0, 2.0]]", ""), "size.tilt"),
        (brush("tilt = [[40.0, 1.0], [80.0, 5.0]]", ""), "size.tilt"),
        (brush("", "streamline = 1.5"), "streamline"),
        (brush("", "[taper]\nstart = -1"), "taper.start"),
        (brush("", "[taper]\nstart = 1\nend = 99"), "taper.end"),
        (brush("", "[taper]\ntip = 2"), "taper.tip"),
    ];
    for (text, at) in wrong {
        assert_eq!(parse(&text).err(), Some(out_of_range(at)), "{text}");
    }
    let lighter = format!("{}tilt = [[0.0, 1.5]]", brush("", ""));
    assert_eq!(
        parse(&lighter).err(),
        Some(out_of_range("opacity.tilt")),
        "an opacity never rises"
    );
    let unknown = brush("", "[taper]\nlength = 2");
    assert!(
        matches!(parse(&unknown), Err(RecipeError::Unknown { field }) if field == "taper.length")
    );
}
