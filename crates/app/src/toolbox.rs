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
//! and what comes back. Since S46 the library is the user's too: the brush editor changes,
//! copies, resets and deletes brushes (`toolbox_brushes.rs`), stored with the toolbox; since S47
//! the brushes lie in sets the hand orders, and Recent lists those drawn with last
//! (`toolbox_sets.rs`).

use std::collections::HashMap;
use std::sync::Arc;

use openpaint_core::{Color, CoreError, ShapeParts, StrokeStyle, StrokeTool};

use crate::brush_sets::{BrushSet, BrushSets};
use crate::brushes::{Brush, BrushKey, BrushLibrary};
use crate::edited_brush::EditedBrush;
use crate::editor_state::{BrushChoice, BrushSetting, EditorState};
use crate::palette::Palette;
use crate::pen_buttons::{ButtonMap, HeldTool};
use crate::streamline::StreamLine;
use crate::tool_intent::{BrushProjection, BrushRow, ToolIntent};

#[path = "toolbox_brushes.rs"]
mod brushes;
#[path = "toolbox_restore.rs"]
mod restore;
#[path = "toolbox_sets.rs"]
mod sets;

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

/// One tool's brush and, per brush it changed, size, opacity and StreamLine; any other brush
/// has its defaults.
#[derive(Debug, Clone)]
struct ToolSet {
    current: BrushKey,
    settings: HashMap<BrushKey, [f32; 3]>,
}

