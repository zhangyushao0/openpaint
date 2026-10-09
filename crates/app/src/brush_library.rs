//! Changing the brush library (S46): a brush edited in place, copied right after itself, reset
//! to its file, renamed or (a copy) deleted; and the library as the toolbox record stores it
//! (`StoredBrush`), which a stored brush the engine refuses never blocks: it falls back to the
//! built-in it comes from.

use std::sync::Arc;

use openpaint_core::BrushRecipe;

use crate::brushes::{Brush, BrushKey, BrushLibrary, SizeRange};
use crate::editor_state::{BrushParts, StoredBrush};
use crate::recipe::{MAX_BRUSH_NAME, brush_name};

/// What a copy's name ends with.
const COPY: &str = " copy";
/// What a copy's key starts with, before its number.
const COPY_KEY: &str = "copy-";

impl BrushLibrary {
    /// Puts `brush` in place of the brush with its key.
    pub(crate) fn replace(&mut self, brush: Brush) {
        if let Some(at) = self.brushes.iter().position(|b| b.key == brush.key) {
            self.brushes[at] = brush;
        }
    }

    /// A copy of `key` named "<name> copy" with a key of its own, right after it. Returns the
    /// copy's key.
    pub(crate) fn duplicate(&mut self, key: &BrushKey) -> Option<BrushKey> {
        let at = self.brushes.iter().position(|b| b.key == *key)?;
        let source = &self.brushes[at];
        let copy = Brush {
            key: self.copy_key(),
            name: copy_name(&source.name),
            source: Some(source.key.clone()),
            ..source.clone()
        };
        let copied = copy.key.clone();
        self.brushes.insert(at + 1, copy);
        Some(copied)
    }

    /// `copy-<n>`, n one more than any copy's.
    fn copy_key(&self) -> BrushKey {
        let last = self
            .brushes
            .iter()
            .filter_map(|b| b.key.as_str().strip_prefix(COPY_KEY)?.parse::<u32>().ok())
            .max()
            .unwrap_or(0);
        BrushKey::new(&format!("{COPY_KEY}{}", last + 1))
    }

    /// Takes the copy `key` out; copies made from it now come from its source. Returns it, or
    /// `None` for a built-in or a key the library does not hold.
    pub(crate) fn delete(&mut self, key: &BrushKey) -> Option<Brush> {
        let at = self.brushes.iter().position(|b| b.key == *key)?;
        self.brushes[at].source.as_ref()?;
        let gone = self.brushes.remove(at);
        for brush in &mut self.brushes {
            if brush.source.as_ref() == Some(key) {
                brush.source.clone_from(&gone.source);
            }
        }
        Some(gone)
    }

    /// The built-in `key` as its file has it again. Returns whether it changed.
    pub(crate) fn reset(&mut self, key: &BrushKey) -> bool {
        let Some(file) = self.file(key).cloned() else {
            return false;
        };
        let changed = self.brush(key) != Some(&file);
        self.replace(file);
        changed
    }

    /// Names the brush `key` `text` (trimmed, 1 to `MAX_BRUSH_NAME` characters). Returns
    /// whether it changed.
    pub(crate) fn rename(&mut self, key: &BrushKey, text: &str) -> bool {
        let Some(name) = brush_name(text) else {
            return false;
        };
        match self.brushes.iter_mut().find(|b| b.key == *key) {
            Some(brush) if brush.name != name => {
                brush.name = name;
                true
            }
            _ => false,
        }
    }

    /// The library as the toolbox record keeps it, in picker order: a built-in as its file has
    /// it by its key alone, any other brush whole. Empty while it is the built-ins as their
    /// files have them, in their order.
    pub(crate) fn stored(&self) -> Vec<StoredBrush> {
        if self.brushes.as_slice() == &*self.files {
            return Vec::new();
        }
        self.brushes
            .iter()
            .map(|brush| StoredBrush {
                key: brush.key.as_str().to_owned(),
                brush: (self.file(&brush.key) != Some(brush)).then(|| parts_of(brush)),
            })
            .collect()
    }

