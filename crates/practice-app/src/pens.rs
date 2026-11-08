//! The pens an exercise's own tools draw and paint with (S76, S82, S100). For lines and shapes: a
//! light pencil to mark points and draw construction lines (V大's right process: they are never
//! judged and are left out of the aligned fit), the ink pen for the copy, and after Compare the
//! blue pen that corrects it, as on paper. For a division and a whole copy's division (S95), the
//! ink pen in V大's three colours: the main lines red, the second level green, the third blue; a
//! stroke's level is read back from them.
//!
//! For painting, after V大's brushes (S100; he keeps opacity and flow at 100 % and controls them by
//! hand, early on with little pressure and little texture): Big and Small are the hard tip, round,
//! a hard edge, the colour picked at full strength at any pressure (pressure sets only its size, a
//! little), for blocks: Big for the big ones, Small for edges and small areas; Soft is the
//! airbrush, only where the picture's own edge or gradient is soft. Value masses' Dark, Middle and
//! Light (S96) are the hard tip in their three greys, a size for big masses.
//!
//! No pen is the toolbox's brush: a user's brush edits never reach them, and a stroke keeps its
//! recipe's snapshot, so strokes drawn before draw as they did. The hard tip is the toolbox's
//! Hard Paint file (S104), one definition, so the practice and the toolbox paint blocks alike; the
//! others are files of their own (`app/pens/`). A long colour copy is painted with the toolbox's
//! own brushes, opening on Hard Paint at a size for its frame (`copy_brush`).

use std::sync::Arc;

use openpaint_core::{Color, CoreError, MAX_BRUSH_SIZE, StrokeStyle};
use openpaint_practice::{FRAME, Level, VALUE_PEN, Value};

use crate::projection::Pen;
use crate::sheet::Practising;
use openpaint_app::{BrushKey, BrushLibrary, HARD_PAINT, HARD_PAINT_FILE};

/// The Draw and Correct pens' diameter, canvas pixels: Compare lays the original's lines over the
/// copy as thick (S88), so an error of a pen's width shows.
pub const INK_DIAMETER: f32 = 5.0;

/// The pens' recipes, compiled into the app: the hard tip Hard Paint's file, the others their own.
const RECIPES: [(&str, &str); 4] = [
    ("sketch", include_str!("../pens/sketch.toml")),
    ("ink", include_str!("../pens/ink.toml")),
    HARD_PAINT_FILE,
    ("soft-tip", include_str!("../pens/soft-tip.toml")),
];

/// Each pen's recipe, diameter (canvas pixels; an exercise's frame is 400 on its longer side),
/// opacity and colour (a painting pen takes the colour picked instead).
const SKETCH: (&str, f32, f32, [f32; 3]) = ("sketch", 4.0, 0.6, [0.48, 0.46, 0.43]);
const DRAW: (&str, f32, f32, [f32; 3]) = ("ink", INK_DIAMETER, 1.0, [0.149, 0.145, 0.137]);
const CORRECT: (&str, f32, f32, [f32; 3]) = ("ink", INK_DIAMETER, 1.0, [0.12, 0.33, 0.80]);
const BIG: (&str, f32, f32, [f32; 3]) = (HARD_PAINT, 64.0, 1.0, [0.5; 3]);
const SMALL: (&str, f32, f32, [f32; 3]) = (HARD_PAINT, 20.0, 1.0, [0.5; 3]);
const SOFT: (&str, f32, f32, [f32; 3]) = ("soft-tip", 100.0, 0.8, [0.5; 3]);
/// Value masses' pens: the hard tip in each value's grey, as wide as Compare's tolerance
/// (`VALUE_PEN` of a frame: 24 pixels), big for masses.
const VALUE: (&str, f32, f32) = (HARD_PAINT, (VALUE_PEN * FRAME) as f32, 1.0);

