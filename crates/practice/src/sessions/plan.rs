//! What a session holds: the stages and its exercises, each a kind and seed (and a single shape's
//! or a colour copy's picture) and its original; a kind and its original are `kinds/`, a family
//! `family.rs`, the stages' sessions `session.rs`, how they lie on the sheet `layout.rs`.

pub use crate::kinds::{Kind, Original};
use crate::packs::pack::ItemRef;
pub use crate::sessions::family::Family;

/// The course's stages, after V大's eight weeks.
pub const STAGES: u8 = 8;
/// The stages whose exercises exist so far: a session can be started in these.
pub const STAGES_READY: u8 = 5;
/// About how many sessions a stage holds; only a guide, never a quota.
pub const STAGE_SESSIONS: u32 = 15;

/// One exercise: its kind, the seed it is generated from and, for a single shape, a colour copy,
/// a division or a whole copy, the picture it copies.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Exercise {
    pub kind: Kind,
    pub seed: u64,
    pub picture: Option<ItemRef>,
}

impl Exercise {
    /// A generated exercise of `kind` seeded `seed`, no picture.
    #[must_use]
    pub const fn generated(kind: Kind, seed: u64) -> Self {
        Self {
            kind,
            seed,
            picture: None,
        }
    }

    /// Its original, the same for the same kind and seed (its kind's description makes it).
    #[must_use]
    pub fn original(&self) -> Original {
        self.kind.described().original(self)
    }

    /// The picture it copies, an empty reference where none was kept.
    pub(crate) fn picture(&self) -> Original {
        Original::Picture(self.picture.clone().unwrap_or_default())
    }
}
