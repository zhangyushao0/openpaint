//! Adjustments in core (S28): an applied adjustment is an operation record at its place among
//! the target's strokes, with its selection and, on an inverted mask, that it acts on the shown
//! grey; refused on a locked layer; it stores and loads, and bad values do not load.

use openpaint_core::{
    Adjustment, AdjustmentKind, AdjustmentParts, Artwork, ArtworkCommand, CanvasSize, CoreError,
    Curves, EditParts, Hsb, LayerCommand, LayerId, MaskCommand, RegionMask, ToneChannel, ToneCurve,
};

const LAYER: LayerId = LayerId::new(1);

fn canvas() -> CanvasSize {
    CanvasSize::new(16, 12).expect("size")
}

fn darker() -> Adjustment {
    Adjustment::Hsb(Hsb::new(0.5, 0.5, 0.25))
}

#[test]
fn an_adjustment_is_recorded_on_the_layer_with_its_selection() {
    let artwork = Artwork::new(canvas());
    let adjusted = artwork
        .adjust_region(None, darker())
        .expect("adjusted")
        .artwork;
    let layer = adjusted.layer(LAYER).expect("layer");
    let edit = layer.edits()[0].adjust().expect("an adjustment");
    assert_eq!(
        (edit.after(), edit.clip(), edit.inverted()),
        (0, None, false)
    );
    assert_eq!(edit.adjustment(), &darker());
    let mut inside = vec![false; 16 * 12];
    inside[3 * 16 + 2] = true;
    let region = RegionMask::from_pixels(canvas(), &inside).expect("region");
    let selected = adjusted
        .adjust_region(Some(region.clone()), darker())
        .expect("inside the selection")
        .artwork;
    let edits = selected.layer(LAYER).expect("layer").edits();
    assert_eq!(edits[1].adjust().and_then(|e| e.clip()), Some(&region));
    let locked = artwork
        .apply(ArtworkCommand::Layer(LayerCommand::SetLocked {
            layer: LAYER,
            locked: true,
        }))
        .expect("lock")
        .artwork;
    let refused = locked.adjust_region(None, darker());
    assert_eq!(refused.err(), Some(CoreError::LayerLocked(LAYER)));
}

#[test]
fn a_mask_records_whether_it_was_inverted_and_everything_stores() {
    let inverted = Artwork::new(canvas())
        .apply(ArtworkCommand::Mask(MaskCommand::Add(LAYER)))
        .and_then(|a| {
            a.artwork
                .apply(ArtworkCommand::Mask(MaskCommand::Invert(LAYER)))
        })
        .expect("an inverted mask")
        .artwork;
    let lifted = ToneCurve::new(&[[0.0, 0.0], [0.3, 0.6], [1.0, 1.0]]).expect("curve");
    let curves = Adjustment::Curves(Curves::default().with(ToneChannel::Composite, lifted));
    let adjusted = inverted
        .adjust_region(None, curves)
        .expect("adjusted")
        .artwork;
    let mask = adjusted.layer(LAYER).and_then(|l| l.mask()).expect("mask");
    assert!(mask.edits()[0].adjust().expect("on the mask").inverted());
    assert!(adjusted.layer(LAYER).expect("layer").edits().is_empty());
    let parts = adjusted.to_parts();
    let loaded = Artwork::from_storage(parts.clone()).expect("loads");
    assert_eq!(loaded.to_parts(), parts);
    let mut bad = parts;
    let edits = &mut bad.layers[0].mask.as_mut().expect("mask").edits;
    let EditParts::Adjust(adjust) = &mut edits[0] else {
        panic!("an adjustment record");
    };
    adjust.adjustment = AdjustmentParts::Hsb([0.5, 1.5, 0.5]);
    assert!(Artwork::from_storage(bad.clone()).is_err(), "out of range");
    let EditParts::Adjust(adjust) = &mut bad.layers[0].mask.as_mut().expect("mask").edits[0] else {
        panic!("an adjustment record");
    };
    adjust.adjustment = AdjustmentParts::Curves(std::array::from_fn(|_| vec![[0.0, 0.0]]));
    assert!(Artwork::from_storage(bad).is_err(), "a curve ends at x = 1");
}

#[test]
fn every_adjustment_starts_as_none() {
    for kind in [
        AdjustmentKind::HueSaturationBrightness,
        AdjustmentKind::Curves,
    ] {
        let none = Adjustment::none(kind);
        assert!(none.is_none());
        assert_eq!(none.kind(), kind);
    }
    assert!(!darker().is_none());
    assert_eq!(Hsb::new(f32::NAN, 2.0, -1.0), Hsb::new(0.5, 1.0, 0.0));
    let luts = Curves::default().luts();
    assert!(
        luts.iter()
            .all(|lut| lut.iter().enumerate().all(|(i, v)| usize::from(*v) == i))
    );
}

#[test]
fn blur_and_sharpen_are_adjustment_records_whose_values_are_checked() {
    use openpaint_core::{Blur, Sharpen};
    for kind in [AdjustmentKind::GaussianBlur, AdjustmentKind::Sharpen] {
        let none = Adjustment::none(kind);
        assert!(none.is_none());
        assert_eq!(none.kind(), kind);
    }
    assert_eq!(Blur::new(f32::NAN), Blur::NONE);
    assert_eq!(Blur::new(99.0).radius, Blur::MAX_RADIUS);
    assert_eq!(Sharpen::new(-1.0), Sharpen::NONE);
    let artwork = Artwork::new(canvas())
        .adjust_region(None, Adjustment::Blur(Blur::new(4.5)))
        .expect("blurred")
        .artwork
        .adjust_region(None, Adjustment::Sharpen(Sharpen::new(0.25)))
        .expect("sharpened")
        .artwork;
    let parts = artwork.to_parts();
    let loaded = Artwork::from_storage(parts.clone()).expect("loads");
    assert_eq!(loaded.to_parts(), parts);
    for (at, value) in [
        (0, AdjustmentParts::Blur(40.0)),
        (1, AdjustmentParts::Sharpen(1.5)),
    ] {
        let mut bad = parts.clone();
        let EditParts::Adjust(adjust) = &mut bad.layers[0].edits[at] else {
            panic!("an adjustment record");
        };
        adjust.adjustment = value;
        assert!(Artwork::from_storage(bad).is_err(), "out of range");
    }
}
