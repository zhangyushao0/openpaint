//! The practice-only clear (S152) on a library of the author's shape: the frozen format-1 studio
//! (every record kind, `store/tests/format.rs`) with practice beside it: a day's sheet of two
//! lessons, a test's, a colour copy, a drawing from life, the Pomodoro's settings, a block under
//! way and one completed, a blind pair, and a sheet whose record this version cannot read (a
//! lesson picked 3 holding metric 47, as records of before S152 did). The clear removes every
//! practice record and every sheet's project, the unreadable one's too, in one write; every other
//! record keeps its bytes, the Pomodoro's settings with them; the practice log made of what is
//! left lays the first lesson. The same clear on a copy of a real library: `copy.rs`.

#[path = "../../../store/tests/common/mod.rs"]
mod common;
mod copy;
#[path = "../../../store/tests/sample/dump.rs"]
mod dump;

use std::path::{Path, PathBuf};

use common::{TestDir, records, untitled, write_records};
use dump::{decode, encode};
use openpaint_app::{ProjectId, ProjectStore, Title};
use openpaint_core::{Artwork, CanvasSize};
use openpaint_practice::{
    Answer, BlindPair, Block, Blocks, CopyRef, First, Givens, ItemRef, Kind, Length, LineTier,
    Metric, Picked, Planned, Purpose, Sheet, SkillLevel, TimerWrite, Timing, UnderWay,
};
use openpaint_practice_app::PracticeLog;
use openpaint_practice_store::{
    PRACTICE, blocks, clear, pairs, put_pair, save, save_timer, sheets, start,
};
use openpaint_store::Library;

/// 2026-10-05 12:00 UTC.
const NOON: u64 = 1_791_201_600;

/// The dump's records, as `store/tests/sample/mod.rs` names them.
type Records = Vec<(String, Vec<u8>, Vec<u8>)>;

fn sample_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../store/tests/samples/format-1.bin")
}

fn named(title: &str) -> Title {
    Title::new(title).expect("title")
}

fn canvas() -> Artwork {
    Artwork::new(CanvasSize::new(64, 48).expect("size"))
}

/// A lesson of four arcs, sheet `id` for `purpose`, begun at `started`, the second compared
/// with a point in space's number (code 46).
fn lesson(id: u64, purpose: Purpose, started: u64) -> Sheet {
    let exercises = (0..4)
        .map(|seed| {
            let kind = Kind::Line(LineTier::Arc);
            (
                openpaint_practice::Exercise::generated(kind, seed),
                SkillLevel::First,
            )
        })
        .collect();
    let planned = Planned {
        picked: Picked::Chosen,
        exercises,
    };
    let mut sheet = Sheet::begun(id, (purpose, started), planned);
    let laid = sheet.laid_mut(1).expect("an arc");
    laid.compared(First::of(started + 60, 3, vec![(Metric::SpaceOff, 0.25)]));
    sheet
}

/// `sheet` started on a project of its own titled `title`, its record as stored.
fn started(library: &mut Library, sheet: &Sheet, title: &Title) -> Sheet {
    let givens = Givens::of_lesson(sheet.lesson().expect("a lesson"), &[]);
    let id = start(library, (&canvas(), title), sheet, &givens).expect("started");
    library.save(id, &canvas()).expect("its sheet saved");
    Sheet {
        canvas: id.get(),
        ..sheet.clone()
    }
}

/// `bytes` of a sheet as records before S152 held it: its first lesson picked 3 (a unit's full
/// lesson, unit 1, a warm-up of 4) and its number 46 stored as 47.
fn of_old_shape(bytes: &[u8]) -> Vec<u8> {
    // Tag, project, started, purpose, no span, one lesson, its started and finished: then picked.
    let picked = 1 + 8 + 8 + 1 + 4 + 4 + 8 + 8;
    assert_eq!(bytes[picked], 2, "a skill's lesson");
    let mut old = bytes[..picked].to_vec();
    old.extend_from_slice(&[3, 1, 4]);
    old.extend_from_slice(&bytes[picked + 1..]);
    let number = [&[46], 0.25f64.to_le_bytes().as_slice()].concat();
    let at = (old.windows(number.len()))
        .position(|window| window == number)
        .expect("the number");
    old[at] = 47;
    old
}

