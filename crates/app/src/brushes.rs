//! The brush library: the brushes the picker offers, each a recipe with its name and the
//! ranges of its size and opacity sliders. The built-in brushes are TOML files compiled into
//! the app (`crates/app/brushes/`, format in `recipe.rs`); since S46 the user edits them and
//! makes copies (`brush_library.rs`), stored with the toolbox.

use std::sync::Arc;

use openpaint_core::{BrushRecipe, MAX_BRUSH_SIZE};

use crate::recipe::{RecipeError, parse};

/// The built-in brushes in picker order, by job (sketch, ink, line, paint, texture): key, file.
const BUILT_IN: [(&str, &str); 10] = [
    ("pencil", include_str!("../brushes/pencil.toml")),
    ("ink-pen", include_str!("../brushes/ink-pen.toml")),
    ("inker", include_str!("../brushes/inker.toml")),
    ("monoline", include_str!("../brushes/monoline.toml")),
    ("hard-round", include_str!("../brushes/hard-round.toml")),
    ("soft-brush", include_str!("../brushes/soft-brush.toml")),
    (
        "textured-round",
        include_str!("../brushes/textured-round.toml"),
    ),
    ("flat", include_str!("../brushes/flat.toml")),
    ("opaque-paint", include_str!("../brushes/opaque-paint.toml")),
    ("speckle", include_str!("../brushes/speckle.toml")),
];
/// The brush a first start draws with: the round brush of every stroke before recipes.
const DEFAULT_BRUSH: &str = "ink-pen";

/// Names a brush for its whole life: a built-in's file key, or the key a copy was given
/// (`copy-<n>`). Stored in the editor state; cloning shares the text.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BrushKey(Arc<str>);

impl BrushKey {
    #[must_use]
    pub fn new(key: &str) -> Self {
        Self(Arc::from(key))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The diameters the size slider spans and where it starts, in canvas pixels:
/// `0 < min <= default <= max <= MAX_BRUSH_SIZE`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SizeRange {
    pub min: f32,
    pub default: f32,
    pub max: f32,
}

impl SizeRange {
    /// `None` unless `0 < min <= default <= max <= MAX_BRUSH_SIZE`.
    #[must_use]
    pub fn new(min: f32, default: f32, max: f32) -> Option<Self> {
        let ordered = 0.0 < min && min <= default && default <= max && max <= MAX_BRUSH_SIZE;
        ordered.then_some(Self { min, default, max })
    }

    /// The diameter at `position` (`0..=1`) along the slider: the square of the position
    /// spreads small sizes over more of the slider than large ones.
    #[must_use]
    pub fn at(&self, position: f32) -> f32 {
        let position = position.clamp(0.0, 1.0);
        (self.min + (self.max - self.min) * position * position).clamp(self.min, self.max)
    }

    /// Where `size` sits on the slider, `0..=1`.
    #[must_use]
    pub fn position(&self, size: f32) -> f32 {
        if self.max <= self.min {
            return 1.0;
        }
        ((size - self.min) / (self.max - self.min))
            .clamp(0.0, 1.0)
            .sqrt()
    }

    pub(crate) fn contains(&self, size: f32) -> bool {
        (self.min..=self.max).contains(&size)
    }

