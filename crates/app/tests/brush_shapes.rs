//! Shapes and scatter (S45): the four stamped built-ins after Soft Brush, `[shape]` in brush
//! files read into the recipe's shape stage with every key checked, and the seed a session gives
//! each stroke, which its mirrored copies keep.

use openpaint_app::{
    BrushLibrary, Contact, ContactPhase, Device, GuideIntent, InputSample, Intent, RecipeError,
    Session, StrokeIntent, Symmetry, Toolbox,
};
use openpaint_core::{CanvasSize, Deposit, Rotation, Shape, ShapeTexture, Stamp};

fn built_in() -> BrushLibrary {
    BrushLibrary::built_in().expect("built-in brushes")
}

fn parse(text: &str) -> Result<BrushLibrary, RecipeError> {
    BrushLibrary::from_files(&[("test", text)], "test")
}

fn brush(shape: &str) -> String {
    format!(
        "name = \"A\"\n[size]\ndefault = 4\nmin = 1\nmax = 10\n[opacity]\ndefault = 0.5\n\
         [shape]\n{shape}\n"
    )
}

fn shape_of(text: &str) -> Shape {
    *parse(text).expect("brush").brushes()[0].recipe().shape()
}

#[test]
fn the_four_stamped_brushes_follow_soft_brush() {
    let library = built_in();
    let [.., textured, flat, opaque, speckle] = library.brushes() else {
        panic!("ten brushes");
    };
    let shape = |brush: &openpaint_app::Brush| *brush.recipe().shape();
    let textured_shape = shape(textured);
    assert_eq!(
        (
            textured.name(),
            textured_shape.stamp(),
            textured_shape.spacing()
        ),
        ("Textured Round", Stamp::Texture(ShapeTexture::Chalk), 0.1)
    );
    assert_eq!(textured_shape.rotation_jitter(), 15.0);
    assert!(textured.recipe().grain().strength() > 0.0, "grain inside");
    assert_eq!(textured.recipe().deposit(), &Deposit::Cover);
    let flat_shape = shape(flat);
    assert_eq!(
        (flat.name(), flat_shape.stamp(), flat_shape.aspect()),
        ("Flat", Stamp::Square, 0.25)
    );
    assert_eq!(flat_shape.rotation(), Rotation::Direction);
    assert_eq!(flat.recipe().softness(), 0.0, "hard");
    let opaque_shape = shape(opaque);
    assert_eq!(
        (opaque.name(), opaque_shape.stamp(), opaque_shape.aspect()),
        (
            "Opaque Paint",
            Stamp::Texture(ShapeTexture::RoughSquare),
            0.5
        )
    );
    assert_eq!(
        (opaque_shape.spacing(), opaque_shape.rotation()),
        (0.08, Rotation::Direction)
    );
    let speckle_shape = shape(speckle);
    assert_eq!(
        (
            speckle.name(),
            speckle_shape.stamp(),
            speckle_shape.spacing()
        ),
        ("Speckle", Stamp::Disc, 0.5)
    );
    assert_eq!(
        (speckle_shape.scatter(), speckle_shape.size_jitter()),
        (3.0, 0.5)
    );
    assert!(matches!(speckle.recipe().deposit(), Deposit::Accumulate(_)));
    // Every brush before them is the continuous capsule, as it was.
    for brush in &library.brushes()[..6] {
        assert_eq!(
            *brush.recipe().shape(),
            Shape::CONTINUOUS,
            "{}",
            brush.name()
        );
    }
}

#[test]
fn every_shape_key_reaches_the_recipe() {
    let shape = shape_of(&brush(
        "stamp = \"ellipse\"\naspect = 0.4\nspacing = 0.2\nscatter = 1.5\nrotation = \
         \"azimuth\"\nangle = -45\nrotation_jitter = 30\nsize_jitter = 0.25",
    ));
    let expected = Shape::stamped(
        (Stamp::Ellipse, 0.4),
        0.2,
        1.5,
        (Rotation::Azimuth, -45.0, 30.0),
        0.25,
    );
    assert_eq!(Ok(shape), expected);
    for (name, texture) in [
        ("chalk", ShapeTexture::Chalk),
        ("grainy", ShapeTexture::Grainy),
        ("splatter", ShapeTexture::Splatter),
        ("rough-square", ShapeTexture::RoughSquare),
    ] {
        let text = brush(&format!("stamp = \"{name}\"\nspacing = 0.3"));
        assert_eq!(shape_of(&text).stamp(), Stamp::Texture(texture));
    }
    assert_eq!(shape_of(&brush("softness = 0.5")), Shape::CONTINUOUS);
    assert_eq!(shape_of(&brush("spacing = 0")), Shape::CONTINUOUS);
}

