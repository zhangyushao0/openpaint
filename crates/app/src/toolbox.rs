//! The toolbox: what the pen does (paint, smudge or erase with a brush), which brush each tool
//! uses at what size, opacity (the smudge's strength) and StreamLine, the colour, the palette,
//! the eyedropper and what the pen's buttons do. It belongs to the app, not to one artwork
//! (Procreate's model: the brush stays when another artwork opens). Like Procreate, the smudge
//! and the eraser choose their own brushes from the same library, and every brush remembers its
//! own size, opacity and StreamLine per tool. Stored apart from artworks (ARCHITECTURE section
//! 5, DECISIONS D09): a state that cannot be used falls back to the defaults.
//!
//! A pen button (or the eraser end) may hold the eraser or the eyedropper (S41, D68): while
//! held the pen uses it, the projection shows it, and the tool chosen before is what is stored
//! and what comes back.

use std::sync::Arc;

use openpaint_core::{Color, CoreError, StrokeStyle, StrokeTool};

use crate::brushes::BrushLibrary;
use crate::editor_state::{BrushChoice, BrushSetting, EditorState};
use crate::palette::Palette;
use crate::pen_buttons::{ButtonMap, HeldTool};
use crate::streamline::StreamLine;
use crate::tool_intent::{BrushIntent, BrushProjection, BrushRow, ToolIntent};

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
    /// The tool chosen.
    tool: Tool,
    /// Brush, eraser, smudge.
    sets: [ToolSet; 3],
    palette: Palette,
    picking: bool,
    /// While picking, the last colour under the contact.
    preview: Option<Color>,
    buttons: ButtonMap,
    /// The tool a pen button or the eraser end holds now; never stored.
    held: Option<HeldTool>,
    /// Changed since the state was last handed to the store.
    unsaved: bool,
    /// A slider or the colour wheel is held; the store gets the value it settles on.
    holding: bool,
}