    /// `size` moved into the range.
    pub(crate) fn clamp(&self, size: f32) -> f32 {
        size.clamp(self.min, self.max)
    }
}

/// One brush of the library.
#[derive(Debug, Clone, PartialEq)]
pub struct Brush {
    pub(crate) key: BrushKey,
    pub(crate) name: String,
    pub(crate) recipe: Arc<BrushRecipe>,
    pub(crate) size: SizeRange,
    /// The opacity the slider starts at, in `(0, 1]`.
    pub(crate) opacity: f32,
    /// The StreamLine every tool starts with on this brush, in `0..=1`.
    pub(crate) streamline: f32,
    /// A copy's brush it was made from (S46); `None` for a built-in.
    pub(crate) source: Option<BrushKey>,
}

impl Brush {
    #[must_use]
    pub const fn key(&self) -> &BrushKey {
        &self.key
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Shared by every stroke drawn with this brush.
    #[must_use]
    pub fn recipe(&self) -> &Arc<BrushRecipe> {
        &self.recipe
    }

    #[must_use]
    pub const fn size(&self) -> SizeRange {
        self.size
    }

    #[must_use]
    pub const fn opacity(&self) -> f32 {
        self.opacity
    }

    #[must_use]
    pub const fn streamline(&self) -> f32 {
        self.streamline
    }

    /// The brush a copy was made from; `None` for a built-in.
    #[must_use]
    pub const fn source(&self) -> Option<&BrushKey> {
        self.source.as_ref()
    }

    /// True for a brush that comes from a file of the app, false for a copy.
    #[must_use]
    pub const fn is_built_in(&self) -> bool {
        self.source.is_none()
    }
}

/// The brushes on offer, in picker order, and the one a first start draws with, with the
/// built-ins as their files have them (what Reset gives back). Cloning copies the list.
#[derive(Debug, Clone, PartialEq)]
pub struct BrushLibrary {
    pub(crate) brushes: Vec<Brush>,
    pub(crate) default: BrushKey,
    /// The built-ins as their files have them, in file order.
    pub(crate) files: Arc<[Brush]>,
}

impl BrushLibrary {
    /// The built-in brushes.
    ///
    /// # Errors
    /// A `RecipeError` when a built-in file is broken (a bug the tests catch).
    pub fn built_in() -> Result<Self, RecipeError> {
        Self::from_files(&BUILT_IN, DEFAULT_BRUSH)
    }

    /// Brushes from `(key, TOML text)` files, in that order; `default` names one of them.
    ///
    /// # Errors
    /// The first file's `RecipeError`; `Missing` when no file has the key `default`.
    pub fn from_files(files: &[(&str, &str)], default: &str) -> Result<Self, RecipeError> {
        let brushes = files
            .iter()
            .map(|(key, text)| parse(BrushKey::new(key), text))
            .collect::<Result<Vec<_>, _>>()?;
        if !brushes.iter().any(|brush| brush.key.as_str() == default) {
            return Err(RecipeError::Missing {
                field: format!("brush {default}"),
            });
        }
        Ok(Self {
            files: brushes.clone().into(),
            brushes,
            default: BrushKey::new(default),
        })
    }

    /// In picker order; never empty.
    #[must_use]
    pub fn brushes(&self) -> &[Brush] {
        &self.brushes
    }

    /// The brush a first start draws with.
    #[must_use]
    pub const fn default_key(&self) -> &BrushKey {
        &self.default
    }

    /// The position of the brush named `key`.
    #[must_use]
    pub fn find(&self, key: &str) -> Option<usize> {
        self.brushes
            .iter()
            .position(|brush| brush.key.as_str() == key)
    }

    /// The brush named `key`.
    #[must_use]
    pub fn brush(&self, key: &BrushKey) -> Option<&Brush> {
        self.brushes.iter().find(|brush| brush.key == *key)
    }

    /// The built-in named `key` as its file has it.
    #[must_use]
    pub fn file(&self, key: &BrushKey) -> Option<&Brush> {
        self.files.iter().find(|brush| brush.key == *key)
    }

    /// The built-in `brush` descends from, as its file has it (itself for a built-in): what a
    /// curve's Reset gives back. `None` only when the chain of copies is broken (a bug).
    #[must_use]
    pub fn origin(&self, brush: &Brush) -> Option<&Brush> {
        let mut at = brush;
        for _ in 0..=self.brushes.len() {
            match &at.source {
                None => return self.file(&at.key),
                Some(source) => at = self.brush(source)?,
            }
        }
        None
    }
}