    /// Takes over a stored library (empty: the built-ins). A built-in missing from it comes
    /// back where its file's order puts it; a stored brush the engine refuses falls back to the
    /// built-in it comes from (its name goes to `fell_back`), an unknown or repeated key is
    /// skipped. Returns whether everything was taken over.
    pub(crate) fn restore(&mut self, stored: &[StoredBrush], fell_back: &mut Vec<String>) -> bool {
        self.brushes = self.files.to_vec();
        if stored.is_empty() {
            return true;
        }
        let mut whole = true;
        let mut brushes: Vec<Brush> = Vec::with_capacity(stored.len());
        for entry in stored {
            let key = BrushKey::new(&entry.key);
            let brush = match &entry.brush {
                _ if brushes.iter().any(|b| b.key == key) => None,
                None => self.file(&key).cloned(),
                Some(parts) => brush_of(&key, parts, self.file(&key).is_some()).or_else(|| {
                    fell_back.push(parts.name.clone());
                    self.fallback(&key, parts)
                }),
            };
            match brush {
                Some(brush) if brush_fits(&brush, entry) => brushes.push(brush),
                Some(brush) => {
                    whole = false;
                    brushes.push(brush);
                }
                None => whole = false,
            }
        }
        self.brushes = self.with_files(brushes);
        self.mend_sources();
        whole
    }

    /// For a stored brush the engine refuses: the built-in `key`, or for a copy the built-in it
    /// was copied from under its own key and name.
    fn fallback(&self, key: &BrushKey, parts: &BrushParts) -> Option<Brush> {
        if let Some(file) = self.file(key) {
            return Some(file.clone());
        }
        let source = BrushKey::new(&parts.source);
        let file = self.file(&source)?;
        Some(Brush {
            key: key.clone(),
            name: brush_name(&parts.name).unwrap_or_else(|| copy_name(&file.name)),
            source: Some(source),
            ..file.clone()
        })
    }

    /// `brushes` with every built-in it lacks inserted after the built-in its file follows.
    fn with_files(&self, mut brushes: Vec<Brush>) -> Vec<Brush> {
        for (index, file) in self.files.iter().enumerate() {
            if brushes.iter().any(|b| b.key == file.key) {
                continue;
            }
            let after = self.files[..index]
                .iter()
                .rev()
                .find_map(|before| brushes.iter().position(|b| b.key == before.key));
            brushes.insert(after.map_or(0, |at| at + 1), file.clone());
        }
        brushes
    }

    /// A copy whose source is not in the library comes from the default brush.
    fn mend_sources(&mut self) {
        let keys: Vec<BrushKey> = self.brushes.iter().map(|b| b.key.clone()).collect();
        let default = self.default.clone();
        for brush in &mut self.brushes {
            if brush
                .source
                .as_ref()
                .is_some_and(|source| !keys.contains(source))
            {
                brush.source = Some(default.clone());
            }
        }
    }
}

/// True unless the brush taken over differs from what was stored (it fell back).
fn brush_fits(brush: &Brush, entry: &StoredBrush) -> bool {
    entry
        .brush
        .as_ref()
        .is_none_or(|parts| parts.name == brush.name && parts.recipe == brush.recipe.to_parts())
}

/// "<name> copy", the name cut so the whole fits `MAX_BRUSH_NAME`.
fn copy_name(name: &str) -> String {
    let room = MAX_BRUSH_NAME - COPY.chars().count();
    let base: String = name.chars().take(room).collect();
    format!("{}{COPY}", base.trim_end())
}

/// A brush as plain data.
pub(crate) fn parts_of(brush: &Brush) -> BrushParts {
    BrushParts {
        name: brush.name.clone(),
        source: brush
            .source
            .as_ref()
            .map_or_else(String::new, |source| source.as_str().to_owned()),
        size: [brush.size.min, brush.size.default, brush.size.max],
        opacity: brush.opacity,
        streamline: brush.streamline,
        recipe: brush.recipe.to_parts(),
    }
}

/// The brush `key` from plain data, checked whole; `None` when anything is out of range, a
/// built-in names a source or a copy none.
fn brush_of(key: &BrushKey, parts: &BrushParts, built_in: bool) -> Option<Brush> {
    let [min, default, max] = parts.size;
    let opacity = parts.opacity > 0.0 && parts.opacity <= 1.0;
    let streamline = (0.0..=1.0).contains(&parts.streamline);
    if !opacity || !streamline || built_in != parts.source.is_empty() {
        return None;
    }
    Some(Brush {
        key: key.clone(),
        name: brush_name(&parts.name)?,
        recipe: Arc::new(BrushRecipe::from_parts(&parts.recipe).ok()?),
        size: SizeRange::new(min, default, max)?,
        opacity: parts.opacity,
        streamline: parts.streamline,
        source: (!built_in).then(|| BrushKey::new(&parts.source)),
    })
}