#[test]
fn shape_keys_are_checked_in_brush_files() {
    let range = |at: &str| {
        Some(RecipeError::OutOfRange {
            field: at.to_owned(),
        })
    };
    let wrong = [
        ("stamp = \"star\"\nspacing = 0.2", range("shape.stamp")),
        ("spacing = 0.01", range("shape.spacing")),
        ("spacing = 5", range("shape.spacing")),
        ("spacing = 0.2\naspect = 0.5", range("shape.aspect")),
        (
            "stamp = \"square\"\nspacing = 0.2\naspect = 0",
            range("shape.aspect"),
        ),
        ("spacing = 0.2\nscatter = 5", range("shape.scatter")),
        ("spacing = 0.2\nrotation = \"up\"", range("shape.rotation")),
        ("spacing = 0.2\nangle = 270", range("shape.angle")),
        (
            "spacing = 0.2\nrotation_jitter = -5",
            range("shape.rotation_jitter"),
        ),
        ("spacing = 0.2\nsize_jitter = 2", range("shape.size_jitter")),
        ("spacing = 0\nscatter = 1", range("shape.spacing")),
        // Too many stamps within reach: a scatter of 3 radii at the spacing floor.
        ("spacing = 0.05\nscatter = 3", range("shape.spacing")),
        (
            "stamp = \"square\"",
            Some(RecipeError::Missing {
                field: "shape.spacing".to_owned(),
            }),
        ),
    ];
    for (shape, error) in wrong {
        assert_eq!(parse(&brush(shape)).err(), error, "{shape}");
    }
    let unknown = parse(&brush("spacing = 0.2\njitter = 1")).err();
    assert!(matches!(unknown, Some(RecipeError::Unknown { field }) if field == "shape.jitter"));
    let papered = format!(
        "{}[accumulate]\ntip = \"flat\"\nflow = 1\n[paper]\ntexture = \"sketch\"\ncontact = \
         [[0.0, 0.5]]\nscale = 4\ndepth = 0.3\ndust = 0.3\n",
        brush("spacing = 0.2")
    );
    assert_eq!(
        parse(&papered).err(),
        range("paper"),
        "stamps take no paper"
    );
}

fn pen(session: &mut Session, phase: ContactPhase, x: f32, time_ms: i64) {
    let sample = InputSample {
        position: [x, 60.0 + 0.1 * x],
        pressure: 0.5,
        tilt: 0.0,
        azimuth: 0.0,
        time_ms,
    };
    let contact = Contact {
        id: 3,
        device: Device::Pen,
        phase,
        samples: vec![sample],
    };
    session
        .dispatch(Intent::Stroke(StrokeIntent::Contact(contact)))
        .expect("contact");
}

#[test]
fn a_stroke_takes_the_sessions_seed_and_its_mirrored_copies_keep_it() {
    let brush = Toolbox::new(built_in()).style().expect("style");
    let mut session = Session::new(CanvasSize::new(400, 300).expect("size"), brush);
    let draw = |session: &mut Session| {
        pen(session, ContactPhase::Down, 220.0, 0);
        pen(session, ContactPhase::Move, 260.0, 8);
        pen(session, ContactPhase::Up, 300.0, 16);
    };
    draw(&mut session);
    session.set_seed(0xABCD_0123);
    let guide = |intent| Intent::Stroke(StrokeIntent::Guide(intent));
    session
        .dispatch(guide(GuideIntent::Symmetry(Symmetry::Vertical)))
        .expect("guide");
    draw(&mut session);
    let artwork = session.artwork();
    let layer = artwork.layer(artwork.active()).expect("layer");
    let seeds: Vec<u32> = layer.strokes().iter().map(|s| s.style().seed()).collect();
    assert_eq!(
        seeds,
        [0, 0xABCD_0123, 0xABCD_0123],
        "0 at first; the copy keeps it"
    );
}