impl Toolbox {
    /// The brush tool in black; the brush and the eraser on the library's default brush, the
    /// smudge on the soft brush at `SMUDGE_STRENGTH`, every brush at its defaults; no recent
    /// colours, no swatches; the pen's buttons as `ButtonMap::default`.
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
        let settings = |opacity: Option<f32>| {
            library
                .brushes()
                .iter()
                .map(|brush| {
                    [
                        brush.size().default,
                        opacity.unwrap_or(brush.opacity()),
                        brush.streamline(),
                    ]
                })
                .collect()
        };
        let set = ToolSet {
            current: library.default_index(),
            settings: settings(None),
        };
        let smudge = ToolSet {
            current: library.find(SMUDGE_BRUSH).unwrap_or(set.current),
            settings: settings(Some(SMUDGE_STRENGTH)),
        };
        Self {
            library,
            rows,
            tool: Tool::Brush,
            sets: [set.clone(), set, smudge],
            palette: Palette::default(),
            picking: false,
            preview: None,
            buttons: ButtonMap::default(),
            held: None,
            unsaved: false,
            holding: false,
        }
    }

    #[must_use]
    pub const fn library(&self) -> &BrushLibrary {
        &self.library
    }

    /// True while the eyedropper is on, chosen or held by a pen button.
    #[must_use]
    pub fn picking(&self) -> bool {
        self.picking || self.held == Some(HeldTool::Eyedropper)
    }

    /// The tool the pen uses now: a held eraser, else the tool chosen.
    fn current(&self) -> Tool {
        if self.held == Some(HeldTool::Eraser) {
            Tool::Eraser
        } else {
            self.tool
        }
    }

    /// Takes over a stored state. Unknown brushes, values outside a brush's ranges and colours
    /// out of range are skipped (those keep their defaults). Returns whether everything was
    /// taken over.
    pub fn restore(&mut self, state: &EditorState) -> bool {
        self.tool = state.tool;
        self.buttons = state.buttons;
        let paint = self.restore_set(Tool::Brush, &state.paint);
        let erase = self.restore_set(Tool::Eraser, &state.erase);
        let smudge = self.restore_set(Tool::Smudge, &state.smudge);
        let colors = self
            .palette
            .restore(state.color, &state.recent, &state.swatches);
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

    /// The whole state, for the store: the tool chosen, never one only held.
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
            buttons: self.buttons,
            swatches: self.palette.swatches().iter().map(rgb).collect(),
        }
    }

    /// Applies `intent` to the tool the pen uses now. Choosing a brush the library does not
    /// hold, or a value that is not a number, changes nothing.
    pub fn dispatch(&mut self, intent: &BrushIntent) {
        let tool = self.current().index();
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

    /// Applies a tool, colour, eyedropper, pen button or palette intent.
    pub fn dispatch_tool(&mut self, intent: &ToolIntent) {
        match *intent {
            ToolIntent::Use(tool) => self.choose(tool),
            ToolIntent::Switch => self.choose(match self.tool {
                Tool::Eraser => Tool::Brush,
                Tool::Brush | Tool::Smudge => Tool::Eraser,
            }),
            ToolIntent::Eyedropper(on) => self.set_picking(on),
            ToolIntent::Color { color, settled } => {
                self.unsaved |= self.palette.set(color, settled);
                self.holding = !settled;
                if settled {
                    self.set_picking(false);
                }
            }
            ToolIntent::Preview(color) if self.picking() => self.preview = Some(color),
            ToolIntent::Preview(_) => {}
            ToolIntent::Pick => {
                if let Some(color) = self.preview.filter(|_| self.picking()) {
                    self.unsaved |= self.palette.set(color, true);
                    self.set_picking(false);
                }
            }
            ToolIntent::Hold(held) => {
                self.held = held;
                self.preview = None;
            }
            ToolIntent::SetButton { button, action } => {
                let buttons = self.buttons.with(button, action);
                self.unsaved |= buttons != self.buttons;
                self.buttons = buttons;
            }
            ToolIntent::AddSwatch => self.unsaved |= self.palette.add_swatch(),
            ToolIntent::RemoveSwatch(color) => self.unsaved |= self.palette.remove_swatch(color),
        }
    }

    fn choose(&mut self, tool: Tool) {
        self.unsaved |= tool != self.tool;
        self.tool = tool;
        self.set_picking(false);
    }

    /// The eyedropper on or off; either way it starts without a preview.
    fn set_picking(&mut self, on: bool) {
        self.picking = on;
        self.preview = None;
    }

    fn set(&mut self, slot: usize, value: f32, settled: bool) {
        let set = &mut self.sets[self.current().index()];
        let setting = &mut set.settings[set.current][slot];
        self.unsaved |= *setting != value;
        *setting = value;
        self.holding = !settled;
    }

    /// The style the next stroke starts with: the brush, size and opacity of the tool the pen
    /// uses now, in the current colour.
    ///
    /// # Errors
    /// `InvalidValue` only if a stored size or opacity escaped the checks (a bug).
    pub fn style(&self) -> Result<StrokeStyle, CoreError> {
        let tool = self.current();
        let set = &self.sets[tool.index()];
        let brush = &self.library.brushes()[set.current];
        let [size, opacity, _] = set.settings[set.current];
        let style = StrokeStyle::new(
            Arc::clone(brush.recipe()),
            size,
            opacity,
            self.palette.color(),
        )?;
        Ok(style.with_tool(match tool {
            Tool::Brush => StrokeTool::Paint,
            Tool::Eraser => StrokeTool::Erase,
            Tool::Smudge => StrokeTool::Smudge,
        }))
    }

    /// How the next stroke follows the pen: the brush's StreamLine of the tool used now.
    #[must_use]
    pub fn streamline(&self) -> StreamLine {
        let set = &self.sets[self.current().index()];
        StreamLine::new(set.settings[set.current][STREAMLINE]).unwrap_or_default()
    }

    #[must_use]
    pub fn projection(&self) -> BrushProjection {
        let tool = self.current();
        let set = &self.sets[tool.index()];
        let brush = &self.library.brushes()[set.current];
        let [size, opacity, _] = set.settings[set.current];
        BrushProjection {
            brushes: Arc::clone(&self.rows),
            tool,
            current: brush.key(),
            size,
            size_position: brush.size().position(size),
            opacity,
            streamline: self.streamline(),
            color: self.palette.color(),
            recent: self.palette.recent(),
            swatches: self.palette.swatches(),
            picking: self.picking(),
            preview: self.preview,
            buttons: self.buttons,
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
