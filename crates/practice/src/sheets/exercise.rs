//! An exercise on a sheet: its kind, its seed and (for a kind copying one) its picture, and the
//! original they make; a kind and its original are `kinds/`.

use std::sync::Arc;

use crate::kinds::{Kind, Original};
use crate::packs::pack::ItemRef;

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
    pub fn original(&self) -> Arc<dyn Original> {
        self.kind.described().generate(self)
    }

    /// The picture it copies, an empty reference where none was kept.
    pub(crate) fn item(&self) -> ItemRef {
        self.picture.clone().unwrap_or_default()
    }
}
