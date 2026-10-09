//! Brush sets (S47, Procreate's brush library cut to five fixed sets): every brush lies in one
//! of four sets, Sketching, Inking, Painting and Texture, in an order the hand changes; Recent
//! lists the last brushes drawn with, newest first, and is never edited. A built-in starts in the
//! set of its job, a copy right after its source. Stored with the toolbox as lists of keys, empty
//! while every brush lies where a first start puts it, so a later default reaches every
//! toolbox nobody arranged.

use std::sync::Arc;

use crate::brushes::{Brush, BrushKey, BrushLibrary};
use crate::tool_intent::BrushRow;

/// A set of the brush pickers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BrushSet {
    /// The brushes drawn with last, newest first; never edited.
    Recent,
    Sketching,
    Inking,
    Painting,
    Texture,
}

impl BrushSet {
    /// As the pickers show them.
    pub const ALL: [Self; 5] = [
        Self::Recent,
        Self::Sketching,
        Self::Inking,
        Self::Painting,
        Self::Texture,
    ];
    /// The sets a brush lies in, whose order the hand changes.
    pub const EDITABLE: [Self; 4] = [Self::Sketching, Self::Inking, Self::Painting, Self::Texture];

    /// Its word in the pickers and the editor.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Recent => "Recent",
            Self::Sketching => "Sketching",
            Self::Inking => "Inking",
            Self::Painting => "Painting",
            Self::Texture => "Texture",
        }
    }

    /// Its place in `ALL`.
    pub(crate) const fn index(self) -> usize {
        match self {
            Self::Recent => 0,
            Self::Sketching => 1,
            Self::Inking => 2,
            Self::Painting => 3,
            Self::Texture => 4,
        }
    }

    /// Its place in `EDITABLE`; `None` for Recent.
    const fn slot(self) -> Option<usize> {
        match self {
            Self::Recent => None,
            _ => Some(self.index() - 1),
        }
    }
}

/// How many brushes Recent lists.
pub const RECENT_BRUSHES: usize = 6;

/// The set each built-in starts in, by its job; a built-in not listed starts in `ELSEWHERE`.
const HOMES: [(&str, BrushSet); 10] = [
    ("pencil", BrushSet::Sketching),
    ("ink-pen", BrushSet::Inking),
    ("inker", BrushSet::Inking),
    ("monoline", BrushSet::Inking),
    ("hard-round", BrushSet::Painting),
    ("soft-brush", BrushSet::Painting),
    ("flat", BrushSet::Painting),
    ("opaque-paint", BrushSet::Painting),
    ("textured-round", BrushSet::Texture),
    ("speckle", BrushSet::Texture),
];
/// Where a brush goes that has no set of its own to go to: a copy whose source is gone.
const ELSEWHERE: BrushSet = BrushSet::Painting;

/// The four sets' brushes in their order, and Recent.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BrushSets {
    /// Sketching, Inking, Painting, Texture: each holds a brush of the library at most once, and
    /// no brush lies in two.
    sets: [Vec<BrushKey>; 4],
    /// Newest first, at most `RECENT_BRUSHES`, no repeats.
    recent: Vec<BrushKey>,
}

/// The sets as the toolbox record keeps them: four lists of keys (empty while every brush lies
/// where a first start puts it) and Recent's keys.
pub(crate) type StoredSets = ([Vec<String>; 4], Vec<String>);

impl BrushSets {
    /// Every brush of `library` where a first start puts it; Recent empty.
    pub(crate) fn new(library: &BrushLibrary) -> Self {
        let mut sets = Self {
            sets: Default::default(),
            recent: Vec::new(),
        };
        sets.place_missing(library, &[]);
        sets
    }

    /// The keys `set` lists, in order.
    pub(crate) fn keys(&self, set: BrushSet) -> &[BrushKey] {
        match set.slot() {
            Some(slot) => &self.sets[slot],
            None => &self.recent,
        }
    }

    /// The set (one of `EDITABLE`) holding `key`.
    pub(crate) fn set_of(&self, key: &BrushKey) -> Option<BrushSet> {
        BrushSet::EDITABLE
            .into_iter()
            .find(|set| self.keys(*set).contains(key))
    }

    /// A stroke drawn with `key` was committed: it is Recent's newest. Returns whether Recent
    /// changed.
    pub(crate) fn drew(&mut self, key: &BrushKey) -> bool {
        if self.recent.first() == Some(key) {
            return false;
        }
        self.recent.retain(|recent| recent != key);
        self.recent.insert(0, key.clone());
        self.recent.truncate(RECENT_BRUSHES);
        true
    }

    /// `key` to place `to` of its set (0 first; past the end: last), the others keeping their
    /// order. Returns whether it moved.
    pub(crate) fn reorder(&mut self, key: &BrushKey, to: usize) -> bool {
        let Some(list) = self.sets.iter_mut().find(|list| list.contains(key)) else {
            return false;
        };
        let Some(from) = list.iter().position(|listed| listed == key) else {
            return false;
        };
        let to = to.min(list.len() - 1);
        if from == to {
            return false;
        }
        let moved = list.remove(from);
        list.insert(to, moved);
        true
    }

