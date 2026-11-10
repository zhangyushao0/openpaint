//! The one-time conversion of the session records (S120c, deleted with it by S121): the sheets of
//! the generated combinations as 1.21.6 stored them (`tests/samples/session-records.bin`, frozen
//! 1.18.1 to 1.21.6) convert to exactly the bytes S120 writes for the same sheets
//! (`areas-records.bin`): each record a sheet of one lesson, the results the numbers settle, its
//! lesson's givens beside it, every project's record untouched; a second run writes nothing, and
//! a record that cannot be read stays as it was and is counted.

#[expect(
    dead_code,
    reason = "the store's helpers taken whole by path; a new title is never made"
)]
#[path = "../../store/tests/common/mod.rs"]
mod common;
#[expect(dead_code, reason = "the samples here are only read")]
#[path = "../../store/tests/sample/dump.rs"]
mod dump;

use std::path::PathBuf;

use common::{TestDir, records, write_records};
use openpaint_practice::{Judge, Outcome, Picked, Purpose};
use openpaint_practice_store::{convert, sheets};
use openpaint_store::Library;

/// The dump's records, as `tests/sample/mod.rs` names them.
type Records = Vec<(String, Vec<u8>, Vec<u8>)>;

fn sample(name: &str) -> Records {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/samples")
        .join(name);
    dump::decode(&std::fs::read(path).expect("a frozen sample"))
}

#[test]
fn the_session_records_convert_to_the_sheets_this_build_writes() {
    let dir = TestDir::new("convert-sample");
    write_records(&dir, &sample("session-records.bin"));
    let mut library = Library::at(&dir);
    let converted = convert(&mut library, &[]).expect("converted");
    assert_eq!(converted.unread, 0);
    let summary = converted.summary();
    assert_eq!(
        summary,
        [
            "1 Day full(2,1) 2/3 5 4 2:Near 3:Clean",
            "2 Day chosen 255/1 2 1 0:Redo",
            "2 records converted, 0 unread: 5 exercises compared, outcomes 1 clean 1 near 1 redo",
        ]
    );
    let (was, sheet) = &converted.sheets[1];
    assert_eq!((was.stage, sheet.purpose), (u8::MAX, Purpose::Day));
    assert_eq!(sheet.lesson().map(|l| l.picked), Some(Picked::Chosen));
    assert_eq!(
        sheet.laid(0).and_then(|laid| laid.result),
        Some((Outcome::Redo, Judge::App))
    );
    assert_eq!(sheets(&mut library).map(|read| read.len()), Ok(2));
    drop(library);
    let once = records(&dir);
    assert!(
        once == sample("areas-records.bin"),
        "the converted library is not the one this build writes"
    );

    let mut library = Library::at(&dir);
    let again = convert(&mut library, &[]).expect("converted again");
    assert!(again.sheets.is_empty() && again.unread == 0);
    drop(library);
    assert!(records(&dir) == once, "a second run writes nothing");
}

#[test]
fn a_record_that_cannot_be_read_is_left_as_it_was_and_counted() {
    let dir = TestDir::new("convert-damaged");
    let mut damaged = sample("session-records.bin");
    let mut key = b"p".to_vec();
    key.extend_from_slice(&3u64.to_be_bytes());
    damaged.push(("library".to_owned(), key, vec![2, 1, 0]));
    write_records(&dir, &damaged);
    let mut library = Library::at(&dir);
    let converted = convert(&mut library, &[]).expect("converted");
    assert_eq!((converted.sheets.len(), converted.unread), (2, 1));
    drop(library);
    let left = records(&dir);
    assert!(
        left.iter()
            .any(|(_, key, value)| key.ends_with(&3u64.to_be_bytes()) && *value == [2, 1, 0])
    );
}
