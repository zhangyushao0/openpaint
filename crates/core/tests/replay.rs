//! The drawing order (S163, D125): every stroke and edit a step at its place (a stroke's id, an
//! edit's `before_stroke`; ties by list, lower layer, paint before mask), merged layers' content
//! as steps, copies at their new ids, hidden lists not shown; strokes keep their start; and the
//! frames: every list cut to a prefix, valid artworks sharing their `Arc`s, the stroke being drawn
//! live on its layer.

use std::sync::Arc;

use openpaint_core::{
    Artwork, ArtworkCommand, BrushRecipe, CanvasPoint, CanvasSize, Color, EditKind, EditParts,
    LayerCommand, LayerId, MaskCommand, RegionMask, Step, StepKind, StrokeId, StrokeSample,
    StrokeStyle,
};

const T: u64 = 1_794_690_420_000;

fn blank() -> Artwork {
    Artwork::new(CanvasSize::new(64, 48).expect("size"))
}

/// `artwork` with a stroke of `samples` samples 10 ms apart from `x` on its target, begun at
/// `began` (0: unknown).
fn draw(artwork: &Artwork, x: f32, began: u64, samples: u32) -> Artwork {
    let style = StrokeStyle::new(Arc::new(BrushRecipe::plain()), 4.0, 1.0, Color::BLACK);
    let sample = |i: u32| {
        let point = CanvasPoint::new(x + i as f32, 10.0).expect("point");
        StrokeSample::new(point, 0.5, 0.0, 0.0, i * 10).expect("sample")
    };
    let session = artwork.begin_stroke(style.expect("style"), sample(0));
    let mut session = session.expect("begin").began_at(began);
    let rest: Vec<StrokeSample> = (1..samples).map(sample).collect();
    session.extend(&rest).expect("extend");
    session.finish().artwork
}

fn layer(artwork: &Artwork, command: LayerCommand) -> Artwork {
    let applied = artwork.apply(ArtworkCommand::Layer(command));
    applied.expect("layer command").artwork
}

fn mask(artwork: &Artwork, command: MaskCommand) -> Artwork {
    let applied = artwork.apply(ArtworkCommand::Mask(command));
    applied.expect("mask command").artwork
}

fn fill(artwork: &Artwork) -> Artwork {
    let region = RegionMask::polygon(artwork.canvas(), &[[2.0, 2.0], [30.0, 2.0], [2.0, 30.0]]);
    let filled = artwork.fill_region(region.expect("region"), Color::BLACK);
    filled.expect("fill").artwork
}

/// Each step as `s<id>` or `e`, in order.
fn names(artwork: &Artwork) -> Vec<String> {
    let order = artwork.drawing_order();
    let steps = order.steps().iter().map(|step| match step.kind {
        StepKind::Stroke { id, .. } => format!("s{}", id.get()),
        StepKind::Edit(_) => "e".to_owned(),
    });
    steps.collect()
}

#[test]
fn strokes_take_their_ids_places_and_edits_the_next_stroke_id_when_made() {
    let one = fill(&draw(&blank(), 2.0, T, 3));
    let two = draw(&layer(&one, LayerCommand::Create), 20.0, T + 1_000, 2);
    let two = fill(&two);
    let back = draw(
        &layer(&two, LayerCommand::Select(LayerId::new(1))),
        4.0,
        T + 2_000,
        4,
    );
    assert_eq!(
        back.layers()[0].edits()[0].before_stroke(),
        2,
        "the fill after stroke 1"
    );
    assert_eq!(
        back.layers()[1].edits()[0].before_stroke(),
        3,
        "the fill after stroke 2"
    );
    let order = back.drawing_order();
    let kinds: Vec<_> = order
        .steps()
        .iter()
        .map(|step| (step.kind, step.layer))
        .collect();
    let stroke = |id: u64, began, samples, drawn_ms| StepKind::Stroke {
        id: StrokeId::new(id),
        began_ms: Some(began),
        samples,
        drawn_ms,
    };
    let (first, second) = (LayerId::new(1), LayerId::new(2));
    assert_eq!(
        kinds,
        [
            (stroke(1, T, 3, 20), first),
            (StepKind::Edit(EditKind::Fill), first),
            (stroke(2, T + 1_000, 2, 10), second),
            (StepKind::Edit(EditKind::Fill), second),
            (stroke(3, T + 2_000, 4, 30), first),
        ]
    );
    assert!(order.steps().iter().all(|step| step.shows));
    let live: Vec<bool> = order.steps().iter().map(|step| step.live).collect();
    assert_eq!(live, [true, false, true, false, true]);
    assert_eq!(back.stroke_count(), 3);
    assert!(order.stroke(0).is_some() && order.stroke(1).is_none());
}