    /// `key` to the end of `set`, out of the set it lay in. Returns whether it moved: not to
    /// Recent, not to the set it lies in, not a key no set holds.
    pub(crate) fn move_to(&mut self, key: &BrushKey, set: BrushSet) -> bool {
        let (Some(slot), Some(from)) = (set.slot(), self.set_of(key).and_then(BrushSet::slot))
        else {
            return false;
        };
        if slot == from {
            return false;
        }
        self.sets[from].retain(|listed| listed != key);
        self.sets[slot].push(key.clone());
        true
    }

    /// The copy `copy` of `source` lies right after it, in its set.
    pub(crate) fn copied(&mut self, source: &BrushKey, copy: &BrushKey) {
        for list in &mut self.sets {
            if let Some(at) = list.iter().position(|listed| listed == source) {
                list.insert(at + 1, copy.clone());
                return;
            }
        }
        if let Some(slot) = ELSEWHERE.slot() {
            self.sets[slot].push(copy.clone());
        }
    }

    /// `key` is gone from the library: out of its set and of Recent.
    pub(crate) fn deleted(&mut self, key: &BrushKey) {
        for list in &mut self.sets {
            list.retain(|listed| listed != key);
        }
        self.recent.retain(|recent| recent != key);
    }

    /// As the toolbox record keeps them: the four lists empty while they are what a first start
    /// with `library` gives.
    pub(crate) fn stored(&self, library: &BrushLibrary) -> StoredSets {
        let keys = |list: &[BrushKey]| list.iter().map(|key| key.as_str().to_owned()).collect();
        let sets = if self.sets == Self::new(library).sets {
            Default::default()
        } else {
            self.sets.each_ref().map(|list| keys(list))
        };
        (sets, keys(&self.recent))
    }

    /// The stored sets and Recent over `library`: a key it does not hold, one listed before, or
    /// one past Recent's length is dropped (named in `dropped`); a brush in no set goes where a
    /// first start puts it, an `orphan` (a copy whose source was gone) to `ELSEWHERE`.
    pub(crate) fn restore(
        library: &BrushLibrary,
        (lists, recent, orphans): (&[Vec<String>; 4], &[String], &[BrushKey]),
        dropped: &mut Vec<String>,
    ) -> Self {
        let mut seen: Vec<BrushKey> = Vec::new();
        let mut take = |text: &String, dropped: &mut Vec<String>| {
            let key = BrushKey::new(text);
            if library.brush(&key).is_none() || seen.contains(&key) {
                dropped.push(text.clone());
                return None;
            }
            seen.push(key.clone());
            Some(key)
        };
        let sets = lists.each_ref().map(|list| {
            list.iter()
                .filter_map(|text| take(text, dropped))
                .collect::<Vec<_>>()
        });
        let mut restored = Self {
            sets,
            recent: Vec::new(),
        };
        for text in recent {
            let key = BrushKey::new(text);
            let known = library.brush(&key).is_some() && !restored.recent.contains(&key);
            if known && restored.recent.len() < RECENT_BRUSHES {
                restored.recent.push(key);
            } else {
                dropped.push(text.clone());
            }
        }
        restored.place_missing(library, orphans);
        restored
    }

    /// Every brush of `library` in no set goes to its home: right after the nearest brush before
    /// it in the library's order that lies there, else first. So with every set empty the sets
    /// follow the library's order, a copy right after its source.
    fn place_missing(&mut self, library: &BrushLibrary, orphans: &[BrushKey]) {
        let brushes = library.brushes();
        for (index, brush) in brushes.iter().enumerate() {
            if self.set_of(brush.key()).is_some() {
                continue;
            }
            let home = if orphans.contains(brush.key()) {
                ELSEWHERE
            } else {
                home(library, brush)
            };
            let Some(slot) = home.slot() else {
                continue;
            };
            let list = &mut self.sets[slot];
            let after = brushes[..index]
                .iter()
                .rev()
                .find_map(|before| list.iter().position(|listed| listed == before.key()));
            list.insert(after.map_or(0, |at| at + 1), brush.key().clone());
        }
    }

    /// Each set's rows as the picker lists them, as `BrushSet::ALL`.
    pub(crate) fn rows(&self, library: &BrushLibrary) -> Arc<[Vec<BrushRow>; 5]> {
        let row = |key: &BrushKey| {
            library.brush(key).map(|brush| BrushRow {
                key: key.clone(),
                name: brush.name().to_owned(),
            })
        };
        Arc::new(BrushSet::ALL.map(|set| self.keys(set).iter().filter_map(row).collect()))
    }
}

/// The set `brush` starts in: its built-in's (a copy's, the built-in it descends from).
fn home(library: &BrushLibrary, brush: &Brush) -> BrushSet {
    let origin = library
        .origin(brush)
        .map_or("", |origin| origin.key().as_str());
    HOMES
        .iter()
        .find(|(key, _)| *key == origin)
        .map_or(ELSEWHERE, |(_, set)| *set)
}
