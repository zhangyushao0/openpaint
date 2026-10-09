//! The brush editor's ranges (S46): every control stops where the engine still draws the
//! recipe, so no change is refused; choices that need others (stamps, Builds up, the paper)
//! follow them; a stored brush the engine refuses falls back to its built-in.

use openpaint_app::{
    Axis, BrushChange, BrushIntent, BrushKey, BrushLibrary, BrushParts, Dynamic, EditorState, Knob,
    StoredBrush, Toolbox,
};
use openpaint_core::{BrushRecipe, Rotation, Stamp, Tip};

const KNOBS: [Knob; 16] = [
    Knob::Spacing,
    Knob::TaperStart,
    Knob::TaperEnd,
    Knob::TaperTip,
    Knob::Aspect,
    Knob::Angle,
    Knob::RotationJitter,
    Knob::SizeJitter,
    Knob::Scatter,
    Knob::Softness,
    Knob::GrainStrength,
    Knob::GrainScale,
    Knob::Flow,
    Knob::MaxDensity,
    Knob::SizeMin,
    Knob::SizeMax,
];

fn on(key: &str) -> Toolbox {
    let mut toolbox = Toolbox::new(BrushLibrary::built_in().expect("built-in brushes"));
    toolbox.dispatch(&BrushIntent::Choose(BrushKey::new(key)));
    toolbox
}

fn change(toolbox: &mut Toolbox, change: BrushChange) {
    toolbox.dispatch(&BrushIntent::Edit {
        change,
        settled: true,
    });
}

fn value(toolbox: &Toolbox, knob: Knob) -> f32 {
    toolbox.projection().edited.value(knob)
}

#[test]
fn every_control_reaches_both_ends_of_its_range_on_every_built_in() {
    let library = BrushLibrary::built_in().expect("built-in brushes");
    for brush in library.brushes() {
        for knob in KNOBS {
            let mut toolbox = on(brush.key().as_str());
            let edited = toolbox.projection().edited;
            let Some(range) = edited.range(knob) else {
                continue;
            };
            let now = edited.value(knob);
            assert!(
                (range.min..=range.max).contains(&now),
                "{} {knob:?}",
                brush.name()
            );
            for position in [1.0, 0.5, 0.0] {
                let wanted = range.at(position);
                change(&mut toolbox, BrushChange::Set(knob, wanted));
                let got = value(&toolbox, knob);
                let continuous = knob == Knob::Spacing && got == 0.0;
                assert!(
                    continuous || (got - wanted).abs() <= 1e-4 * wanted.abs().max(1.0),
                    "{} {knob:?} at {position}: {got} for {wanted}",
                    brush.name()
                );
                let style = toolbox.style().expect("a style the engine draws");
                assert!(BrushRecipe::from_parts(&style.recipe().to_parts()).is_ok());
            }
        }
    }
}

#[test]
fn stamps_never_exceed_what_a_pixel_takes() {
    let mut speckle = on("speckle");
    // Scattered 3 radii, sized 1 +- 0.5: the stamps fit from about 0.125 diameters apart.
    change(&mut speckle, BrushChange::Set(Knob::Spacing, 0.07));
    let least = value(&speckle, Knob::Spacing);
    assert!(least > 0.12 && least < 0.13, "held at the least: {least}");
    let scatter = speckle
        .projection()
        .edited
        .range(Knob::Scatter)
        .expect("scatter");
    assert!(
        scatter.max < 3.01,
        "no more scatter fits there: {}",
        scatter.max
    );
    change(&mut speckle, BrushChange::Set(Knob::Scatter, 4.0));
    assert!(value(&speckle, Knob::Scatter) <= 3.01);
    let limit = speckle
        .projection()
        .edited
        .curve_limit(Dynamic::Size, Axis::Tilt);
    assert!(
        limit < 1.01,
        "a size widening with tilt would not fit: {limit}"
    );
    // Below half the least: continuous; a spacing brings the stamps back as they were.
    change(&mut speckle, BrushChange::Set(Knob::Spacing, 0.03));
    let continuous = speckle.projection().edited;
    assert_eq!(
        (continuous.stamp(), continuous.range(Knob::Scatter)),
        (None, None)
    );
    change(&mut speckle, BrushChange::Set(Knob::Spacing, 0.5));
    assert_eq!(
        value(&speckle, Knob::Scatter),
        3.0,
        "kept while it was continuous"
    );
    assert_eq!(value(&speckle, Knob::SizeJitter), 0.5);
}

