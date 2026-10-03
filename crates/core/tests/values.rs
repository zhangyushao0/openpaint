//! Validated values: every constructor accepts its range and rejects what lies outside.

use std::f32::consts::{FRAC_PI_2, PI};
use std::sync::Arc;

use openpaint_core::{
    BrushRecipe, COORD_LIMIT, CanvasPoint, CanvasSize, Color, CoreError, LayerName, MAX_BRUSH_SIZE,
    MAX_CANVAS_SIDE, StrokeSample, StrokeStyle,
};

fn invalid(what: &'static str) -> CoreError {
    CoreError::InvalidValue { what }
}

fn origin() -> CanvasPoint {
    CanvasPoint::new(0.0, 0.0).unwrap()
}

#[test]
fn canvas_size_range() {
    let size = CanvasSize::new(2560, 1600).unwrap();
    assert_eq!((size.width(), size.height()), (2560, 1600));
    assert!(CanvasSize::new(MAX_CANVAS_SIDE, 1).is_ok());
    assert_eq!(CanvasSize::new(0, 10), Err(invalid("canvas size")));
    assert_eq!(
        CanvasSize::new(10, MAX_CANVAS_SIDE + 1),
        Err(invalid("canvas size"))
    );
}

#[test]
fn canvas_point_is_finite_and_bounded() {
    let point = CanvasPoint::new(-5.5, 12.0).unwrap();
    assert_eq!((point.x(), point.y()), (-5.5, 12.0));
    assert!(CanvasPoint::new(COORD_LIMIT, -COORD_LIMIT).is_ok());
    for (x, y) in [
        (f32::NAN, 0.0),
        (0.0, f32::INFINITY),
        (COORD_LIMIT * 2.0, 0.0),
    ] {
        assert_eq!(CanvasPoint::new(x, y), Err(invalid("canvas point")));
    }
}

#[test]
fn color_channels_are_unit() {
    let color = Color::rgb(0.25, 0.5, 1.0).unwrap();
    assert_eq!((color.r(), color.g(), color.b()), (0.25, 0.5, 1.0));
    assert_eq!(Color::rgb(0.0, 0.0, 0.0).unwrap(), Color::BLACK);
    for (r, g, b) in [(-0.1, 0.0, 0.0), (0.0, 1.5, 0.0), (0.0, 0.0, f32::NAN)] {
        assert_eq!(Color::rgb(r, g, b), Err(invalid("colour")));
    }
}

#[test]
fn layer_name_is_trimmed_and_limited() {
    assert_eq!(LayerName::new("  Inks ").unwrap().as_str(), "Inks");
    assert!(LayerName::new(&"é".repeat(80)).is_ok());
    for bad in ["", "   ", "line\nbreak", &"a".repeat(81)] {
        assert_eq!(LayerName::new(bad), Err(invalid("layer name")), "{bad:?}");
    }
}

#[test]
fn sample_ranges() {
    let sample = StrokeSample::new(origin(), 0.5, FRAC_PI_2, -PI, 7).unwrap();
    assert_eq!(sample.pressure(), 0.5);
    assert_eq!(sample.tilt(), FRAC_PI_2);
    assert_eq!(sample.azimuth(), -PI);
    assert_eq!(sample.time_ms(), 7);
    let cases = [
        (1.1, 0.0, 0.0, "pressure"),
        (f32::NAN, 0.0, 0.0, "pressure"),
        (0.5, -0.1, 0.0, "tilt"),
        (0.5, 2.0, 0.0, "tilt"),
        (0.5, 0.0, 4.0, "azimuth"),
    ];
    for (pressure, tilt, azimuth, what) in cases {
        let result = StrokeSample::new(origin(), pressure, tilt, azimuth, 0);
        assert_eq!(result, Err(invalid(what)));
    }
}

#[test]
fn style_ranges() {
    let plain = Arc::new(BrushRecipe::plain());
    let style = |size, opacity, color| StrokeStyle::new(Arc::clone(&plain), size, opacity, color);
    let white = style(12.0, 0.8, Color::WHITE).unwrap();
    assert_eq!((white.size(), white.opacity()), (12.0, 0.8));
    assert_eq!(white.color(), Color::WHITE);
    assert!(Arc::ptr_eq(white.recipe(), &plain));
    assert!(style(MAX_BRUSH_SIZE, 1.0, Color::BLACK).is_ok());
    for size in [0.0, -1.0, MAX_BRUSH_SIZE + 1.0, f32::NAN] {
        let result = style(size, 1.0, Color::BLACK);
        assert_eq!(result, Err(invalid("brush size")));
    }
    for opacity in [0.0, 1.5, f32::NAN] {
        let result = style(10.0, opacity, Color::BLACK);
        assert_eq!(result, Err(invalid("stroke opacity")));
    }
}
