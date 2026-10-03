//! `Artwork::bytes_since` (what a version adds over another, for history budgets) and O(1)
//! positional access into a layer's strokes.

use openpaint_core::{
    Artwork, ArtworkCommand, BrushRecipe, CanvasPoint, CanvasSize, Color, LayerCommand,
    StrokeSample, StrokeStyle,
};

fn blank() -> Artwork {
    Artwork::new(CanvasSize::new(64, 48).unwrap())
}

fn sample(x: f32, time_ms: u32) -> StrokeSample {
    StrokeSample::new(CanvasPoint::new(x, 10.0).unwrap(), 0.5, 0.0, 0.0, time_ms).unwrap()
}

/// A stroke of `samples` samples on the active layer.
fn draw(artwork: &Artwork, samples: u32) -> Artwork {
    let recipe = std::sync::Arc::new(BrushRecipe::plain());
    let style = StrokeStyle::new(recipe, 8.0, 1.0, Color::BLACK).unwrap();
    let mut session = artwork.begin_stroke(style, sample(0.0, 0)).unwrap();
    let rest: Vec<StrokeSample> = (1..samples).map(|i| sample(i as f32, i)).collect();
    session.extend(&rest).unwrap();
    session.finish().artwork
}

const SAMPLE_BYTES: usize = size_of::<StrokeSample>();

#[test]
fn the_same_version_adds_nothing() {
    let artwork = draw(&blank(), 10);
    assert_eq!(artwork.bytes_since(&artwork), 0);
}

#[test]
fn a_stroke_adds_its_samples_and_a_small_fixed_part() {
    let base = blank();
    let short = draw(&base, 10);
    let long = draw(&base, 10_010);
    let short_bytes = short.bytes_since(&base);
    let long_bytes = long.bytes_since(&base);
    assert!(short_bytes > 10 * SAMPLE_BYTES);
    assert_eq!(long_bytes - short_bytes, 10_000 * SAMPLE_BYTES);
}

#[test]
fn older_strokes_shared_with_the_base_are_not_counted_again() {
    let mut artwork = blank();
    for _ in 0..300 {
        artwork = draw(&artwork, 1000);
    }
    let next = draw(&artwork, 10);
    let added = next.bytes_since(&artwork);
    assert!(
        added < 1000 * SAMPLE_BYTES,
        "only the new stroke's samples count, got {added} bytes"
    );
    assert!(added > 10 * SAMPLE_BYTES);
}

#[test]
fn a_property_change_costs_only_the_layer_table() {
    let artwork = draw(&blank(), 1000);
    let layer = artwork.active();
    let command = ArtworkCommand::Layer(LayerCommand::SetVisible {
        layer,
        visible: false,
    });
    let hidden = artwork.apply(command).unwrap().artwork;
    assert!(hidden.bytes_since(&artwork) < 1000 * SAMPLE_BYTES);
}

#[test]
fn strokes_are_reached_by_position() {
    let mut artwork = blank();
    for _ in 0..600 {
        artwork = draw(&artwork, 2);
    }
    let strokes = artwork.layers()[0].strokes();
    let all: Vec<_> = strokes.iter().map(|stroke| stroke.id()).collect();
    assert_eq!(strokes.get(0).map(|s| s.id()), Some(all[0]));
    assert_eq!(strokes.get(513).map(|s| s.id()), Some(all[513]));
    assert!(strokes.get(600).is_none());
    let tail: Vec<_> = strokes.since(510).map(|stroke| stroke.id()).collect();
    assert_eq!(tail, all[510..]);
    assert_eq!(strokes.since(600).count(), 0);
    assert_eq!(strokes.since(9000).count(), 0);
}
