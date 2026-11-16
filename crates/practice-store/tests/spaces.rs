//! Practice's record spaces beside the sheets. The Pomodoro's records (S124) in the space `t`:
//! the settings, the block under way and each block completed round-trip in their bytes; a block
//! completed and the one under way gone land in one write, and a write refused leaves the block
//! under way as it was; a record of another shape, a code unknown or a key not the space's own
//! is refused, never guessed. The blind pairs (S147) in the space `b`: `pairs/`.

mod pairs;
mod support;
#[path = "../../store/tests/common/test_dir.rs"]
mod test_dir;

use openpaint_app::ProjectStore;
use openpaint_practice::{Block, Blocks, Length, TimerWrite, Timing, UnderWay};
use openpaint_practice_store::{TIMER, blocks, forget_blocks, save_timer};
use openpaint_store::Library;
use support::Batches;
use test_dir::TestDir;

/// 2026-10-05 12:00 UTC, and its local day in Beijing.
const NOON: u64 = 1_791_201_600;
const DAY: i64 = 20_731;

/// A block of `length` under way, started at `started`.
fn under_way(started: u64, length: Length, left: u32, paused: bool) -> UnderWay {
    UnderWay {
        started,
        day: DAY,
        length,
        left,
        paused,
    }
}

/// The key of `rest` in the space.
fn key(rest: &[u8]) -> Vec<u8> {
    [b"t".as_slice(), rest].concat()
}

#[test]
fn the_settings_the_block_under_way_and_the_blocks_completed_round_trip() {
    let dir = TestDir::new("practice-timer");
    let mut store = Batches::of(Library::at(&dir));
    assert_eq!(blocks(&mut store), Ok(Blocks::default()), "none stored yet");
    let timing = Timing {
        length: Length::Forty,
        chime: false,
    };
    let running = under_way(NOON, Length::Forty, 1234, true);
    let write = TimerWrite {
        timing,
        under_way: Some(running),
        completed: None,
    };
    save_timer(&mut store, &write).expect("saved");
    let stored = store.records(TIMER, &[]).expect("timer");
    let mut bytes = vec![0x55, 1];
    bytes.extend_from_slice(&NOON.to_le_bytes());
    bytes.extend_from_slice(&DAY.to_le_bytes());
    bytes.push(40);
    bytes.extend_from_slice(&1234u32.to_le_bytes());
    bytes.push(1);
    let expected = vec![(key(b"s"), vec![0x54, 40, 0]), (key(b"u"), bytes)];
    assert_eq!(
        stored, expected,
        "the settings and the block under way, in their bytes"
    );
    drop(store);
    let read = blocks(&mut Library::at(&dir)).expect("read");
    assert_eq!((read.timing, read.under_way), (timing, Some(running)));
}

#[test]
fn blocks_completed_land_each_in_one_write_and_read_oldest_first() {
    let dir = TestDir::new("practice-timer-done");
    let mut store = Batches::of(Library::at(&dir));
    let timing = Timing {
        length: Length::Forty,
        chime: false,
    };
    // The time up and End: the block completed and none under way, in one write.
    let done = Block {
        started: NOON,
        day: DAY,
        length: Length::Forty,
    };
    let earlier = Block {
        started: NOON - 7200,
        day: -3,
        length: Length::Fifteen,
    };
    for completed in [done, earlier] {
        let write = TimerWrite {
            timing,
            under_way: None,
            completed: Some(completed),
        };
        save_timer(&mut store, &write).expect("saved");
    }
    assert_eq!(store.writes, [3, 3], "a block completed lands in one write");
    drop(store);
    let mut library = Library::at(&dir);
    let read = blocks(&mut library).expect("read");
    let expected = Blocks {
        timing,
        under_way: None,
        done: vec![earlier, done],
    };
    assert_eq!(read, expected, "the blocks oldest first");
    assert_eq!(read.on(DAY), 1);
    let mut block = vec![0x42];
    block.extend_from_slice(&DAY.to_le_bytes());
    block.push(40);
    let kept = library.records(TIMER, b"b").expect("blocks");
    assert_eq!(
        kept[1],
        (key(&[b"b".as_slice(), &NOON.to_be_bytes()].concat()), block)
    );
}

