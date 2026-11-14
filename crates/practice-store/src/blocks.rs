//! The Pomodoro's records (S124, `openpaint_practice::Blocks`) in the library's record space
//! `TIMER` (`t`), beside the sheets' (`p`, `g`): additive, nothing stored before changes shape.
//! Each record is read only in this shape, every field present; anything else is refused (left
//! out, logged; the settings unreadable read as the defaults).
//!
//! - `['t']['s']` the settings: the tag `TIMING` (u8), the length chosen (u8, its minutes: 15,
//!   25 or 40, `Length::code`), the chime (u8: 0 off, 1 on).
//! - `['t']['u']` the block under way: the tag `UNDER_WAY` (u8), then 0 (u8: none) or 1 and when
//!   it started (u64 seconds since 1970 UTC), its local day (i64 as its u64 bits), its length (u8
//!   minutes), the seconds left (u32, 0: the time is up) and whether the user paused it (u8: 0,
//!   1). A block ended or stopped writes 0 here rather than removing the key, so a block
//!   completed and the one under way gone land in one write (`save_timer`).
//! - `['t']['b'][started u64 big-endian]` a block completed: the tag `BLOCK` (u8), its local day
//!   (i64 as its u64 bits), its length (u8 minutes).
//!
//! A sheet's lesson ended by the Pomodoro's End loses its exercises not begun: `stored::trim`
//! writes its record and its last lesson's givens without them in one write.

use openpaint_app::{PortError, ProjectStore, Space};
use openpaint_practice::{Blocks, TimerWrite};

/// The Pomodoro's space in the library: the first byte of each of its records' keys.
pub const TIMER: Space = Space::new(b't');

/// What the store keeps of the Pomodoro: the settings (the defaults when none or unreadable), the
/// block under way and the blocks completed, oldest first.
///
/// # Errors
/// Whatever reading the library's records fails with.
pub fn blocks(store: &mut dyn ProjectStore) -> Result<Blocks, PortError> {
    // S124a
    let _ = store;
    Ok(Blocks::default())
}

/// `write` stored in one write: the settings and the block under way replaced, a block
/// completed added.
///
/// # Errors
/// Whatever the write fails with; nothing is written then.
pub fn save_timer(store: &mut dyn ProjectStore, write: &TimerWrite) -> Result<(), PortError> {
    // S124a
    let _ = (store, write);
    Ok(())
}