/// The projects `library` lists, the one touched last first.
fn listed(library: &mut Library) -> Vec<ProjectId> {
    (library.list().expect("listed").iter())
        .map(|project| project.id)
        .collect()
}

/// The library at `dir`: the frozen studio, then practice of every kind beside it. Returns the
/// artworks' projects and the sheets' (the unreadable one's last).
fn authors_shape(dir: &Path) -> (Vec<ProjectId>, Vec<u64>) {
    let frozen = std::fs::read(sample_path()).expect("the format-1 sample");
    write_records(dir, &decode(&frozen));
    assert!(encode(&records(dir)) == frozen, "the studio as frozen");
    let mut library = Library::at(dir);
    let artworks = listed(&mut library);
    let projects = sheets_of_every_kind(&mut library);
    timed_and_paired(&mut library);
    let read: Vec<u64> = (sheets(&mut library).expect("read").iter())
        .map(|sheet| sheet.id)
        .collect();
    assert_eq!(read, [1, 2, 3, 4], "the old record is not read");
    (artworks, projects)
}

/// A day's sheet, a test's, a colour copy, a drawing from life and a sheet of the old shape,
/// each on its project: the projects, oldest first.
fn sheets_of_every_kind(library: &mut Library) -> Vec<u64> {
    let mut day = started(library, &lesson(1, Purpose::Day, NOON), &named("练习 1"));
    day.lessons[0].finished = Some(NOON + 600);
    save(library, &day).expect("its record");
    let test = started(library, &lesson(2, Purpose::Test, NOON + 700), &untitled());
    let item = |pack: &str, item: &str| ItemRef {
        pack: pack.to_owned(),
        item: item.to_owned(),
    };
    let copy = Sheet::copy(3, (5, NOON + 800), item("openpaint-shapes", "teapot"));
    let copy = started(library, &copy, &named("Teapot"));
    let life = Sheet::from_life(4, (6, NOON + 900), item("personal", "mine-1"));
    let life = started(library, &life, &named("写生 1"));
    let old = started(
        library,
        &lesson(5, Purpose::Day, NOON + 1000),
        &named("练习 3"),
    );
    let key = [&[PRACTICE.byte()], 5u64.to_be_bytes().as_slice()].concat();
    let stored = library
        .records(PRACTICE, &5u64.to_be_bytes())
        .expect("its record");
    let old_bytes = of_old_shape(&stored[0].1);
    library
        .put_records(&[(PRACTICE, (key, old_bytes))])
        .expect("an old record");
    [day, test, copy, life, old]
        .map(|sheet| sheet.canvas)
        .to_vec()
}

/// The Pomodoro's settings.
const TIMING: Timing = Timing {
    length: Length::Forty,
    chime: true,
};

/// The Pomodoro's settings, a block completed and one under way, and a blind pair answered.
fn timed_and_paired(library: &mut Library) {
    let timing = TIMING;
    let completed = Block {
        started: NOON,
        day: 20_731,
        length: Length::Forty,
    };
    let write = TimerWrite {
        timing,
        under_way: None,
        completed: Some(completed),
    };
    save_timer(library, &write).expect("a block completed");
    let under_way = UnderWay {
        started: NOON + 3000,
        day: 20_731,
        length: Length::Forty,
        left: 600,
        paused: false,
    };
    let write = TimerWrite {
        under_way: Some(under_way),
        completed: None,
        ..write
    };
    save_timer(library, &write).expect("a block under way");
    let pair = BlindPair {
        asked: NOON + 1200,
        lesson: (1, 0),
        older: CopyRef {
            sheet: 2,
            exercise: 1,
        },
        newer: CopyRef {
            sheet: 1,
            exercise: 1,
        },
        older_left: true,
        answer: Answer::Alike,
    };
    put_pair(library, &pair).expect("a pair");
}

