//! The practice-only clear (S152, D112): every practice record and every practice sheet removed
//! from the library in one write, the studio's artworks, the packs beside the library and the
//! user's own pictures left as they are. It removes by space and key, never by decoding, so it
//! clears records this version cannot read: every record of the sheets' (`p`), the givens' (`g`)
//! and the blind pairs' (`b`) spaces, the Pomodoro's blocks (`t`, its settings kept: a
//! preference, not a record), and every project a sheet's record names in its head
//! (`named_project`) or a lesson's givens name in their key, with everything stored for it.

use std::collections::BTreeSet;

use openpaint_app::{PortError, ProjectId, ProjectStore, Space};

use crate::blocks::{TIMER, is_settings};
use crate::pairs::PAIRS;
use crate::records::{GIVENS, PRACTICE, named_project};

/// What the clear removed, by count, and the projects it removed (their display caches go too).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Cleared {
    /// The sheets' records, read or not.
    pub sheets: usize,
    /// The lessons' givens.
    pub givens: usize,
    /// The Pomodoro's records but its settings: the block under way and the blocks completed.
    pub blocks: usize,
    /// The blind pairs answered.
    pub pairs: usize,
    /// The practice sheets' projects, oldest first.
    pub projects: Vec<ProjectId>,
}

impl Cleared {
    /// Every record and project it removed.
    #[must_use]
    pub fn removed(&self) -> usize {
        self.sheets + self.givens + self.blocks + self.pairs + self.projects.len()
    }
}

/// Every practice record and sheet removed in one write (see the module).
///
/// # Errors
/// Whatever reading the records or the write fails with; nothing is removed then.
pub fn clear(store: &mut dyn ProjectStore) -> Result<Cleared, PortError> {
    let sheets = store.records(PRACTICE, &[])?;
    let givens = store.records(GIVENS, &[])?;
    let blocks: Vec<_> = (store.records(TIMER, &[])?.into_iter())
        .filter(|(key, _)| !is_settings(key))
        .collect();
    let pairs = store.records(PAIRS, &[])?;
    let mut projects: BTreeSet<u64> = (sheets.iter())
        .filter_map(|(_, bytes)| named_project(bytes))
        .collect();
    projects.extend(givens.iter().filter_map(|(key, _)| givens_project(key)));
    let spaced = |space: Space, records: &[(Vec<u8>, Vec<u8>)]| {
        records
            .iter()
            .map(move |(key, _)| (space, key.clone()))
            .collect::<Vec<_>>()
    };
    let mut keys = spaced(PRACTICE, &sheets);
    keys.extend(spaced(GIVENS, &givens));
    keys.extend(spaced(TIMER, &blocks));
    keys.extend(spaced(PAIRS, &pairs));
    let projects: Vec<ProjectId> = projects.into_iter().map(ProjectId::new).collect();
    let cleared = Cleared {
        sheets: sheets.len(),
        givens: givens.len(),
        blocks: blocks.len(),
        pairs: pairs.len(),
        projects,
    };
    if cleared.removed() > 0 {
        store.remove_owned(&keys, &cleared.projects)?;
    }
    log::info!(
        "practice cleared: {} sheets' records, {} givens, {} Pomodoro records, {} pairs, {} \
         projects",
        cleared.sheets,
        cleared.givens,
        cleared.blocks,
        cleared.pairs,
        cleared.projects.len()
    );
    Ok(cleared)
}

/// The project a givens' key names (`['g'][canvas u64][lesson u16]`, big-endian); none for 0 or
/// a key of another length.
fn givens_project(key: &[u8]) -> Option<u64> {
    let canvas: [u8; 8] = key.get(1..9)?.try_into().ok()?;
    let canvas = u64::from_be_bytes(canvas);
    (key.len() == 1 + 8 + 2 && canvas != 0).then_some(canvas)
}