#[test]
fn edits_of_one_place_go_lower_layer_first_and_paint_before_mask() {
    let base = draw(&blank(), 2.0, T, 2);
    let upper = fill(&layer(&base, LayerCommand::Create));
    // Made later, at the same place: the lower layer's fill, then its mask's.
    let lower = fill(&layer(&upper, LayerCommand::Select(LayerId::new(1))));
    let masked = fill(&mask(&lower, MaskCommand::Add(LayerId::new(1))));
    let order = masked.drawing_order();
    let edits: Vec<(LayerId, bool)> = order
        .steps()
        .iter()
        .filter(|step| matches!(step.kind, StepKind::Edit(_)))
        .map(|step| (step.layer, step.live))
        .collect();
    assert_eq!(
        edits,
        [
            (LayerId::new(1), false),
            (LayerId::new(1), false),
            (LayerId::new(2), false),
        ]
    );
    let first = &masked.layers()[0];
    let mask_place = first.mask().expect("a mask").edits()[0].before_stroke();
    assert_eq!((first.edits()[0].before_stroke(), mask_place), (2, 2));
    // The paint's fill is the second step, the mask's the third: paint before mask.
    let frame = order.frame(2, 0);
    let held = &frame.artwork.layers()[0];
    assert_eq!(held.edits().len(), 1);
    assert!(held.mask().expect("its mask").edits().is_empty());
}

#[test]
fn merged_layers_content_steps_and_hidden_lists_do_not_show() {
    let lower = draw(&blank(), 2.0, T, 2);
    let upper = draw(&layer(&lower, LayerCommand::Create), 20.0, T + 100, 2);
    let lower_again = layer(&upper, LayerCommand::Select(LayerId::new(1)));
    let lower_again = draw(&lower_again, 4.0, T + 200, 2);
    let merged = layer(&lower_again, LayerCommand::MergeDown(LayerId::new(2)));
    let order = merged.drawing_order();
    assert_eq!(names(&merged)[..3], ["s1", "s2", "s3"]);
    let live: Vec<bool> = order.steps().iter().map(|step| step.live).collect();
    assert_eq!(
        live,
        [true, false, true],
        "the merged stroke is no live one"
    );
    assert!(
        order
            .steps()
            .iter()
            .all(|step| step.layer == LayerId::new(1))
    );
    assert_eq!(merged.stroke_count(), 3);
    // Strokes inside a merged record show whole once done; the record keeps its place among
    // the strokes kept.
    assert!(order.frame(2, 1).active.is_some(), "stroke 3 begun");
    let frame = order.frame(2, 0);
    assert!(frame.active.is_none(), "stroke 3 not begun");
    let held = &frame.artwork.layers()[0];
    assert_eq!(held.strokes().len(), 1);
    let record = held.edits()[0].nested().expect("the merged record");
    assert_eq!((record.after(), record.layer().strokes().len()), (1, 1));
    // A hidden layer and a disabled mask: steps that do not show.
    let masked = draw(
        &mask(&merged, MaskCommand::Add(LayerId::new(1))),
        6.0,
        T + 300,
        2,
    );
    let off = MaskCommand::SetEnabled {
        layer: LayerId::new(1),
        enabled: false,
    };
    let off = mask(&masked, off);
    let shows: Vec<bool> = off
        .drawing_order()
        .steps()
        .iter()
        .map(|s| s.shows)
        .collect();
    assert_eq!(shows, [true, true, true, false]);
    let hidden = layer(
        &off,
        LayerCommand::SetVisible {
            layer: LayerId::new(1),
            visible: false,
        },
    );
    let order = hidden.drawing_order();
    assert!(order.steps().iter().all(|step| !step.shows));
}

