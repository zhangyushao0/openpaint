//! What the UI asks of the toolbox (`Toolbox::dispatch`, `Toolbox::dispatch_tool`) and what it
//! shows of it (`BrushProjection`).

use std::sync::Arc;

use openpaint_core::Color;

use crate::brush_edit::BrushChange;
use crate::brushes::BrushKey;
use crate::edited_brush::EditedBrush;
use crate::pen_buttons::{ButtonAction, ButtonMap, HeldTool, PenButton};
use crate::streamline::StreamLine;
use crate::toolbox::Tool;

/// Changes of the current tool's brush; they apply to the tool the pen uses now. The brush
/// editor's (S46) change the brush itself, for every tool, from the next stroke on; strokes
/// drawn keep their recipes, and none is a history step.
#[derive(Debug, Clone, PartialEq)]
pub enum BrushIntent {
    /// Draw with this brush from the next stroke on, at the size and opacity it had last.
    Choose(BrushKey),
    /// `position` in `0..=1` along the size slider. `settled` when the hand lifts.
    Size { position: f32, settled: bool },
    /// `opacity` in `0..=1` from the slider; below `MIN_OPACITY` counts as it.
    Opacity { opacity: f32, settled: bool },
    /// The StreamLine `amount` in `0..=1` from the brush panel's slider, from the next stroke on.
    StreamLine { amount: f32, settled: bool },
    /// One of the brush's settings; `settled` when the hand lifts.
    Edit { change: BrushChange, settled: bool },
    /// The brush's name (trimmed, 1 to `MAX_BRUSH_NAME` characters; anything else is ignored).
    Rename(String),
    /// A copy "<name> copy" right after the brush, chosen, with the same values.
    Duplicate,
    /// A built-in as its file has it again, the tool's StreamLine on it too.
    Reset,
    /// A copy goes; its source is chosen where it was.
    Delete,
}

/// The tool, the colour, the eyedropper, the pen's buttons and the palette.
#[derive(Debug, Clone, PartialEq)]
pub enum ToolIntent {
    /// The pen paints or erases from the next stroke on; ends the eyedropper.
    Use(Tool),
    /// The next contact on the canvas picks a colour instead of drawing (on), or draws again
    /// (off).
    Eyedropper(bool),
    /// Paint with `color`. `settled` when the hand lifts from the wheel or a recent colour: the
    /// colour joins the recent ones and the eyedropper ends.
    Color { color: Color, settled: bool },
    /// While the eyedropper is on: the colour under the picking contact now (the loupe's top
    /// half). The current colour stays until the contact lifts.
    Preview(Color),
    /// The picking contact lifted: the last colour previewed, not one where the contact drifted
    /// to, becomes the colour, joins the recent ones and ends the eyedropper. With none
    /// previewed the eyedropper stays on.
    Pick,
    /// A pen button (or the eraser end) holds this tool for the pen's contacts from now on;
    /// `None`: the tool chosen before comes back (S41). Never stored.
    Hold(Option<HeldTool>),
    /// A pen button switches between the brush and the eraser: the eraser from any other tool,
    /// the brush from the eraser.
    Switch,
    /// What `button` does from now on.
    SetButton {
        button: PenButton,
        action: ButtonAction,
    },
    /// The current colour joins the palette's end (nothing when it is there or full).
    AddSwatch,
    /// `color` leaves the palette.
    RemoveSwatch(Color),
}

/// One brush as the picker lists it.
#[derive(Debug, Clone, PartialEq)]
pub struct BrushRow {
    pub key: BrushKey,
    pub name: String,
}

/// What the tool buttons, the brush picker, the side bar, the colour panel and the pen buttons'
/// page show.
#[derive(Debug, Clone, PartialEq)]
pub struct BrushProjection {
    /// In picker order.
    pub brushes: Arc<[BrushRow]>,
    /// The tool the pen uses now: a held eraser shows as the eraser.
    pub tool: Tool,
    /// The current tool's brush.
    pub current: BrushKey,
    /// The current brush's diameter in canvas pixels, and where it sits on the size slider.
    pub size: f32,
    pub size_position: f32,
    /// In `MIN_OPACITY..=1`.
    pub opacity: f32,
    /// The current brush's StreamLine for the current tool.
    pub streamline: StreamLine,
    pub color: Color,
    /// Newest first.
    pub recent: Arc<[Color]>,
    /// The palette's swatches, in the order they were added.
    pub swatches: Arc<[Color]>,
    /// The eyedropper is on (or held by a pen button): the next contact on the canvas picks.
    pub picking: bool,
    /// While picking, the colour under the contact, which lifting makes the colour.
    pub preview: Option<Color>,
    /// What each pen button does.
    pub buttons: ButtonMap,
    /// The current tool's brush as its editor shows it (S46).
    pub edited: Arc<EditedBrush>,
}
