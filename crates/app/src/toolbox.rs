//! The toolbox: what the pen does (paint, smudge or erase with a brush), which brush each tool
//! uses at what size, opacity (the smudge's strength) and StreamLine, the colour, and the
//! eyedropper. It belongs to the app, not to one artwork (Procreate's model: the brush stays
//! when another artwork opens). Like Procreate, the smudge and the eraser choose their own
//! brushes from the same library, and every brush remembers its own size, opacity and
//! StreamLine per tool. Stored apart from artworks (ARCHITECTURE section 5, DECISIONS D09): a
//! state that cannot be used falls back to the defaults.

use std::sync::Arc;

use openpaint_core::{Color, CoreError, StrokeStyle, StrokeTool};

use crate::brushes::{BrushKey, BrushLibrary};
use crate::editor_state::{BrushChoice, BrushSetting, EditorState};
use crate::palette::Palette;
use crate::streamline::StreamLine;

/// The lowest opacity the slider gives: a stroke always leaves some paint.
pub const MIN_OPACITY: f32 = 0.01;

/// What the pen does on the canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tool {
    /// Paints with the current colour.
    #[default]
    Brush,
    /// Takes paint away from the active layer.
    Eraser,
    /// Drags the active layer's colour along the pen's path; its opacity is the strength.
    Smudge,
}

impl Tool {
    const fn index(self) -> usize {
        match self {
            Self::Brush => 0,
            Self::Eraser => 1,
            Self::Smudge => 2,
        }
    }
}

/// The smudge's brush at first (draweave's Soft Airbrush), when the library holds it.
const SMUDGE_BRUSH: &str = "soft-brush";
/// The smudge's strength for every brush at first (draweave's 70 %).
pub const SMUDGE_STRENGTH: f32 = 0.7;

/// Changes of the current tool's brush; they apply to the tool the pen uses now.
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
}

/// The tool, the colour and the eyedropper.
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
}

/// One brush as the picker lists it.
#[derive(Debug, Clone, PartialEq)]
pub struct BrushRow {
    pub key: BrushKey,
    pub name: String,
}

/// What the tool buttons, the brush picker, the side bar and the colour panel show.
#[derive(Debug, Clone, PartialEq)]
pub struct BrushProjection {
    /// In picker order.
    pub brushes: Arc<[BrushRow]>,
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
    /// The eyedropper is on: the next contact on the canvas picks a colour.
    pub picking: bool,
    /// While picking, the colour under the contact, which lifting makes the colour.
    pub preview: Option<Color>,
}

/// Slots of a brush's settings: size, opacity, StreamLine.
const SIZE: usize = 0;
const OPACITY: usize = 1;
const STREAMLINE: usize = 2;

/// One tool's brush and, per brush of the library in its order, size, opacity and StreamLine.
#[derive(Debug, Clone)]
struct ToolSet {
    current: usize,
    settings: Vec<[f32; 3]>,
}

#[derive(Debug, Clone)]
pub struct Toolbox {
    library: BrushLibrary,
    rows: Arc<[BrushRow]>,
    tool: Tool,
    /// Brush, eraser, smudge.
    sets: [ToolSet; 3],
    palette: Palette,
    picking: bool,
    /// While picking, the last colour under the contact.
    preview: Option<Color>,
    /// Changed since the state was last handed to the store.
    unsaved: bool,
    /// A slider or the colour wheel is held; the store gets the value it settles on.
    holding: bool,
}

