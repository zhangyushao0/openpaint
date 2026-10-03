//! Layers composite in stack order with their opacity and visibility (Normal, source over), and
//! switching versions by `changed_regions` equals evaluating the new version cold.

use openpaint_core::{
    Artwork, ArtworkCommand, Color, LayerCommand, LayerId, StrokeSample, StrokeStyle,
};
use openpaint_render::{CanvasTarget, FrameRequest, PixelRect, RegionEvaluator, changed_regions};

use crate::fixture::{SIZE, blank, cold, core, pixel, request, sample, with_stroke};

const RED: [u8; 4] = [255, 0, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];
/// Half of blue over red.
const HALF: [u8; 4] = [128, 0, 128, 255];
/// Where the two strokes cross.
const CROSSING: [u32; 2] = [160, 100];

fn line(color: Color, from: [f32; 2], to: [f32; 2]) -> (StrokeStyle, Vec<StrokeSample>) {
    let style = StrokeStyle::new(24.0, 1.0, color).expect("style");
    let samples = (0..=8u16)
        .map(|i| {
            let t = f32::from(i) / 8.0;
            let [x, y] = [
                from[0] + (to[0] - from[0]) * t,
                from[1] + (to[1] - from[1]) * t,
            ];
            sample(x, y, 1.0, u32::from(i) * 8)
        })
        .collect();
    (style, samples)
}

fn red() -> (StrokeStyle, Vec<StrokeSample>) {
    line(
        Color::rgb(1.0, 0.0, 0.0).expect("red"),
        [40.0, 100.0],
        [280.0, 100.0],
    )
}

fn blue() -> (StrokeStyle, Vec<StrokeSample>) {
    line(
        Color::rgb(0.0, 0.0, 1.0).expect("blue"),
        [160.0, 20.0],
        [160.0, 180.0],
    )
}

fn layer(artwork: &Artwork, command: LayerCommand) -> Artwork {
    artwork
        .apply(ArtworkCommand::Layer(command))
        .expect("layer command")
        .artwork
}

/// Red on the bottom layer, blue on a layer above it; returns the artwork and both ids.
fn stacked() -> (Artwork, LayerId, LayerId) {
    let bottom = with_stroke(&blank(), red());
    let created = layer(&bottom, LayerCommand::Create);
    let top = created.active();
    (with_stroke(&created, blue()), bottom.active(), top)
}

#[test]
fn the_upper_layer_covers_the_lower_one_in_stack_order() {
    let core = core();
    let (artwork, bottom, top) = stacked();
    assert_eq!(pixel(&cold(&core, &artwork), CROSSING), BLUE);
    let lowered = layer(
        &artwork,
        LayerCommand::Reorder {
            layer: top,
            index: 0,
        },
    );
    assert_eq!(pixel(&cold(&core, &lowered), CROSSING), RED);
    let raised = layer(
        &lowered,
        LayerCommand::Reorder {
            layer: bottom,
            index: 0,
        },
    );
    assert_eq!(cold(&core, &raised), cold(&core, &artwork));
    let half = layer(
        &artwork,
        LayerCommand::SetOpacity {
            layer: top,
            opacity: 0.5,
        },
    );
    assert_eq!(pixel(&cold(&core, &half), CROSSING), HALF);
    let hidden = layer(
        &artwork,
        LayerCommand::SetVisible {
            layer: top,
            visible: false,
        },
    );
    assert_eq!(pixel(&cold(&core, &hidden), CROSSING), RED);
    let deleted = layer(&artwork, LayerCommand::Delete(bottom));
    let image = cold(&core, &deleted);
    assert_eq!(pixel(&image, CROSSING), BLUE);
    assert_eq!(
        pixel(&image, [60, 100]),
        [255; 4],
        "the red stroke went with its layer"
    );
}

#[test]
fn layer_edits_through_changed_regions_equal_cold() {
    let core = core();
    let (artwork, bottom, top) = stacked();
    let steps = [
        LayerCommand::SetOpacity {
            layer: top,
            opacity: 0.3,
        },
        LayerCommand::Reorder {
            layer: top,
            index: 0,
        },
        LayerCommand::SetVisible {
            layer: bottom,
            visible: false,
        },
        LayerCommand::Create,
        LayerCommand::Delete(top),
    ];
    let mut versions = vec![artwork];
    for command in steps {
        let next = layer(versions.last().expect("version"), command);
        versions.push(next);
    }
    let mut target = CanvasTarget::new(&core, SIZE).expect("target");
    let mut shown = versions[0].clone();
    RegionEvaluator::new(&core, &mut target)
        .evaluate(&request(&shown), &[PixelRect::full(SIZE)])
        .expect("evaluate");
    // Forward through every edit, then back as undo goes.
    let order = (1..versions.len()).chain((0..versions.len() - 1).rev());
    for index in order {
        let next = &versions[index];
        let regions = changed_regions(&shown, next);
        RegionEvaluator::new(&core, &mut target)
            .evaluate(&request(next), &regions)
            .expect("evaluate");
        assert!(
            target.read(&core).expect("readback") == cold(&core, next),
            "version {index}"
        );
        shown = next.clone();
    }
}

#[test]
fn a_stroke_drawn_under_another_layer_stays_under_it() {
    let core = core();
    let (artwork, bottom, _) = stacked();
    let below = layer(&artwork, LayerCommand::Select(bottom));
    let mut target = CanvasTarget::new(&core, SIZE).expect("target");
    RegionEvaluator::new(&core, &mut target)
        .evaluate(&request(&below), &[PixelRect::full(SIZE)])
        .expect("evaluate");
    let (style, samples) = line(Color::BLACK, [160.0, 60.0], [160.0, 140.0]);
    let mut session = below.begin_stroke(style, samples[0]).expect("begin");
    for chunk in samples[1..].chunks(3) {
        session.extend(chunk).expect("extend");
        let frame = FrameRequest {
            active: Some(session.active()),
            ..request(&below)
        };
        RegionEvaluator::new(&core, &mut target)
            .evaluate(&frame, &[])
            .expect("evaluate");
        let shown = target.read(&core).expect("readback");
        assert_eq!(pixel(&shown, CROSSING), BLUE, "the blue layer stays on top");
    }
    let finished = session.finish().artwork;
    RegionEvaluator::new(&core, &mut target)
        .evaluate(&request(&finished), &[])
        .expect("evaluate");
    assert!(target.read(&core).expect("readback") == cold(&core, &finished));
}