#[test]
fn copies_take_their_new_ids_places_with_their_edits_and_keep_their_starts() {
    let one = draw(&fill(&draw(&blank(), 2.0, T, 2)), 4.0, T + 500, 2);
    let copied = layer(&one, LayerCommand::Duplicate(LayerId::new(1)));
    let copy = copied.layers().iter().find(|l| l.id() != LayerId::new(1));
    let copy = copy.expect("the copy");
    let ids: Vec<u64> = copy.strokes().iter().map(|s| s.id().get()).collect();
    assert_eq!(ids, [3, 4]);
    let starts: Vec<_> = copy.strokes().iter().map(|s| s.began_ms()).collect();
    assert_eq!(starts, [Some(T), Some(T + 500)], "a copy keeps its starts");
    assert_eq!(
        copy.edits()[0].before_stroke(),
        4,
        "after the copy of stroke 1"
    );
    assert_eq!(names(&copied), ["s1", "e", "s2", "s3", "e", "s4"]);
    // A paste: its records renumbered, each placed once those before it took their ids.
    let pasted = one
        .copied(false, None)
        .and_then(|copy| one.paste(&copy))
        .expect("pasted")
        .artwork;
    let record = pasted.layers()[0]
        .edits()
        .last()
        .expect("the pasted record");
    let content = record.nested().expect("a merged record").layer();
    let ids: Vec<u64> = content.strokes().iter().map(|s| s.id().get()).collect();
    assert_eq!((ids, record.before_stroke()), (vec![3, 4], 5));
    assert_eq!(content.edits()[0].before_stroke(), 4);
    assert_eq!(pasted.stroke_count(), 4);
}

#[test]
fn a_stroke_keeps_its_start_through_storage() {
    let drawn = draw(&draw(&blank(), 2.0, T, 2), 8.0, 0, 3);
    let starts: Vec<_> = drawn.layers()[0]
        .strokes()
        .iter()
        .map(|s| s.began_ms())
        .collect();
    assert_eq!(starts, [Some(T), None]);
    let parts = drawn.to_parts();
    assert_eq!(parts.layers[0].strokes[0].began_ms, T);
    let stored = Artwork::from_storage(parts.clone()).expect("valid");
    assert_eq!(stored.to_parts(), parts);
    let mut later = fill(&drawn).to_parts();
    let EditParts::Fill(edit) = &mut later.layers[0].edits[0] else {
        panic!("a fill");
    };
    assert_eq!(edit.before_stroke, 3, "after strokes 1 and 2");
    edit.before_stroke = 4;
    assert!(
        Artwork::from_storage(later).is_err(),
        "a place past the next stroke id"
    );
}

/// An artwork of every kind of list: two layers, a mask, a merged layer, fills, a long list.
fn busy() -> Artwork {
    let mut artwork = blank();
    for at in 0..300 {
        artwork = draw(&artwork, (at % 40) as f32, T + at * 50, 3);
    }
    let artwork = fill(&artwork);
    let artwork = draw(&layer(&artwork, LayerCommand::Create), 20.0, T + 20_000, 2);
    let artwork = draw(&layer(&artwork, LayerCommand::Create), 22.0, T + 21_000, 2);
    let artwork = layer(&artwork, LayerCommand::MergeDown(LayerId::new(3)));
    let artwork = draw(
        &mask(&artwork, MaskCommand::Add(LayerId::new(2))),
        3.0,
        T + 22_000,
        5,
    );
    let artwork = fill(&artwork);
    let artwork = layer(&artwork, LayerCommand::Select(LayerId::new(1)));
    draw(&artwork, 9.0, T + 23_000, 2)
}

