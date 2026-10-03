//! The toolbox: which brush draws, at what size and opacity. It belongs to the app, not to one
//! artwork (Procreate's model: the brush stays when another artwork opens), and every brush
//! remembers its own size and opacity. Stored apart from artworks (ARCHITECTURE section 5,
//! DECISIONS D09): a state that cannot be used falls back to the brushes' defaults.

use std::sync::Arc;

use openpaint_core::{Color, CoreError, StrokeStyle};

use crate::brushes::{BrushKey, BrushLibrary};

/// The lowest opacity the slider gives: a stroke always leaves some paint.
pub const MIN_OPACITY: f32 = 0.01;

#[derive(Debug, Clone, PartialEq)]
pub enum BrushIntent {
    /// Draw with this brush from the next stroke on, at the size and opacity it had last.
    Choose(BrushKey),
    /// `position` in `0..=1` along the size slider. `settled` when the hand lifts.
    Size { position: f32, settled: bool },
    /// `opacity` in `0..=1` from the slider; below `MIN_OPACITY` counts as it.
    Opacity { opacity: f32, settled: bool },
}

/// One brush as the picker lists it.
#[derive(Debug, Clone, PartialEq)]
pub struct BrushRow {
    pub key: BrushKey,
    pub name: String,
}

/// What the brush picker and the side bar show.
#[derive(Debug, Clone, PartialEq)]
pub struct BrushProjection {
    /// In picker order.
    pub brushes: Arc<[BrushRow]>,
    pub current: BrushKey,
    /// The current brush's diameter in canvas pixels, and where it sits on the size slider.
    pub size: f32,
    pub size_position: f32,
    /// In `MIN_OPACITY..=1`.
    pub opacity: f32,
}

/// The toolbox as plain data for the store: the current brush and each brush's size and
/// opacity, by key.
#[derive(Debug, Clone, PartialEq)]
pub struct EditorState {
    pub brush: String,
    pub brushes: Vec<BrushSetting>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BrushSetting {
    pub key: String,
    pub size: f32,
    pub opacity: f32,
}

#[derive(Debug, Clone)]
pub struct Toolbox {
    library: BrushLibrary,
    rows: Arc<[BrushRow]>,
    current: usize,
    /// Per brush of the library, in its order: size and opacity.
    settings: Vec<[f32; 2]>,
    /// Changed since the state was last handed to the store.
    unsaved: bool,
    /// A slider is held; the store gets the value it settles on.
    holding: bool,
}

impl Toolbox {
    /// Every brush at its defaults, the library's default brush current.
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
        let settings = library
            .brushes()
            .iter()
            .map(|brush| [brush.size().default, brush.opacity()])
            .collect();
        Self {
            current: library.default_index(),
            library,
            rows,
            settings,
            unsaved: false,
            holding: false,
        }
    }

    #[must_use]
    pub const fn library(&self) -> &BrushLibrary {
        &self.library
    }

    /// Takes over a stored state. Unknown brushes and values outside a brush's ranges are
    /// skipped (those keep their defaults). Returns whether everything was taken over.
    pub fn restore(&mut self, state: &EditorState) -> bool {
        let mut whole = true;
        for setting in &state.brushes {
            let Some(index) = self.library.find(&setting.key) else {
                whole = false;
                continue;
            };
            let brush = &self.library.brushes()[index];
            let opacity = (MIN_OPACITY..=1.0).contains(&setting.opacity);
            if brush.size().contains(setting.size) && opacity {
                self.settings[index] = [setting.size, setting.opacity];
            } else {
                whole = false;
            }
        }
        match self.library.find(&state.brush) {
            Some(index) => self.current = index,
            None => whole = false,
        }
        whole
    }

    /// The whole state, for the store.
    #[must_use]
    pub fn state(&self) -> EditorState {
        let brushes = self
            .library
            .brushes()
            .iter()
            .zip(&self.settings)
            .map(|(brush, [size, opacity])| BrushSetting {
                key: brush.key().as_str().to_owned(),
                size: *size,
                opacity: *opacity,
            })
            .collect();
        EditorState {
            brush: self.rows[self.current].key.as_str().to_owned(),
            brushes,
        }
    }

    /// Applies `intent`. Choosing a brush the library does not hold, or a value that is not a
    /// number, changes nothing.
    pub fn dispatch(&mut self, intent: &BrushIntent) {
        match *intent {
            BrushIntent::Choose(key) => {
                if let Some(index) = self.library.find(key.as_str()) {
                    self.unsaved |= index != self.current;
                    self.current = index;
                }
            }
            BrushIntent::Size { position, settled } if position.is_finite() => {
                let size = self.library.brushes()[self.current].size().at(position);
                self.set(0, size, settled);
            }
            BrushIntent::Opacity { opacity, settled } if opacity.is_finite() => {
                self.set(1, opacity.clamp(MIN_OPACITY, 1.0), settled);
            }
            BrushIntent::Size { .. } | BrushIntent::Opacity { .. } => {}
        }
    }

    fn set(&mut self, slot: usize, value: f32, settled: bool) {
        let setting = &mut self.settings[self.current][slot];
        self.unsaved |= *setting != value;
        *setting = value;
        self.holding = !settled;
    }

    /// The style the next stroke starts with: the current brush in black.
    ///
    /// # Errors
    /// `InvalidValue` only if a stored size or opacity escaped the checks (a bug).
    pub fn style(&self) -> Result<StrokeStyle, CoreError> {
        let brush = &self.library.brushes()[self.current];
        let [size, opacity] = self.settings[self.current];
        StrokeStyle::new(Arc::clone(brush.recipe()), size, opacity, Color::BLACK)
    }

    #[must_use]
    pub fn projection(&self) -> BrushProjection {
        let brush = &self.library.brushes()[self.current];
        let [size, opacity] = self.settings[self.current];
        BrushProjection {
            brushes: Arc::clone(&self.rows),
            current: brush.key(),
            size,
            size_position: brush.size().position(size),
            opacity,
        }
    }

    /// The state to store now, if it changed since it was last handed out and no slider is
    /// held.
    pub fn save_request(&mut self) -> Option<EditorState> {
        if !self.unsaved || self.holding {
            return None;
        }
        self.unsaved = false;
        Some(self.state())
    }
}
