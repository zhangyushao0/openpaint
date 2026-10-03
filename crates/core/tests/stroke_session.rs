//! `StrokeSession`: begin, extend, finish and cancel, and that a stroke in progress never
//! touches the artwork it started on.

use std::sync::Arc;

use openpaint_core::{
    Artwork, ArtworkCommand, BrushRecipe, CanvasPoint, CanvasSize, Change, Color, CoreError,
    LayerCommand, LayerId, MAX_STROKE_SAMPLES, StrokeId, StrokeSample, StrokeSession, StrokeStyle,
};

fn blank() -> Artwork {
    Artwork::new(CanvasSize::new(64, 48).unwrap())
}

fn sample(x: f32, time_ms: u32) -> StrokeSample {
    let position = CanvasPoint::new(x, 10.0).unwrap();
    StrokeSample::new(position, 0.5, 0.2, 1.0, time_ms).unwrap()
}

fn style() -> StrokeStyle {
    StrokeStyle::new(Arc::new(BrushRecipe::plain()), 8.0, 1.0, Color::BLACK).unwrap()
}

fn begin(artwork: &Artwork) -> StrokeSession {
    artwork.begin_stroke(style(), sample(0.0, 0)).unwrap()
}

fn draw(artwork: &Artwork) -> Artwork {
    let mut session = begin(artwork);
    session.extend(&[sample(1.0, 4), sample(2.0, 8)]).unwrap();
    session.finish().artwork
}

#[test]
fn finish_commits_one_stroke_to_the_active_layer() {
    let base = blank();
    let mut session = begin(&base);
    session.extend(&[sample(1.0, 4), sample(2.0, 4)]).unwrap();
    session.extend(&[sample(3.0, 9)]).unwrap();
    assert_eq!(session.active().samples().len(), 4);
    assert_eq!(session.active().layer(), base.active());
    assert!(
        base.layers()[0].strokes().is_empty(),
        "the base never sees the active stroke"
    );
    let applied = session.finish();
    assert_eq!(applied.change, Change::Content);
    let strokes = applied.artwork.layers()[0].strokes();
    assert_eq!(strokes.len(), 1);
    let stroke = strokes.iter().next().unwrap();
    assert_eq!(stroke.id(), StrokeId::new(1));
    assert_eq!(stroke.style(), &style());
    let times: Vec<u32> = stroke.samples().iter().map(StrokeSample::time_ms).collect();
    assert_eq!(times, [0, 4, 4, 9]);
}

#[test]
fn stroke_ids_count_up_across_layers() {
    let one = draw(&blank());
    let two = one
        .apply(ArtworkCommand::Layer(LayerCommand::Create))
        .unwrap()
        .artwork;
    let two = draw(&two);
    let ids: Vec<u64> = two
        .layers()
        .iter()
        .flat_map(|layer| layer.strokes().iter().map(|stroke| stroke.id().get()))
        .collect();
    assert_eq!(ids, [1, 2]);
}

#[test]
fn cancel_returns_the_base_version() {
    let base = blank();
    let mut session = begin(&base);
    session.extend(&[sample(5.0, 3)]).unwrap();
    let back = session.cancel();
    assert!(back.same_version(&base));
}

#[test]
fn begin_on_a_locked_layer_fails() {
    let id = LayerId::new(1);
    let locked = blank()
        .apply(ArtworkCommand::Layer(LayerCommand::SetLocked {
            layer: id,
            locked: true,
        }))
        .unwrap()
        .artwork;
    let result = locked.begin_stroke(style(), sample(0.0, 0));
    assert_eq!(result.unwrap_err(), CoreError::LayerLocked(id));
}

#[test]
fn extend_rejects_time_going_back_and_keeps_the_stroke() {
    let mut session = begin(&blank());
    session.extend(&[sample(1.0, 10)]).unwrap();
    let result = session.extend(&[sample(2.0, 12), sample(3.0, 11)]);
    assert_eq!(
        result,
        Err(CoreError::SampleOrder {
            previous_ms: 12,
            next_ms: 11
        })
    );
    let result = session.extend(&[sample(2.0, 9)]);
    assert_eq!(
        result,
        Err(CoreError::SampleOrder {
            previous_ms: 10,
            next_ms: 9
        })
    );
    assert_eq!(
        session.active().samples().len(),
        2,
        "failed batches add nothing"
    );
}

#[test]
fn extend_stops_at_the_sample_limit() {
    let mut session = begin(&blank());
    let batch = vec![sample(1.0, 1); MAX_STROKE_SAMPLES - 1];
    session.extend(&batch).unwrap();
    let result = session.extend(&[sample(1.0, 2)]);
    assert_eq!(
        result,
        Err(CoreError::SampleLimit {
            max: MAX_STROKE_SAMPLES
        })
    );
    assert_eq!(session.active().samples().len(), MAX_STROKE_SAMPLES);
}

#[test]
fn many_strokes_keep_order_and_old_versions() {
    let mut versions = vec![blank()];
    for _ in 0..600 {
        let next = draw(versions.last().unwrap());
        versions.push(next);
    }
    let last = &versions[600].layers()[0];
    assert_eq!(last.strokes().len(), 600);
    let ids: Vec<u64> = last
        .strokes()
        .iter()
        .map(|stroke| stroke.id().get())
        .collect();
    assert_eq!(ids, (1..=600).collect::<Vec<u64>>());
    let newest = last.strokes().last().map(|stroke| stroke.id().get());
    assert_eq!(newest, Some(600));
    assert!(versions[0].layers()[0].strokes().last().is_none());
    assert_eq!(versions[300].layers()[0].strokes().len(), 300);
    assert_ne!(versions[599], versions[600]);
}