/// `frame` is a valid artwork, and only the layer of `step` (its last step) changed from
/// `before`: a stroke appended there shares every stroke before it.
fn only_its_layer_changed(before: &Artwork, frame: &Artwork, step: Step) {
    let parts = frame.to_parts();
    let stored = Artwork::from_storage(parts.clone()).expect("valid");
    assert_eq!(stored.to_parts(), parts);
    for (old, new) in before.layers().iter().zip(frame.layers()) {
        let drawn = old.id() == step.layer;
        assert!(drawn || old.same_paint(new), "layer {:?}", old.id());
        let appended = new.strokes().len() == old.strokes().len() + 1;
        if drawn && step.live && appended {
            let kept = old.strokes().last().is_none_or(|last| {
                let same = new.strokes().get(old.strokes().len() - 1);
                same.expect("kept").shares_samples(last)
            });
            assert!(kept, "an append");
        }
    }
}

#[test]
fn every_frame_is_a_valid_prefix_and_appends_share_their_arcs() {
    let artwork = busy();
    let order = artwork.drawing_order();
    let steps = order.steps().len();
    let fills = steps - artwork.stroke_count();
    assert_eq!(fills, 2, "the strokes and two fills");
    let blank = order.frame(0, 0).artwork;
    let empty =
        |layer: &openpaint_core::Layer| layer.strokes().is_empty() && layer.edits().is_empty();
    assert!(blank.layers().iter().all(empty));
    assert_eq!(blank.layers().len(), artwork.layers().len());
    let mut before = blank;
    for done in 1..=steps {
        let frame = order.frame(done, 0).artwork;
        only_its_layer_changed(&before, &frame, order.steps()[done - 1]);
        before = frame;
    }
    assert!(
        before.same_version(&artwork),
        "every step done: the very artwork"
    );
    // One step before the end, every list but the last step's is the artwork's own.
    let last = order.frame(steps - 1, 0).artwork;
    let shared = last
        .layers()
        .iter()
        .zip(artwork.layers())
        .filter(|(a, b)| a.same_paint(b));
    assert_eq!(shared.count(), artwork.layers().len() - 1);
    assert!(
        order
            .frame(5, 0)
            .artwork
            .same_version(&order.frame(5, 0).artwork),
        "kept"
    );
}

#[test]
fn the_stroke_being_drawn_is_live_on_its_layer_s_paint_or_mask() {
    let artwork = busy();
    let order = artwork.drawing_order();
    let at = |find: &dyn Fn(&Step) -> bool| order.steps().iter().position(find).expect("a step");
    let painted = at(&|step| step.live);
    let frame = order.frame(painted, 2);
    let active = frame.active.expect("drawn live");
    assert_eq!(
        (active.layer(), active.samples().len()),
        (order.steps()[painted].layer, 2)
    );
    assert!(!active.on_mask());
    assert_eq!(active.began_ms(), Some(T));
    assert!(order.frame(painted, 0).active.is_none(), "not begun");
    assert_eq!(
        order
            .frame(painted, 99)
            .active
            .expect("all")
            .samples()
            .len(),
        3
    );
    let merged = at(&|step| !step.live && matches!(step.kind, StepKind::Stroke { .. }));
    assert!(
        order.frame(merged, 2).active.is_none(),
        "a merged stroke shows whole"
    );
    let edit = at(&|step| matches!(step.kind, StepKind::Edit(_)));
    assert!(order.frame(edit, 1).active.is_none());
    let on_mask = order.steps().iter().enumerate().position(|(index, step)| {
        step.live
            && order
                .frame(index, 1)
                .active
                .is_some_and(|active| active.on_mask())
    });
    let on_mask = on_mask.expect("a stroke on the mask");
    assert_eq!(order.steps()[on_mask].layer, LayerId::new(2));
}