impl Toolbox {
    /// The brush tool in black; the brush and the eraser on the library's default brush, the
    /// smudge on the soft brush at `SMUDGE_STRENGTH`, every brush at its defaults; no recent
    /// colours.
    #[must_use]
    pub fn new(library: BrushLibrary) -> Self {
        let rows = library
            .brushes()
            .iter()
            .map(|brush| BrushRow {
                key: brush.key(),
                name: brush.name().to_owned(),
            })
            .collect();
        let set = ToolSet {
            current: library.default_index(),
            settings: library
                .brushes()
                .iter()
                .map(|brush| [brush.size().default, brush.opacity(), 0.0])
                .collect(),
        };
        let smudge = ToolSet {
            current: library.find(SMUDGE_BRUSH).unwrap_or(set.current),
            settings: library
                .brushes()
                .iter()
                .map(|brush| [brush.size().default, SMUDGE_STRENGTH, 0.0])
                .collect(),
        };
        Self {
            library,
            rows,
            tool: Tool::Brush,
            sets: [set.clone(), set, smudge],
            palette: Palette::default(),
            picking: false,
            preview: None,
            unsaved: false,
            holding: false,
        }
    }

    #[must_use]
    pub const fn library(&self) -> &BrushLibrary {
        &self.library
    }

    /// True while the eyedropper is on.
    #[must_use]
    pub const fn picking(&self) -> bool {
        self.picking
    }

    /// Takes over a stored state. Unknown brushes, values outside a brush's ranges and colours
    /// out of range are skipped (those keep their defaults). Returns whether everything was
    /// taken over.
    pub fn restore(&mut self, state: &EditorState) -> bool {
        self.tool = state.tool;
        let paint = self.restore_set(Tool::Brush, &state.paint);
        let erase = self.restore_set(Tool::Eraser, &state.erase);
        let smudge = self.restore_set(Tool::Smudge, &state.smudge);
        let colors = self.palette.restore(state.color, &state.recent);
        paint && erase && smudge && colors
    }

    fn restore_set(&mut self, tool: Tool, choice: &BrushChoice) -> bool {
        let mut whole = true;
        let set = &mut self.sets[tool.index()];
        for setting in &choice.brushes {
            let Some(index) = self.library.find(&setting.key) else {
                whole = false;
                continue;
            };
            let brush = &self.library.brushes()[index];
            let opacity = (MIN_OPACITY..=1.0).contains(&setting.opacity);
            let streamline = StreamLine::new(setting.streamline).is_some();
            if brush.size().contains(setting.size) && opacity && streamline {
                set.settings[index] = [setting.size, setting.opacity, setting.streamline];
            } else {
                whole = false;
            }
        }
        if !choice.brush.is_empty() {
            match self.library.find(&choice.brush) {
                Some(index) => set.current = index,
                None => whole = false,
            }
        }
        whole
    }

    /// The whole state, for the store.
    #[must_use]
    pub fn state(&self) -> EditorState {
        let choice = |set: &ToolSet| BrushChoice {
            brush: self.rows[set.current].key.as_str().to_owned(),
            brushes: self
                .library
                .brushes()
                .iter()
                .zip(&set.settings)
                .map(|(brush, [size, opacity, streamline])| BrushSetting {
                    key: brush.key().as_str().to_owned(),
                    size: *size,
                    opacity: *opacity,
                    streamline: *streamline,
                })
                .collect(),
        };
        let rgb = |color: &Color| [color.r(), color.g(), color.b()];
        EditorState {
            tool: self.tool,
            paint: choice(&self.sets[0]),
            erase: choice(&self.sets[1]),
            smudge: choice(&self.sets[2]),
            color: rgb(&self.palette.color()),
            recent: self.palette.recent().iter().map(rgb).collect(),
        }
    }