impl ToolSet {
    fn on(current: BrushKey) -> Self {
        Self {
            current,
            settings: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Toolbox {
    library: BrushLibrary,
    rows: Arc<[BrushRow]>,
    /// Where each brush lies, and Recent (S47); `set_rows` lists them.
    brush_sets: BrushSets,
    set_rows: Arc<[Vec<BrushRow>; 5]>,
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
    /// The stamps a brush had before the editor made its stroke continuous (S46), which a
    /// spacing brings back while it is edited; never stored.
    kept: Option<(BrushKey, ShapeParts)>,
    /// The names of stored brushes the engine refused at the last restore.
    fell_back: Vec<String>,
    /// The keys a stored set or Recent listed that the last restore dropped.
    dropped: Vec<String>,
}

impl Toolbox {
    /// The brush tool in black; the brush and the eraser on the library's default brush, the
    /// smudge on the soft brush at `SMUDGE_STRENGTH`, every brush at its defaults; no recent
    /// colours, no swatches; the pen's buttons as `ButtonMap::default`.
    #[must_use]
    pub fn new(library: BrushLibrary) -> Self {
        let brush = library.default_key().clone();
        let smudge = match library.find(SMUDGE_BRUSH) {
            Some(at) => library.brushes()[at].key().clone(),
            None => brush.clone(),
        };
        let brush_sets = BrushSets::new(&library);
        Self {
            rows: rows_of(&library),
            set_rows: brush_sets.rows(&library),
            brush_sets,
            library,
            tool: Tool::Brush,
            sets: [
                ToolSet::on(brush.clone()),
                ToolSet::on(brush),
                ToolSet::on(smudge),
            ],
            palette: Palette::default(),
            picking: false,
            preview: None,
            buttons: ButtonMap::default(),
            held: None,
            unsaved: false,
            holding: false,
            kept: None,
            fell_back: Vec::new(),
            dropped: Vec::new(),
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

    /// `tool`'s brush.
    fn brush(&self, tool: Tool) -> &Brush {
        let key = &self.sets[tool.index()].current;
        self.library
            .brush(key)
            .unwrap_or(&self.library.brushes()[0])
    }

    /// What `tool` set for `brush`: its size, opacity and StreamLine, or the brush's defaults.
    fn setting(&self, tool: Tool, brush: &Brush) -> [f32; 3] {
        let set = &self.sets[tool.index()];
        set.settings.get(brush.key()).copied().unwrap_or_else(|| {
            let opacity = match tool {
                Tool::Smudge => SMUDGE_STRENGTH,
                Tool::Brush | Tool::Eraser => brush.opacity(),
            };
            [brush.size().default, opacity, brush.streamline()]
        })
    }

    /// The whole state, for the store: the tool chosen, never one only held.
    #[must_use]
    pub fn state(&self) -> EditorState {
        let choice = |tool: Tool| BrushChoice {
            brush: self.sets[tool.index()].current.as_str().to_owned(),
            brushes: self
                .library
                .brushes()
                .iter()
                .map(|brush| {
                    let [size, opacity, streamline] = self.setting(tool, brush);
                    BrushSetting {
                        key: brush.key().as_str().to_owned(),
                        size,
                        opacity,
                        streamline,
                    }
                })
                .collect(),
        };
        let rgb = |color: &Color| [color.r(), color.g(), color.b()];
        let (sets, recent_brushes) = self.brush_sets.stored(&self.library);
        EditorState {
            tool: self.tool,
            paint: choice(Tool::Brush),
            erase: choice(Tool::Eraser),
            smudge: choice(Tool::Smudge),
            color: rgb(&self.palette.color()),
            recent: self.palette.recent().iter().map(rgb).collect(),
            buttons: self.buttons,
            swatches: self.palette.swatches().iter().map(rgb).collect(),
            library: self.library.stored(),
            sets,
            recent_brushes,
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

    /// The style the next stroke starts with: the brush, size and opacity of the tool the pen
    /// uses now, in the current colour.
    ///
    /// # Errors
    /// `InvalidValue` only if a stored size or opacity escaped the checks (a bug).
    pub fn style(&self) -> Result<StrokeStyle, CoreError> {
        let tool = self.current();
        let brush = self.brush(tool);
        let [size, opacity, _] = self.setting(tool, brush);
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
        let tool = self.current();
        let [_, _, amount] = self.setting(tool, self.brush(tool));
        StreamLine::new(amount).unwrap_or_default()
    }

    #[must_use]
    pub fn projection(&self) -> BrushProjection {
        let tool = self.current();
        let brush = self.brush(tool);
        let [size, opacity, streamline] = self.setting(tool, brush);
        BrushProjection {
            brushes: Arc::clone(&self.rows),
            sets: Arc::clone(&self.set_rows),
            set: self
                .brush_sets
                .set_of(brush.key())
                .unwrap_or(BrushSet::Painting),
            tool,
            current: brush.key().clone(),
            size,
            size_position: brush.size().position(size),
            opacity,
            streamline: StreamLine::new(streamline).unwrap_or_default(),
            color: self.palette.color(),
            recent: self.palette.recent(),
            swatches: self.palette.swatches(),
            picking: self.picking(),
            preview: self.preview,
            buttons: self.buttons,
            edited: Arc::new(self.edited(brush, streamline)),
        }
    }

    /// `brush` as the editor shows it, with `streamline` the tool's on it.
    fn edited(&self, brush: &Brush, streamline: f32) -> EditedBrush {
        let file = self.library.file(brush.key());
        let resettable = file.is_some_and(|file| file != brush || file.streamline() != streamline);
        let recipe = brush.recipe().to_parts();
        let origin = self
            .library
            .origin(brush)
            .map_or_else(|| recipe.clone(), |origin| origin.recipe().to_parts());
        EditedBrush {
            key: brush.key().clone(),
            name: brush.name().to_owned(),
            built_in: brush.is_built_in(),
            resettable,
            recipe,
            size: brush.size(),
            origin,
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

/// The picker's rows of `library`, in its order.
fn rows_of(library: &BrushLibrary) -> Arc<[BrushRow]> {
    library
        .brushes()
        .iter()
        .map(|brush| BrushRow {
            key: brush.key().clone(),
            name: brush.name().to_owned(),
        })
        .collect()
}
