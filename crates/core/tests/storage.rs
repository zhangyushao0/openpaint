//! `from_storage`: stored data round-trips, and every broken invariant is rejected with a type.

use openpaint_core::{
    Artwork, ArtworkCommand, ArtworkParts, BrushRecipe, CanvasPoint, CanvasSize, Color, CoreError,
    LayerCommand, LayerId, LayerName, MAX_ID, MAX_LAYERS, StrokeSample, StrokeStyle,
};

fn sample(x: f32, time_ms: u32) -> StrokeSample {
    let position = CanvasPoint::new(x, 3.0).unwrap();
    StrokeSample::new(position, 0.75, 0.1, -0.5, time_ms).unwrap()
}

fn layer(command: LayerCommand) -> ArtworkCommand {
    ArtworkCommand::Layer(command)
}

/// Two layers with one stroke each; the bottom one renamed, hidden and half transparent.
fn painted() -> Artwork {
    let recipe = std::sync::Arc::new(BrushRecipe::plain());
    let style = StrokeStyle::new(recipe, 5.0, 0.5, Color::rgb(0.2, 0.4, 0.6).unwrap()).unwrap();
    let draw = |artwork: &Artwork| {
        let mut session = artwork.begin_stroke(style.clone(), sample(0.0, 0)).unwrap();
        session.extend(&[sample(1.0, 5), sample(2.0, 9)]).unwrap();
        session.finish().artwork
    };
    let bottom = LayerId::new(1);
    let mut artwork = draw(&Artwork::new(CanvasSize::new(320, 200).unwrap()));
    for command in [
        LayerCommand::Rename {
            layer: bottom,
            name: LayerName::new("Sketch").unwrap(),
        },
        LayerCommand::SetVisible {
            layer: bottom,
            visible: false,
        },
        LayerCommand::SetOpacity {
            layer: bottom,
            opacity: 0.5,
        },
        LayerCommand::Create,
    ] {
        artwork = artwork.apply(layer(command)).unwrap().artwork;
    }
    draw(&artwork)
}

fn rejects(change: impl FnOnce(&mut ArtworkParts), expected: CoreError) {
    let mut parts = painted().to_parts();
    change(&mut parts);
    assert_eq!(Artwork::from_storage(parts), Err(expected));
}

fn invalid(what: &'static str) -> CoreError {
    CoreError::InvalidValue { what }
}

#[test]
fn round_trip_is_equal() {
    let artwork = painted();
    let parts = artwork.to_parts();
    assert_eq!(parts.layers.len(), 2);
    assert_eq!(parts.layers[0].name, "Sketch");
    assert_eq!(parts.layers[1].strokes[0].id, 2);
    let loaded = Artwork::from_storage(parts.clone()).unwrap();
    assert_eq!(loaded, artwork);
    assert!(!loaded.same_version(&artwork));
    assert_eq!(loaded.to_parts(), parts);
}

#[test]
fn loaded_counters_continue() {
    let loaded = Artwork::from_storage(painted().to_parts()).unwrap();
    let next = loaded.apply(layer(LayerCommand::Create)).unwrap().artwork;
    assert_eq!(next.active(), LayerId::new(3));
}

#[test]
fn rejects_bad_canvas_and_layer_counts() {
    rejects(|p| p.width = 0, invalid("canvas size"));
    rejects(|p| p.layers.clear(), CoreError::NoLayers);
    rejects(
        |p| {
            let extra = p.layers[0].clone();
            p.layers.resize(MAX_LAYERS + 1, extra);
        },
        CoreError::LayerLimit { max: MAX_LAYERS },
    );
}

#[test]
fn rejects_bad_ids() {
    rejects(|p| p.next_layer_id = 0, invalid("id counter"));
    rejects(|p| p.next_stroke_id = MAX_ID + 1, invalid("id counter"));
    rejects(
        |p| p.layers[1].id = 1,
        CoreError::DuplicateId {
            what: "layer id",
            id: 1,
        },
    );
    rejects(
        |p| p.layers[0].id = 0,
        CoreError::IdOutOfRange {
            what: "layer id",
            id: 0,
        },
    );
    rejects(
        |p| p.next_layer_id = 2,
        CoreError::IdOutOfRange {
            what: "layer id",
            id: 2,
        },
    );
    rejects(
        |p| p.layers[1].strokes[0].id = 1,
        CoreError::DuplicateId {
            what: "stroke id",
            id: 1,
        },
    );
    rejects(
        |p| p.layers[1].strokes[0].id = 3,
        CoreError::IdOutOfRange {
            what: "stroke id",
            id: 3,
        },
    );
    rejects(|p| p.active = 7, CoreError::UnknownLayer(LayerId::new(7)));
}

#[test]
fn rejects_bad_layer_values() {
    rejects(|p| p.layers[0].name = " ".into(), invalid("layer name"));
    rejects(|p| p.layers[0].opacity = 2.0, invalid("layer opacity"));
}

#[test]
fn rejects_bad_strokes() {
    rejects(|p| p.layers[0].strokes[0].size = 0.0, invalid("brush size"));
    rejects(
        |p| p.layers[0].strokes[0].opacity = f32::NAN,
        invalid("stroke opacity"),
    );
    rejects(|p| p.layers[0].strokes[0].color[1] = 1.5, invalid("colour"));
    rejects(
        |p| p.layers[0].strokes[0].samples.clear(),
        CoreError::EmptyStroke,
    );
    rejects(
        |p| p.layers[0].strokes[0].samples[2].time_ms = 1,
        CoreError::SampleOrder {
            previous_ms: 5,
            next_ms: 1,
        },
    );
}

#[test]
fn rejects_bad_samples() {
    rejects(
        |p| p.layers[0].strokes[0].samples[0].x = f32::INFINITY,
        invalid("canvas point"),
    );
    rejects(
        |p| p.layers[0].strokes[0].samples[0].pressure = -1.0,
        invalid("pressure"),
    );
    rejects(
        |p| p.layers[0].strokes[0].samples[0].tilt = 3.0,
        invalid("tilt"),
    );
    rejects(
        |p| p.layers[0].strokes[0].samples[0].azimuth = 9.0,
        invalid("azimuth"),
    );
}
