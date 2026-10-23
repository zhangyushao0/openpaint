//! The pens a line exercise's own tools draw with (S76): a light pencil to mark points and draw
//! construction lines (V大's right process: they are never judged and are left out of the
//! aligned fit), the ink pen for the copy, and after Compare the blue pen that corrects it, as on
//! paper. Each is a built-in brush as its file has it (the user's edits never reach them) at a
//! size and colour of its own; the toolbox is not touched.

use std::sync::Arc;

use openpaint_core::{Color, CoreError, StrokeStyle};

use crate::brushes::{BrushKey, BrushLibrary};
use crate::practice::Pen;

/// Each pen's brush, diameter (canvas pixels; a frame is 400), opacity and colour.
const SKETCH: (&str, f32, f32, [f32; 3]) = ("pencil", 4.0, 0.6, [0.48, 0.46, 0.43]);
const DRAW: (&str, f32, f32, [f32; 3]) = ("ink-pen", 5.0, 1.0, [0.149, 0.145, 0.137]);
const CORRECT: (&str, f32, f32, [f32; 3]) = ("ink-pen", 5.0, 1.0, [0.12, 0.33, 0.80]);

/// The three pens' styles.
#[derive(Debug, Clone, PartialEq)]
pub struct PracticePens {
    sketch: StrokeStyle,
    draw: StrokeStyle,
    correct: StrokeStyle,
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
        })
    }

    #[must_use]
    pub const fn style(&self, pen: Pen) -> &StrokeStyle {
        match pen {
            Pen::Sketch => &self.sketch,
            Pen::Draw => &self.draw,
            Pen::Correct => &self.correct,
        }
    }

    /// Whether `style` is the sketch pen's: a construction line. Its colour, size and opacity
    /// (stored exactly) tell it; no brush of the toolbox starts at them.
    pub(crate) fn is_sketch(&self, style: &StrokeStyle) -> bool {
        let sketch = &self.sketch;
        style.color() == sketch.color()
            && style.size() == sketch.size()
            && style.opacity() == sketch.opacity()
    }
}