/// `before` as the clear leaves it: practice's spaces gone but the Pomodoro's settings, and
/// every record of `projects` gone, `last` with them when it names one.
fn without_practice(before: &Records, projects: &[u64]) -> Records {
    let named = |key: &[u8], kind: u8| {
        key.first() == Some(&kind)
            && (key.get(1..9)).is_some_and(|id| projects.iter().any(|p| id == p.to_be_bytes()))
    };
    let last = |value: &[u8]| projects.iter().any(|p| value == p.to_le_bytes());
    let gone = |(keyspace, key, value): &(String, Vec<u8>, Vec<u8>)| {
        if keyspace == "projects" {
            return (key.get(..8)).is_some_and(|id| projects.iter().any(|p| id == p.to_be_bytes()));
        }
        let practice = matches!(key.first(), Some(b'p' | b'g' | b'b'));
        let blocks = key.first() == Some(&b't') && key.as_slice() != b"ts";
        practice
            || blocks
            || named(key, b'a')
            || named(key, b'v')
            || (key == b"last" && last(value))
    };
    before
        .iter()
        .filter(|record| !gone(record))
        .cloned()
        .collect()
}

/// No practice left in `library` but the Pomodoro's settings `timing`: no sheet, pair or block,
/// and the practice log made of it lays the first lesson.
fn no_practice_left(library: &mut Library, timing: Timing) {
    assert_eq!(sheets(library), Ok(Vec::new()));
    assert_eq!(pairs(library), Ok(Vec::new()));
    let left = Blocks {
        timing,
        ..Blocks::default()
    };
    assert_eq!(blocks(library), Ok(left), "the settings kept, no block");
    let log = PracticeLog::new(Vec::new(), Vec::new(), Vec::new());
    assert!(log.path().first, "the home lays the first lesson");
}

#[test]
fn the_clear_removes_every_practice_record_and_sheet_and_leaves_every_other_byte() {
    let dir = TestDir::new("practice-clear");
    let (artworks, sheet_projects) = authors_shape(&dir);
    let before = records(&dir);
    let mut library = Library::at(&dir);
    let old = ProjectId::new(sheet_projects[4]);
    let unread = listed(&mut library).contains(&old);
    assert!(
        unread,
        "the unreadable sheet's project, which the home would list"
    );
    let files: Vec<Vec<u8>> = (artworks.iter())
        .map(|id| library.artwork_file(*id).expect("its file"))
        .collect();

    let cleared = clear(&mut library).expect("cleared");
    let counts = (
        cleared.sheets,
        cleared.givens,
        cleared.blocks,
        cleared.pairs,
    );
    assert_eq!(counts, (5, 5, 2, 1), "every sheet's record, read or not");
    let projects: Vec<u64> = cleared.projects.iter().map(|id| id.get()).collect();
    assert_eq!(
        projects, sheet_projects,
        "every sheet's project, oldest first"
    );
    // What the new reader reads: the artworks alone, as they were; no practice.
    assert_eq!(listed(&mut library), artworks);
    for (id, file) in artworks.iter().zip(&files) {
        assert_eq!(&library.artwork_file(*id).expect("its file"), file);
    }
    no_practice_left(&mut library, TIMING);
    let again = clear(&mut library);
    assert_eq!(
        again,
        Ok(Default::default()),
        "nothing left: nothing written"
    );
    drop(library);

    let after = records(&dir);
    let expected = without_practice(&before, &projects);
    assert_eq!(after, expected, "every other record's bytes");
    let opened = Library::at(&dir).open_last().expect("opens");
    assert!(opened.is_none(), "no sheet left open");
}
