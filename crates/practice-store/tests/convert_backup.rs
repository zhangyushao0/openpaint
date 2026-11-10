//! The conversion proved on a copy of the author's library (S120c, deleted with it by S121):
//! `cargo test -p openpaint-practice-store --test convert_backup -- --ignored --nocapture`.
//!
//! The library converted is the one `OPENPAINT_LIBRARY_COPY` names (a directory holding
//! `library/` and `packs/`, converted in place), else a fresh copy of the backup unpacked at
//! `target/tmp/s116/raw` (never written itself). Every record outside the practice spaces keeps
//! its bytes; each sheet keeps its record's spans; the sessions done by stage, the kinds met, the
//! lines and colours compared, the copies, the time, the book's days and the seals the old records
//! give are what the sheets give; a second run writes nothing. Prints one line a record and the
//! totals.

#[path = "../../store/tests/common/mod.rs"]
#[expect(
    dead_code,
    unused_imports,
    reason = "the store's helpers taken whole by path"
)]
mod common;

use std::path::{Path, PathBuf};

use common::records;
use openpaint_practice::{
    Book, Earned, Family, Kind, STAGES, Sheet, Tally, by_day, earned, met, number, seconds,
};
use openpaint_practice_store::{Packs, Was, convert, sheets};
use openpaint_store::Library;

/// The device's offset from UTC the book is counted on: Beijing's.
const OFFSET: i32 = 8 * 60;

type Records = Vec<(String, Vec<u8>, Vec<u8>)>;

/// The library to convert: the one named, or a fresh copy of the backup.
fn library_copy() -> PathBuf {
    if let Some(named) = std::env::var_os("OPENPAINT_LIBRARY_COPY") {
        return PathBuf::from(named);
    }
    let target = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp");
    let raw = target.join("s116/raw");
    assert!(
        raw.join("library").is_dir(),
        "no backup at {}",
        raw.display()
    );
    let copy = target.join(format!("s120c/convert-{}", std::process::id()));
    copy_dir(&raw, &copy);
    copy
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("a directory");
    for entry in std::fs::read_dir(from).expect("read") {
        let entry = entry.expect("an entry");
        let path = entry.path();
        let into = to.join(entry.file_name());
        if path.is_dir() {
            copy_dir(&path, &into);
        } else {
            std::fs::copy(&path, &into).expect("copied");
        }
    }
}

/// The records outside the practice spaces (`p`, `g` in the library's keyspace).
fn others(all: &Records) -> Records {
    let practice = |(keyspace, key, _): &&(String, Vec<u8>, Vec<u8>)| {
        keyspace == "library" && matches!(key.first(), Some(b'p' | b'g'))
    };
    all.iter().filter(|r| !practice(r)).cloned().collect()
}

/// What the old records add up to, as `Tally::add` added them before S120c.
fn old_tally(old: &[&Was]) -> Tally {
    let mut tally = Tally::default();
    for was in old {
        tally.seconds += seconds(&was.spans);
        if was.stage == 0 {
            tally.copies += u32::from(was.canvas != 0);
            continue;
        }
        if was.finished && was.stage != u8::MAX {
            tally.sessions += 1;
            if let Some(done) = tally.stages.get_mut(usize::from(was.stage).wrapping_sub(1)) {
                *done += 1;
            }
        }
        for (_, kind) in &was.compared {
            match kind.family() {
                Family::Line | Family::Combined => tally.lines += 1,
                Family::Colour => tally.colours += 1,
                _ => {}
            }
        }
    }
    tally
}

/// The seals the old records earned, in order, as `earned` gave them.
fn old_earned(old: &[&Was]) -> Vec<Earned> {
    let mut found: Vec<Earned> = Vec::new();
    for at in 0..old.len() {
        for seal in old_tally(&old[..=at]).seals() {
            if !found.iter().any(|kept| kept.seal == seal) {
                found.push(Earned {
                    seal,
                    by: old[at].id,
                });
            }
        }
    }
    found
}

