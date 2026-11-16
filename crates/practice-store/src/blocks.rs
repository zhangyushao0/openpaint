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

use openpaint_app::{PortError, ProjectStore, Record, Space};
use openpaint_practice::{Block, Blocks, Length, TimerWrite, Timing, UnderWay};
use openpaint_store::{Reader, Writer};

/// The Pomodoro's space in the library: the first byte of each of its records' keys.
pub const TIMER: Space = Space::new(b't');

/// The second byte of each record's key: the settings, the block under way, a block completed.
const SETTINGS: u8 = b's';
const CURRENT: u8 = b'u';
const DONE: u8 = b'b';

/// The first byte of each record's bytes.
const TIMING: u8 = 0x54;
const UNDER_WAY: u8 = 0x55;
const BLOCK: u8 = 0x42;

/// What the store keeps of the Pomodoro: the settings (the defaults when none or unreadable), the
/// block under way and the blocks completed, oldest first.
///
/// # Errors
/// Whatever reading the library's records fails with.
pub fn blocks(store: &mut dyn ProjectStore) -> Result<Blocks, PortError> {
    let mut blocks = Blocks::default();
    for (key, value) in store.records(TIMER, &[])? {
        let read = match key.get(1..) {
            Some([SETTINGS]) => decode_timing(&value).map(|timing| blocks.timing = timing),
            Some([CURRENT]) => decode_under_way(&value).map(|under| blocks.under_way = under),
            _ => decode_block(&key, &value).map(|block| blocks.done.push(block)),
        };
        if let Err(error) = read {
            log::warn!("a Pomodoro record is not read: {error}");
        }
    }
    // Keys in order are starts in order (big-endian); kept so whatever the store's order.
    blocks.done.sort_by_key(|block| block.started);
    Ok(blocks)
}

/// `write` stored in one write: the settings and the block under way replaced, a block
/// completed added.
///
/// # Errors
/// Whatever the write fails with; nothing is written then.
pub fn save_timer(store: &mut dyn ProjectStore, write: &TimerWrite) -> Result<(), PortError> {
    let mut records: Vec<(Space, Record)> = vec![
        (TIMER, (key(&[SETTINGS]), encode_timing(write.timing))),
        (TIMER, (key(&[CURRENT]), encode_under_way(write.under_way))),
    ];
    if let Some(block) = write.completed {
        records.push((TIMER, (block_key(block.started), encode_block(&block))));
    }
    store.put_records(&records)?;
    if let Some(block) = write.completed {
        log::info!(
            "a Pomodoro block of {} minutes completed",
            block.length.minutes()
        );
    }
    Ok(())
}

/// For debug builds' test chore only (`practice-test end`): every block begun at or after
/// `since` removed in one write, completed (its record) or under way (none under way then); the
/// settings and every other block stay. Returns how many went.
///
/// # Errors
/// Whatever reading or the write fails with; nothing is removed then.
pub fn forget_blocks(store: &mut dyn ProjectStore, since: u64) -> Result<usize, PortError> {
    let kept = blocks(store)?;
    let gone: Vec<(Space, Vec<u8>)> = (kept.done.iter())
        .filter(|block| block.started >= since)
        .map(|block| (TIMER, block_key(block.started)))
        .collect();
    let under_way = kept.under_way.filter(|block| block.started >= since);
    let put: Vec<(Space, Record)> = (under_way.iter())
        .map(|_| (TIMER, (key(&[CURRENT]), encode_under_way(None))))
        .collect();
    if !gone.is_empty() || !put.is_empty() {
        store.rewrite_records(&put, &gone)?;
    }
    Ok(gone.len() + put.len())
}

/// Whether `key` is the settings' (a preference the practice-only clear keeps, `clear.rs`).
pub(crate) fn is_settings(key: &[u8]) -> bool {
    key == [TIMER.byte(), SETTINGS]
}

/// The key of the record `rest` names in the space.
fn key(rest: &[u8]) -> Vec<u8> {
    [&[TIMER.byte()], rest].concat()
}

/// The key of the block completed that started at `started`.
fn block_key(started: u64) -> Vec<u8> {
    key(&[&[DONE], started.to_be_bytes().as_slice()].concat())
}

fn encode_timing(timing: Timing) -> Vec<u8> {
    let mut writer = Writer::with_capacity(3);
    writer.u8(TIMING);
    writer.u8(timing.length.code());
    writer.bool(timing.chime);
    writer.finish()
}

fn encode_under_way(under_way: Option<UnderWay>) -> Vec<u8> {
    let mut writer = Writer::with_capacity(2 + 8 + 8 + 1 + 4 + 1);
    writer.u8(UNDER_WAY);
    match under_way {
        None => writer.u8(0),
        Some(under) => {
            writer.u8(1);
            writer.u64(under.started);
            writer.u64(under.day.cast_unsigned());
            writer.u8(under.length.code());
            writer.u32(under.left);
            writer.bool(under.paused);
        }
    }
    writer.finish()
}

fn encode_block(block: &Block) -> Vec<u8> {
    let mut writer = Writer::with_capacity(1 + 8 + 1);
    writer.u8(BLOCK);
    writer.u64(block.day.cast_unsigned());
    writer.u8(block.length.code());
    writer.finish()
}

/// A length's code read; another code is refused.
fn read_length(reader: &mut Reader<'_>) -> Result<Length, PortError> {
    Length::of_code(reader.u8()?).ok_or(reader.corrupt())
}

fn decode_timing(bytes: &[u8]) -> Result<Timing, PortError> {
    let mut reader = Reader::new(bytes, "Pomodoro settings");
    if reader.u8()? != TIMING {
        return Err(reader.corrupt());
    }
    let length = read_length(&mut reader)?;
    let chime = reader.bool()?;
    reader.end()?;
    Ok(Timing { length, chime })
}

fn decode_under_way(bytes: &[u8]) -> Result<Option<UnderWay>, PortError> {
    let mut reader = Reader::new(bytes, "Pomodoro block under way");
    if reader.u8()? != UNDER_WAY {
        return Err(reader.corrupt());
    }
    let under = match reader.u8()? {
        0 => None,
        1 => Some(UnderWay {
            started: reader.u64()?,
            day: reader.u64()?.cast_signed(),
            length: read_length(&mut reader)?,
            left: reader.u32()?,
            paused: reader.bool()?,
        }),
        _ => return Err(reader.corrupt()),
    };
    reader.end()?;
    Ok(under)
}

fn decode_block(key: &[u8], bytes: &[u8]) -> Result<Block, PortError> {
    let mut reader = Reader::new(bytes, "Pomodoro block");
    let started = (key.strip_prefix(&[TIMER.byte(), DONE]))
        .and_then(|started| started.try_into().ok())
        .map(u64::from_be_bytes)
        .ok_or(reader.corrupt())?;
    if reader.u8()? != BLOCK {
        return Err(reader.corrupt());
    }
    let day = reader.u64()?.cast_signed();
    let length = read_length(&mut reader)?;
    reader.end()?;
    Ok(Block {
        started,
        day,
        length,
    })
}