#[test]
fn the_test_chores_end_forgets_the_blocks_begun_since_in_one_write() {
    let dir = TestDir::new("practice-timer-forget");
    let mut store = Batches::of(Library::at(&dir));
    let timing = Timing::default();
    let block = |started| Block {
        started,
        day: DAY,
        length: Length::TwentyFive,
    };
    for started in [NOON - 3600, NOON, NOON + 60] {
        let write = TimerWrite {
            timing,
            under_way: Some(under_way(NOON + 600, Length::TwentyFive, 30, false)),
            completed: Some(block(started)),
        };
        save_timer(&mut store, &write).expect("saved");
    }
    store.writes.clear();
    assert_eq!(forget_blocks(&mut store, NOON), Ok(3));
    assert_eq!(
        store.writes,
        [3],
        "two blocks and the one under way in one write"
    );
    let expected = Blocks {
        timing,
        under_way: None,
        done: vec![block(NOON - 3600)],
    };
    assert_eq!(blocks(&mut store), Ok(expected), "the author's block stays");
    assert_eq!(forget_blocks(&mut store, NOON), Ok(0), "nothing left to go");
}

#[test]
fn a_completing_write_refused_leaves_the_block_under_way_and_no_block() {
    let dir = TestDir::new("practice-timer-refused");
    let mut store = Batches::of(Library::at(&dir));
    let running = under_way(NOON, Length::TwentyFive, 0, false);
    let begun = TimerWrite {
        timing: Timing::default(),
        under_way: Some(running),
        completed: None,
    };
    save_timer(&mut store, &begun).expect("begun");
    store.refused = |(_, (key, _))| key.get(1) == Some(&b'b');
    let completed = TimerWrite {
        under_way: None,
        completed: Some(Block {
            started: NOON,
            day: DAY,
            length: Length::TwentyFive,
        }),
        ..begun
    };
    assert!(save_timer(&mut store, &completed).is_err());
    let read = blocks(&mut store).expect("read");
    assert_eq!(
        read.under_way,
        Some(running),
        "still under way, its time up"
    );
    assert!(read.done.is_empty());
}

#[test]
fn a_timer_record_of_another_shape_is_refused_not_guessed() {
    let dir = TestDir::new("practice-timer-damaged");
    let mut store = Batches::of(Library::at(&dir));
    let good = Block {
        started: NOON,
        day: DAY,
        length: Length::Fifteen,
    };
    let write = TimerWrite {
        timing: Timing {
            length: Length::Fifteen,
            chime: true,
        },
        under_way: Some(under_way(NOON, Length::Fifteen, 60, false)),
        completed: Some(good),
    };
    save_timer(&mut store, &write).expect("saved");
    let block_key = |started: u64| key(&[b"b".as_slice(), &started.to_be_bytes()].concat());
    let day = DAY.to_le_bytes().to_vec();
    let damaged = [
        // The settings with a length of 30 minutes: the defaults.
        (key(b"s"), vec![0x54, 30, 1]),
        // The block under way with a byte after it: none.
        (key(b"u"), [vec![0x55, 0], vec![0]].concat()),
        // A block of another tag, of a length unknown, one cut short, one keyed short.
        (block_key(1), [vec![0x41], day.clone(), vec![25]].concat()),
        (block_key(2), [vec![0x42], day.clone(), vec![20]].concat()),
        (block_key(3), [vec![0x42], day[..4].to_vec()].concat()),
        (key(b"b\x01"), [vec![0x42], day, vec![25]].concat()),
        // A record of a kind this version does not know.
        (key(b"x"), vec![0x58]),
    ];
    let records: Vec<_> = damaged.into_iter().map(|record| (TIMER, record)).collect();
    store.put_records(&records).expect("written raw");
    let read = blocks(&mut store).expect("read");
    let expected = Blocks {
        timing: Timing::default(),
        under_way: None,
        done: vec![good],
    };
    assert_eq!(read, expected, "only the block whole is read");
}