/// What a long colour copy opens on (S104): the toolbox's Hard Paint, as large to the copy's frame
/// as Big is to an exercise's (256 pixels on its 1,600-pixel frame); none on any other sheet. The
/// copy is painted with the app's own brushes, one way of painting, so its hand starts with the
/// big blocks.
#[must_use]
pub fn copy_brush((record, _, sheet, _): &Practising) -> Option<(BrushKey, f32)> {
    let frame = sheet.pairs.first().filter(|_| record.is_project())?.copy;
    let (_, big, _, _) = BIG;
    let size = (big * (frame.unit() / FRAME) as f32).min(MAX_BRUSH_SIZE);
    Some((BrushKey::new(HARD_PAINT), size))
}

/// The pens' styles.
#[derive(Debug, Clone, PartialEq)]
pub struct PracticePens {
    sketch: StrokeStyle,
    draw: StrokeStyle,
    correct: StrokeStyle,
    big: StrokeStyle,
    small: StrokeStyle,
    soft: StrokeStyle,
    /// A division's levels' (S95): the ink pen as Draw's, in each level's colour.
    levels: [StrokeStyle; 3],
    /// Value masses' greys (S96), dark, middle, light.
    values: [StrokeStyle; 3],
}

/// A division's level's colour as a stroke holds it.
fn level_colour(level: Level) -> Result<Color, CoreError> {
    let [r, g, b] = level.colour().map(|c| f32::from(c) / 255.0);
    Color::rgb(r, g, b)
}

/// A value's grey as a stroke or a fill holds it (S96).
///
/// # Errors
/// Never for the three greys.
pub fn value_colour(value: Value) -> Result<Color, CoreError> {
    let [r, g, b] = value.grey().map(|c| f32::from(c) / 255.0);
    Color::rgb(r, g, b)
}

impl PracticePens {
    /// The pens on their own recipes.
    ///
    /// # Errors
    /// `InvalidValue` when a pen's recipe file is broken (a bug the tests catch).
    pub fn built_in() -> Result<Self, CoreError> {
        let what = "a practice pen's recipe";
        let recipes = BrushLibrary::from_files(&RECIPES, "ink")
            .map_err(|_| CoreError::InvalidValue { what })?;
        let style = |(key, size, opacity, [r, g, b]): (&str, f32, f32, [f32; 3])| {
            let pen =
                (recipes.file(&BrushKey::new(key))).ok_or(CoreError::InvalidValue { what })?;
            let recipe = Arc::clone(pen.recipe());
            StrokeStyle::new(recipe, size, opacity, Color::rgb(r, g, b)?)
        };
        let draw = style(DRAW)?;
        let level = |level: Level| {
            let recipe = Arc::clone(draw.recipe());
            StrokeStyle::new(recipe, draw.size(), draw.opacity(), level_colour(level)?)
        };
        let (key, size, opacity) = VALUE;
        let grey = |value: Value| {
            let rgb = value.grey().map(|c| f32::from(c) / 255.0);
            style((key, size, opacity, rgb))
        };
        Ok(Self {
            sketch: style(SKETCH)?,
            correct: style(CORRECT)?,
            big: style(BIG)?,
            small: style(SMALL)?,
            soft: style(SOFT)?,
            levels: [
                level(Level::Main)?,
                level(Level::Second)?,
                level(Level::Third)?,
            ],
            values: [
                grey(Value::Dark)?,
                grey(Value::Middle)?,
                grey(Value::Light)?,
            ],
            draw,
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
            Pen::Main => &self.levels[0],
            Pen::Second => &self.levels[1],
            Pen::Third => &self.levels[2],
            Pen::Dark => &self.values[0],
            Pen::Middle => &self.values[1],
            Pen::Light => &self.values[2],
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

/// The division's level `style` draws, if it is a level's pen's (S95): its colour, size and
/// opacity (stored exactly) tell it.
pub(crate) fn levelled(style: &StrokeStyle) -> Option<Level> {
    let (_, size, opacity, _) = DRAW;
    let pen = style.size() == size && style.opacity() == opacity;
    let colour = |level: &Level| level_colour(*level).is_ok_and(|c| c == style.color());
    Level::ALL.into_iter().find(|level| pen && colour(level))
}