#[test]
fn a_shape_lays_stamps_and_builds_up_and_the_paper_follow_their_rules() {
    let mut ink = on("ink-pen");
    let edited = ink.projection().edited;
    assert_eq!((edited.stamp(), edited.range(Knob::Aspect)), (None, None));
    change(&mut ink, BrushChange::Stamp(Stamp::Square));
    let square = ink.projection().edited;
    assert_eq!(
        (square.stamp(), square.rotation()),
        (Some(Stamp::Square), Some(Rotation::Fixed))
    );
    assert!(
        (square.value(Knob::Spacing) - 0.1).abs() < 1e-6,
        "stamps a tenth apart"
    );
    assert!(square.range(Knob::Aspect).is_some());
    change(&mut ink, BrushChange::Stamp(Stamp::Disc));
    assert_eq!(
        ink.projection().edited.range(Knob::Aspect),
        None,
        "a disc is round"
    );
    change(&mut ink, BrushChange::BuildsUp(true));
    let builds = ink.projection().edited;
    assert_eq!(
        (builds.tip(), builds.range(Knob::Softness)),
        (Some(Tip::Flat), None)
    );
    assert!(!builds.offers_paper(), "stamps take no paper");
    change(&mut ink, BrushChange::Set(Knob::Spacing, 0.0));
    assert!(ink.projection().edited.offers_paper());
    change(&mut ink, BrushChange::Paper(true));
    let paper = ink.projection().edited;
    assert!(paper.paper() && !paper.offers_bell() && !paper.offers_stamps());
    assert_eq!(
        paper.range(Knob::Spacing),
        None,
        "the paper keeps it continuous"
    );
    change(&mut ink, BrushChange::Tip(Tip::Bell));
    assert_eq!(
        ink.projection().edited.tip(),
        Some(Tip::Flat),
        "no bell on paper"
    );
    change(&mut ink, BrushChange::BuildsUp(false));
    let covers = ink.projection().edited;
    assert_eq!((covers.tip(), covers.paper()), (None, false));
    let soft = on("soft-brush").projection().edited;
    assert_eq!(
        soft.range(Knob::Softness),
        None,
        "a building tip has its own edge"
    );
}

#[test]
fn sizes_curves_and_their_resets_stay_in_range() {
    let mut ink = on("ink-pen");
    ink.dispatch(&BrushIntent::Size {
        position: 0.0,
        settled: true,
    });
    change(&mut ink, BrushChange::Set(Knob::SizeMin, 50.0));
    assert_eq!(ink.projection().size, 50.0, "the tool's size moved inside");
    change(&mut ink, BrushChange::Set(Knob::SizeMax, 30.0));
    assert_eq!(value(&ink, Knob::SizeMax), 50.0, "never below the minimum");
    let curve = BrushChange::Curve {
        dynamic: Dynamic::Size,
        axis: Axis::Tilt,
        points: vec![[0.0, 1.0], [70.0, 9.0], [200.0, 2.0]],
    };
    change(&mut ink, curve);
    let edited = ink.projection().edited;
    assert_eq!(
        edited.curve(Dynamic::Size, Axis::Tilt),
        [[0.0, 1.0], [70.0, 4.0], [90.0, 2.0]]
    );
    assert!(edited.curve_changed(Dynamic::Size, Axis::Tilt));
    change(
        &mut ink,
        BrushChange::ResetCurve {
            dynamic: Dynamic::Size,
            axis: Axis::Tilt,
        },
    );
    let reset = ink.projection().edited;
    assert_eq!(reset.curve(Dynamic::Size, Axis::Tilt), [[0.0, 1.0]]);
    assert!(!reset.curve_changed(Dynamic::Size, Axis::Tilt));
    let crossed = BrushChange::Curve {
        dynamic: Dynamic::Opacity,
        axis: Axis::Pressure,
        points: vec![[0.5, 1.0], [0.25, 0.5]],
    };
    change(&mut ink, crossed);
    assert_eq!(
        ink.projection()
            .edited
            .curve(Dynamic::Opacity, Axis::Pressure),
        [[0.0, 1.0]]
    );
}

#[test]
fn a_stored_brush_the_engine_refuses_falls_back_to_its_built_in() {
    let library = BrushLibrary::built_in().expect("built-in brushes");
    let mut toolbox = Toolbox::new(library.clone());
    toolbox.dispatch(&BrushIntent::Choose(BrushKey::new("inker")));
    toolbox.dispatch(&BrushIntent::Duplicate);
    let mut state = toolbox.state();
    let broken = |entry: &mut StoredBrush| {
        let parts: &mut BrushParts = entry.brush.as_mut().expect("whole");
        parts.recipe.taper = [-1.0, 0.0, 0.0];
    };
    let at = state
        .library
        .iter()
        .position(|b| b.key == "copy-1")
        .expect("the copy");
    broken(&mut state.library[at]);
    let mut inker = state.library[at].clone();
    inker.key = "inker".into();
    if let Some(parts) = inker.brush.as_mut() {
        parts.source.clear();
        parts.name = "Inker".into();
    }
    state.library[at - 1] = inker;
    state.library.push(StoredBrush {
        key: "charcoal".into(),
        brush: None,
    });
    let mut reopened = Toolbox::new(library.clone());
    assert!(!reopened.restore(&state));
    assert_eq!(reopened.fell_back(), ["Inker", "Inker copy"]);
    let inker = reopened
        .library()
        .brush(&BrushKey::new("inker"))
        .expect("inker");
    assert_eq!(
        Some(inker),
        library.file(&BrushKey::new("inker")),
        "the built-in"
    );
    let copy = reopened
        .library()
        .brush(&BrushKey::new("copy-1"))
        .expect("the copy");
    assert_eq!((copy.name(), copy.recipe()), ("Inker copy", inker.recipe()));
    assert_eq!(
        reopened.library().brushes().len(),
        11,
        "the unknown key skipped"
    );
    let empty = EditorState {
        library: Vec::new(),
        ..state
    };
    let mut fresh = Toolbox::new(library);
    fresh.restore(&empty);
    assert_eq!(fresh.library().brushes().len(), 10, "empty: the built-ins");
}