/// The kinds the old records compared, copies left out, each once by code.
fn old_met(old: &[&Was]) -> Vec<Kind> {
    let mut kinds: Vec<Kind> = (old.iter().filter(|was| was.stage != 0))
        .flat_map(|was| was.compared.iter().map(|(_, kind)| *kind))
        .collect();
    kinds.sort_by_key(|kind| kind.code());
    kinds.dedup();
    kinds
}

/// The local days the old records were practised on, with their seconds.
fn old_days(old: &[&Was]) -> Vec<(i64, u64)> {
    let mut days: Vec<(i64, u64)> = Vec::new();
    for was in old {
        for (day, seconds) in by_day(&was.spans, OFFSET) {
            match days.iter_mut().find(|(kept, _)| *kept == day) {
                Some((_, kept)) => *kept += seconds,
                None => days.push((day, seconds)),
            }
        }
    }
    days.sort_by_key(|(day, _)| *day);
    days
}

/// What the old records counted is what the sheets `read` count: the tally (sessions done by
/// stage, lines, colours, copies, time), the kinds met, the seals in order, the book's days and
/// road.
fn counts_kept(old: &[&Was], read: &[Sheet]) {
    let tally = Tally::of(read);
    assert_eq!(tally, old_tally(old), "the tally");
    assert_eq!(met(read), old_met(old), "the kinds met");
    assert_eq!(earned(read), old_earned(old), "the seals");
    let stage = (old.iter().rev())
        .find(|was| (1..=STAGES).contains(&was.stage))
        .map_or(1, |was| was.stage);
    let book = Book::of(read, stage, OFFSET);
    let days: Vec<(i64, u64)> = book.days.iter().map(|day| (day.day, day.seconds)).collect();
    assert_eq!(days, old_days(old), "the book's days");
    assert_eq!(book.road.done, tally.stages, "the road");
    assert_eq!(book.seconds, tally.seconds);
    println!(
        "tally: {} sessions {:?} by stage, {} lines, {} colours, {} copies, {} s; {} kinds met; \
         {} seals; {} days",
        tally.sessions,
        tally.stages,
        tally.lines,
        tally.colours,
        tally.copies,
        tally.seconds,
        met(read).len(),
        book.seals.len(),
        days.len()
    );
}

#[test]
#[ignore = "on a copy of the author's library: run by hand"]
fn the_author_s_library_converts_with_every_count_kept() {
    let dir = library_copy();
    let library_dir = dir.join("library");
    println!("converting {}", library_dir.display());
    let before = records(&library_dir);
    let packs = Packs::at(dir.join("packs")).list().expect("the packs");
    let mut library = Library::at(&library_dir);
    let converted = convert(&mut library, &packs).expect("converted");
    let read = sheets(&mut library).expect("the sheets");
    drop(library);
    let after = records(&library_dir);
    assert!(
        others(&after) == others(&before),
        "a record outside practice changed"
    );
    println!(
        "{} records outside practice byte for byte",
        others(&before).len()
    );

    let mut old: Vec<&Was> = converted.sheets.iter().map(|(was, _)| was).collect();
    old.sort_by_key(|was| was.id);
    assert_eq!(converted.unread, 0, "an old record unread");
    assert_eq!(read.len(), old.len(), "a sheet unread");
    for (was, sheet) in &converted.sheets {
        let stored = read.iter().find(|s| s.id == was.id).expect("its sheet");
        assert_eq!(stored, sheet, "sheet {} as stored", was.id);
        assert_eq!(sheet.spans, was.spans, "sheet {}'s spans", was.id);
    }
    for (line, (was, sheet)) in converted.summary().iter().zip(&converted.sheets) {
        let derived = number(&read, sheet);
        let same = if derived == was.number {
            ""
        } else {
            " NUMBER DIFFERS"
        };
        println!("{line} (number now {derived}{same})");
    }
    println!("{}", converted.summary().last().expect("the totals"));

    counts_kept(&old, &read);

    let mut library = Library::at(&library_dir);
    let again = convert(&mut library, &packs).expect("converted again");
    drop(library);
    assert!(again.sheets.is_empty() && again.unread == 0);
    assert!(records(&library_dir) == after, "a second run wrote");
    println!("a second run writes nothing");
}