    /// Applies `intent` to the current tool. Choosing a brush the library does not hold, or a
    /// value that is not a number, changes nothing.
    pub fn dispatch(&mut self, intent: &BrushIntent) {
        let tool = self.tool.index();
        match *intent {
            BrushIntent::Choose(key) => {
                if let Some(index) = self.library.find(key.as_str()) {
                    let set = &mut self.sets[tool];
                    self.unsaved |= index != set.current;
                    set.current = index;
                    self.set_picking(false);
                }
            }
            BrushIntent::Size { position, settled } if position.is_finite() => {
                let current = self.sets[tool].current;
                let size = self.library.brushes()[current].size().at(position);
                self.set(SIZE, size, settled);
            }
            BrushIntent::Opacity { opacity, settled } if opacity.is_finite() => {
                self.set(OPACITY, opacity.clamp(MIN_OPACITY, 1.0), settled);
            }
            BrushIntent::StreamLine { amount, settled } if amount.is_finite() => {
                self.set(STREAMLINE, amount.clamp(0.0, 1.0), settled);
            }
            BrushIntent::Size { .. }
            | BrushIntent::Opacity { .. }
            | BrushIntent::StreamLine { .. } => {}
        }
    }

    /// Applies a tool, colour or eyedropper intent.
    pub fn dispatch_tool(&mut self, intent: &ToolIntent) {
        match *intent {
            ToolIntent::Use(tool) => {
                self.unsaved |= tool != self.tool;
                self.tool = tool;
                self.set_picking(false);
            }
            ToolIntent::Eyedropper(on) => self.set_picking(on),
            ToolIntent::Color { color, settled } => {
                self.unsaved |= self.palette.set(color, settled);
                self.holding = !settled;
                if settled {
                    self.set_picking(false);
                }
            }
            ToolIntent::Preview(color) if self.picking => self.preview = Some(color),
            ToolIntent::Pick => {
                if let Some(color) = self.preview.filter(|_| self.picking) {
                    self.unsaved |= self.palette.set(color, true);
                    self.set_picking(false);
                }
            }
            ToolIntent::Preview(_) => {}
        }
    }

    /// The eyedropper on or off; either way it starts without a preview.
    fn set_picking(&mut self, on: bool) {
        self.picking = on;
        self.preview = None;
    }

    fn set(&mut self, slot: usize, value: f32, settled: bool) {
        let set = &mut self.sets[self.tool.index()];
        let setting = &mut set.settings[set.current][slot];
        self.unsaved |= *setting != value;
        *setting = value;
        self.holding = !settled;
    }

    /// The style the next stroke starts with: the current tool's brush, size and opacity, in
    /// the current colour.
    ///
    /// # Errors
    /// `InvalidValue` only if a stored size or opacity escaped the checks (a bug).
    pub fn style(&self) -> Result<StrokeStyle, CoreError> {
        let set = &self.sets[self.tool.index()];
        let brush = &self.library.brushes()[set.current];
        let [size, opacity, _] = set.settings[set.current];
        let style = StrokeStyle::new(
            Arc::clone(brush.recipe()),
            size,
            opacity,
            self.palette.color(),
        )?;
        Ok(style.with_tool(match self.tool {
            Tool::Brush => StrokeTool::Paint,
            Tool::Eraser => StrokeTool::Erase,
            Tool::Smudge => StrokeTool::Smudge,
        }))
    }

    /// How the next stroke follows the pen: the current tool's brush's StreamLine.
    #[must_use]
    pub fn streamline(&self) -> StreamLine {
        let set = &self.sets[self.tool.index()];
        StreamLine::new(set.settings[set.current][STREAMLINE]).unwrap_or_default()
    }

    #[must_use]
    pub fn projection(&self) -> BrushProjection {
        let set = &self.sets[self.tool.index()];
        let brush = &self.library.brushes()[set.current];
        let [size, opacity, _] = set.settings[set.current];
        BrushProjection {
            brushes: Arc::clone(&self.rows),
            tool: self.tool,
            current: brush.key(),
            size,
            size_position: brush.size().position(size),
            opacity,
            streamline: self.streamline(),
            color: self.palette.color(),
            recent: self.palette.recent(),
            picking: self.picking,
            preview: self.preview,
        }
    }

    /// The state to store now, if it changed since it was last handed out and nothing is held.
    pub fn save_request(&mut self) -> Option<EditorState> {
        if !self.unsaved || self.holding {
            return None;
        }
        self.unsaved = false;
        Some(self.state())
    }
}
