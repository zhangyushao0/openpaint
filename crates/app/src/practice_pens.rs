//! The pens an exercise's own tools draw and paint with (S76, S82). For lines and shapes: a light
//! pencil to mark points and draw construction lines (V大's right process: they are never judged
//! and are left out of the aligned fit), the ink pen for the copy, and after Compare the blue pen
//! that corrects it, as on paper. For colour combinations and value scales, in the colour picked
//! on the wheel: a big brush for the masses first, a small one for the edges and the hierarchy
//! later, and a soft one for soft edges (V大: a big brush first, smaller as the copy goes on).
//! Each is a built-in brush as its file has it (the user's edits never reach them) at a size of
//! its own; the toolbox is not touched.

use std::sync::Arc;

use openpaint_core::{Color, CoreError, StrokeStyle};

use crate::brushes::{BrushKey, BrushLibrary};
use crate::practice::Pen;

/// The Draw and Correct pens' diameter, canvas pixels: Compare lays the original's lines over the
/// copy as thick (S88), so an error of a pen's width shows.
pub const INK_DIAMETER: f32 = 5.0;

/// Each pen's brush, diameter (canvas pixels; a frame is 400), opacity and colour (a painting
/// pen takes the colour picked instead).
const SKETCH: (&str, f32, f32, [f32; 3]) = ("pencil", 4.0, 0.6, [0.48, 0.46, 0.43]);
const DRAW: (&str, f32, f32, [f32; 3]) = ("ink-pen", INK_DIAMETER, 1.0, [0.149, 0.145, 0.137]);
const CORRECT: (&str, f32, f32, [f32; 3]) = ("ink-pen", INK_DIAMETER, 1.0, [0.12, 0.33, 0.80]);
const BIG: (&str, f32, f32, [f32; 3]) = ("opaque-paint", 64.0, 1.0, [0.5; 3]);
const SMALL: (&str, f32, f32, [f32; 3]) = ("opaque-paint", 20.0, 1.0, [0.5; 3]);
const SOFT: (&str, f32, f32, [f32; 3]) = ("soft-brush", 100.0, 0.8, [0.5; 3]);

/// The pens' styles.
#[derive(Debug, Clone, PartialEq)]
pub struct PracticePens {
    sketch: StrokeStyle,
    draw: StrokeStyle,
    correct: StrokeStyle,
    big: StrokeStyle,
    small: StrokeStyle,
    soft: StrokeStyle,
}

impl PracticePens {
    /// The pens on `library`'s built-in files.
    ///
    /// # Errors
    /// `InvalidValue` when a built-in the pens use is missing (a bug the tests catch).
    pub fn of(library: &BrushLibrary) -> Result<Self, CoreError> {
        let style = |(key, size, opacity, [r, g, b]): (&str, f32, f32, [f32; 3])| {
            let brush = library
                .file(&BrushKey::new(key))
                .ok_or(CoreError::InvalidValue {
                    what: "a practice pen's brush",
                })?;
            StrokeStyle::new(
                Arc::clone(brush.recipe()),
                size,
                opacity,
                Color::rgb(r, g, b)?,
            )
        };
        Ok(Self {
            sketch: style(SKETCH)?,
            draw: style(DRAW)?,
            correct: style(CORRECT)?,
            big: style(BIG)?,
            small: style(SMALL)?,
            soft: style(SOFT)?,
        })
    }

    #[must_use]
    pub const fn style(&self, pen: Pen) -> &StrokeStyle {
        match pen {
            Pen::Sketch => &self.sketch,
            Pen::Draw => &self.draw,
            Pen::Correct => &self.correct,
            Pen::Big => &self.big,
            Pen::Small => &self.small,
            Pen::Soft => &self.soft,
        }
    }

    /// `pen`'s style for the next stroke, a painting pen in `colour`.
    pub(crate) fn stroke(&self, pen: Pen, colour: Color) -> StrokeStyle {
        let style = self.style(pen);
        if !pen.paints() {
            return style.clone();
        }
        let recipe = Arc::clone(style.recipe());
        StrokeStyle::new(recipe, style.size(), style.opacity(), colour)
            .unwrap_or_else(|_| style.clone())
    }
}

/// Whether `style` is the sketch pen's: a construction line. Its colour, size and opacity (stored
/// exactly) tell it; no brush of the toolbox starts at them.
pub(crate) fn sketched(style: &StrokeStyle) -> bool {
    let (_, size, opacity, [r, g, b]) = SKETCH;
    style.size() == size
        && style.opacity() == opacity
        && Color::rgb(r, g, b).is_ok_and(|colour| colour == style.color())
}
