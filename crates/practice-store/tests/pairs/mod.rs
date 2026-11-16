//! The blind pairs (S147) in the record space `b`: a pair round-trips in its 23 bytes under its key,
//! a library holding none reads as before (its sheets as they were, no pair), a record of another
//! shape or a code unknown is refused and left out, and the test chore's end removes the pairs
//! asked at its sheets and nothing else.

use super::support::Batches;
use super::test_dir::TestDir;
use openpaint_app::{ProjectStore, Space};
use openpaint_practice::{Answer, BlindPair, CopyRef};
use openpaint_practice_store::{PAIRS, forget_pairs, pairs, put_pair, sheets};
use openpaint_store::Library;

/// 2026-10-05 12:00 UTC.
const NOON: u64 = 1_791_201_600;

fn pair(asked: u64, lesson: u64, newer: u16, answer: Answer) -> BlindPair {
    BlindPair {
        asked,
        lesson: (lesson, 2),
        older: CopyRef {
            sheet: 3,
            exercise: 4,
        },
        newer: CopyRef {
            sheet: lesson,
            exercise: newer,
        },
        older_left: newer.is_multiple_of(2),
        answer,
    }
}

#[test]
fn a_pair_round_trips_in_its_bytes_and_a_library_without_one_reads_as_before() {
    let dir = TestDir::new("practice-pairs");
    let mut store = Batches::of(Library::at(&dir));
    assert_eq!(pairs(&mut store), Ok(Vec::new()), "none stored yet");
    assert_eq!(sheets(&mut store), Ok(Vec::new()));
    let answered = pair(NOON, 9, 1, Answer::Right);
    put_pair(&mut store, &answered).expect("stored");
    assert_eq!(store.writes, [1], "one write");
    let stored = store.records(PAIRS, &[]).expect("pairs");
    let mut key = vec![b'b'];
    key.extend_from_slice(&NOON.to_be_bytes());
    key.extend_from_slice(&9u64.to_be_bytes());
    key.extend_from_slice(&1u16.to_be_bytes());
    let mut bytes = vec![0x50];
    bytes.extend_from_slice(&9u64.to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&3u64.to_le_bytes());
    bytes.extend_from_slice(&4u16.to_le_bytes());
    bytes.extend_from_slice(&[2, 3]);
    assert_eq!(bytes.len(), 23);
    assert_eq!(stored, [(key, bytes)]);
    assert_eq!(pairs(&mut store), Ok(vec![answered]));
    assert_eq!(sheets(&mut store), Ok(Vec::new()), "no sheet came of it");
}

#[test]
fn a_pair_of_another_shape_is_left_out() {
    let dir = TestDir::new("practice-pairs-refused");
    let mut store = Batches::of(Library::at(&dir));
    let kept = pair(NOON, 5, 0, Answer::Alike);
    put_pair(&mut store, &kept).expect("stored");
    let (key, bytes) = store.records(PAIRS, &[]).expect("pairs").remove(0);
    let mut damaged = Vec::new();
    for (at, byte) in [(0, 0x51), (21, 3), (22, 4)] {
        let mut wrong = bytes.clone();
        wrong[at] = byte;
        let mut other = key.clone();
        other[1] = 0x7F - u8::try_from(at).expect("small");
        damaged.push((other, wrong));
    }
    damaged.push(([key.as_slice(), &[0]].concat(), bytes.clone()));
    let mut longer = key.clone();
    longer[1] = 0x40;
    damaged.push((longer, [bytes.as_slice(), &[1]].concat()));
    let space: Space = PAIRS;
    let put: Vec<_> = damaged.into_iter().map(|record| (space, record)).collect();
    store.put_records(&put).expect("written");
    assert_eq!(
        pairs(&mut store),
        Ok(vec![kept]),
        "only the pair of its shape"
    );
}

#[test]
fn the_test_chore_removes_the_pairs_asked_at_its_sheets_and_nothing_else() {
    let dir = TestDir::new("practice-pairs-forgotten");
    let mut store = Batches::of(Library::at(&dir));
    let (mine, test, other) = (
        pair(NOON, 4, 0, Answer::Left),
        pair(NOON + 60, 7, 1, Answer::Right),
        pair(NOON + 120, 7, 2, Answer::Alike),
    );
    for answered in [mine, test, other] {
        put_pair(&mut store, &answered).expect("stored");
    }
    assert_eq!(forget_pairs(&mut store, &[7, 11]), Ok(2));
    assert_eq!(pairs(&mut store), Ok(vec![mine]));
    assert_eq!(
        forget_pairs(&mut store, &[11]),
        Ok(0),
        "nothing asked there"
    );
}
