//! The practice pens on the GPU (S100): a stroke of the hard tip (Big, Small, value masses' greys)
//! pressed as lightly as a pen can lays the colour picked, exactly, all along its middle, as at
//! full pressure; Opaque Paint, the pens' brush before, laid a paler colour there. Here because the
//! pens are the app's and the renderer is the renderer's.

use std::sync::Arc;

use openpaint_app::{BrushKey, BrushLibrary, Pen, PracticePens};
use openpaint_core::{Artwork, CanvasPoint, CanvasSize, Color, StrokeSample, StrokeStyle};
use openpaint_render::RgbaImage;

use crate::support::{cold, core};

const SIZE: [u32; 2] = [320, 200];
const PICKED: [u8; 3] = [190, 70, 40];

fn picked() -> Color {
    let [r, g, b] = PICKED.map(|c| f32::from(c) / 255.0);
    Color::rgb(r, g, b).expect("colour")
}

/// A straight stroke across the canvas at `pressure`, drawn cold on white.
fn stroke(core: &openpaint_render::Core, style: &StrokeStyle, pressure: f32) -> RgbaImage {
    let samples: Vec<StrokeSample> = (0..24u16)
        .map(|i| {
            let x = 40.0 + 10.0 * f32::from(i);
            let at = CanvasPoint::new(x, 100.0).expect("point");
            StrokeSample::new(at, pressure, 0.0, 0.0, 4 * u32::from(i)).expect("sample")
        })
        .collect();
    let artwork = Artwork::new(CanvasSize::new(SIZE[0], SIZE[1]).expect("size"));
    let mut session = artwork
        .begin_stroke(style.clone(), samples[0])
        .expect("begin");
    session.extend(&samples[1..]).expect("extend");
    cold(core, &session.finish().artwork)
}

/// The colours along the stroke's middle line, away from its ends.
fn middle(image: &RgbaImage) -> Vec<[u8; 3]> {
    (80..240)
        .step_by(8)
        .map(|x| {
            let at = ((100 * SIZE[0] + x) * 4) as usize;
            let pixel = &image.pixels()[at..at + 3];
            [pixel[0], pixel[1], pixel[2]]
        })
        .collect()
}

#[test]
fn a_hard_tip_stroke_at_the_lightest_pressure_lays_the_colour_picked_exactly() {
    let core = core();
    let pens = PracticePens::built_in().expect("pens");
    for pen in [Pen::Big, Pen::Small, Pen::Dark] {
        let own = pens.style(pen);
        let style = StrokeStyle::new(
            Arc::clone(own.recipe()),
            own.size(),
            own.opacity(),
            picked(),
        );
        let style = style.expect("style");
        for pressure in [0.0, 0.02, 0.3, 1.0] {
            let along = middle(&stroke(&core, &style, pressure));
            assert!(
                along.iter().all(|colour| *colour == PICKED),
                "{pen:?} at {pressure}: {along:?}"
            );
        }
    }
    // Opaque Paint, the pens' brush up to 1.17.0, laid 30 % of the colour at the lightest.
    let library = BrushLibrary::built_in().expect("built-in brushes");
    let opaque = library
        .file(&BrushKey::new("opaque-paint"))
        .expect("a built-in");
    let before = StrokeStyle::new(Arc::clone(opaque.recipe()), 64.0, 1.0, picked());
    let along = middle(&stroke(&core, &before.expect("style"), 0.0));
    assert!(along.iter().all(|colour| *colour != PICKED), "{along:?}");
}
