//! Small validated values: canvas size, canvas points, colours and layer names.

use std::sync::Arc;

use crate::error::CoreError;

/// Largest canvas side in pixels.
pub const MAX_CANVAS_SIDE: u32 = 16_384;
/// Canvas coordinates may leave the canvas (a stroke can run off its edge) but stay within this.
pub const COORD_LIMIT: f32 = 1.0e6;
/// Longest layer name, in characters.
pub const MAX_NAME_CHARS: usize = 80;

pub(crate) fn invalid(what: &'static str) -> CoreError {
    CoreError::InvalidValue { what }
}

/// True for a finite value in `0..=1`.
pub(crate) fn is_unit(value: f32) -> bool {
    (0.0..=1.0).contains(&value)
}

/// Canvas size in pixels; each side in `1..=MAX_CANVAS_SIDE`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanvasSize {
    width: u32,
    height: u32,
}

impl CanvasSize {
    /// # Errors
    /// `InvalidValue` when a side is zero or above `MAX_CANVAS_SIDE`.
    pub fn new(width: u32, height: u32) -> Result<Self, CoreError> {
        let side = 1..=MAX_CANVAS_SIDE;
        if side.contains(&width) && side.contains(&height) {
            Ok(Self { width, height })
        } else {
            Err(invalid("canvas size"))
        }
    }

    #[must_use]
    pub const fn width(self) -> u32 {
        self.width
    }

    #[must_use]
    pub const fn height(self) -> u32 {
        self.height
    }
}

/// A point in canvas pixels, origin at the top-left corner, y down.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CanvasPoint {
    x: f32,
    y: f32,
}

impl CanvasPoint {
    /// # Errors
    /// `InvalidValue` when a coordinate is not finite or beyond `COORD_LIMIT`.
    pub fn new(x: f32, y: f32) -> Result<Self, CoreError> {
        let range = -COORD_LIMIT..=COORD_LIMIT;
        if range.contains(&x) && range.contains(&y) {
            Ok(Self { x, y })
        } else {
            Err(invalid("canvas point"))
        }
    }

    #[must_use]
    pub const fn x(self) -> f32 {
        self.x
    }

    #[must_use]
    pub const fn y(self) -> f32 {
        self.y
    }
}

/// An opaque sRGB colour, each channel in `0..=1`. Opacity is a separate stroke property.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    r: f32,
    g: f32,
    b: f32,
}

impl Color {
    pub const BLACK: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
    };
    pub const WHITE: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 1.0,
    };

    /// # Errors
    /// `InvalidValue` when a channel is not finite or outside `0..=1`.
    pub fn rgb(r: f32, g: f32, b: f32) -> Result<Self, CoreError> {
        if is_unit(r) && is_unit(g) && is_unit(b) {
            Ok(Self { r, g, b })
        } else {
            Err(invalid("colour"))
        }
    }

    #[must_use]
    pub const fn r(self) -> f32 {
        self.r
    }

    #[must_use]
    pub const fn g(self) -> f32 {
        self.g
    }

    #[must_use]
    pub const fn b(self) -> f32 {
        self.b
    }
}

/// A layer name: trimmed, not empty, at most `MAX_NAME_CHARS` characters, no control characters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerName(Arc<str>);

impl LayerName {
    /// Trims surrounding whitespace, then validates.
    ///
    /// # Errors
    /// `InvalidValue` when the trimmed text is empty, too long or holds a control character.
    pub fn new(text: &str) -> Result<Self, CoreError> {
        let text = text.trim();
        let valid = !text.is_empty()
            && text.chars().count() <= MAX_NAME_CHARS
            && !text.chars().any(char::is_control);
        if valid {
            Ok(Self(Arc::from(text)))
        } else {
            Err(invalid("layer name"))
        }
    }

    /// The default name of a new layer: "Layer" and its id.
    pub(crate) fn numbered(number: u64) -> Self {
        Self(Arc::from(format!("Layer {number}")))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
