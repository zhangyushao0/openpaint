//! Records of S82's generated colour combinations (`Kind::Areas`, painted from 1.5.0 to 1.18.1;
//! never planned since S101), frozen (`tests/samples/areas-records.bin`) as 1.18.1 to 1.21.1 write
//! them beside a painting: a stage-2 session holding two among its mix and a drill of them, each
//! with its sheet, results and time. Read by this build, every count the practice book, the time
//! and the seals take from them is what it was, each combination's original its generated picture
//! again. The sample is what an upgrade retiring the kind must keep these facts for (S108); since
//! S110 it is written through the library's record space and read through this crate.
//!
//! The dump format is the store's format-1 sample's (`store/tests/sample/dump.rs`, its helpers
//! taken by path); a missing sample is written
//! from the current encoding and the test fails, so a new sample is reviewed before it is
//! committed.

#[path = "../../store/tests/common/mod.rs"]
mod common;
#[path = "../../store/tests/sample/dump.rs"]
mod dump;

use std::path::PathBuf;

use common::{TestDir, records, untitled, write_records};
use openpaint_app::{ProjectStore, Title};
use openpaint_core::{Artwork, CanvasSize};
use openpaint_practice::{
    Arrangement, Book, Combination, Exercise, ExerciseResult, Family, GeneratedAreas, Kind,
    LineTier, Metric, Original, Plan, ScaleTone, SessionRecord, Sheet, Span, Tally, summary,
};
use openpaint_practice_store::{save, sessions, start};
use openpaint_store::Library;

/// The dump's records, as `tests/sample/mod.rs` names them.
type Records = Vec<(String, Vec<u8>, Vec<u8>)>;

/// 2026-10-27 08:00 UTC, when the session began.
const BEGUN: u64 = 1_793_088_000;

fn sample_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/samples/areas-records.bin")
}

fn sheet(pairs: usize) -> Artwork {
    let [width, height] = Sheet::of(&vec![None; pairs]).size;
    Artwork::new(CanvasSize::new(width, height).expect("size"))
}

/// Session 3 of stage 2: a line as its warm-up, then combined lines, two generated combinations
/// and a value scale, finished; both combinations compared, one corrected and compared again.
fn session() -> SessionRecord {
    let plan = Plan {
        exercises: vec![
            Exercise::generated(Kind::Line(LineTier::ALL[1]), 0x5EED_0108_0001),
            Exercise::generated(Kind::Combined(Combination::ALL[0]), 0x5EED_0108_0002),
            Exercise::generated(Kind::Areas(Arrangement::Strata), 0x5EED_0108_0003),
            Exercise::generated(Kind::Areas(Arrangement::Blocks), 0x5EED_0108_0004),
            Exercise::generated(Kind::Scale(ScaleTone::ALL[1], 9), 0x5EED_0108_0005),
        ],
        warm_up: 1,
    };
    let mut session = SessionRecord::begun(1, (2, 3), (0x5EED_0108, BEGUN), plan);
    let areas = |delta, weak, strong| {
        vec![
            (Metric::AreasDelta, delta),
            (Metric::AreasWeak, weak),
            (Metric::AreasMadeStrong, strong),
        ]
    };
    session.results = vec![
        ExerciseResult::first(0, BEGUN + 120, 3, Vec::new()),
        ExerciseResult::first(1, BEGUN + 400, 9, Vec::new()),
        ExerciseResult::first(2, BEGUN + 900, 21, areas(0.062, 3.0, 1.0)),
        ExerciseResult {
            compares: 2,
            ..ExerciseResult::first(3, BEGUN + 1500, 34, areas(0.041, 2.0, 0.0))
        },
    ];
    session.current = 4;
    session.finished = Some(BEGUN + 1900);
    session.time = vec![
        Span {
            from: BEGUN,
            to: BEGUN + 1000,
        },
        Span {
            from: BEGUN + 1300,
            to: BEGUN + 1900,
        },
    ];
    session
}

/// Drill 1 of colour combinations, two generated ones, the first compared.
fn drill() -> SessionRecord {
    let plan = Plan {
        exercises: vec![
            Exercise::generated(Kind::Areas(Arrangement::Patches), 0x5EED_0108_0011),
            Exercise::generated(Kind::Areas(Arrangement::Strata), 0x5EED_0108_0012),
        ],
        warm_up: 0,
    };
    let started = BEGUN + 86_400;
    let mut drill = SessionRecord::drill(2, 1, (0x5EED_0108_0010, started), plan);
    drill.results = vec![ExerciseResult::first(
        0,
        started + 300,
        6,
        vec![(Metric::AreasDelta, 0.08), (Metric::AreasMadeStrong, 2.0)],
    )];
    drill.current = 1;
    drill.time = vec![Span {
        from: started,
        to: started + 420,
    }];
    drill
}

/// A library holding a painting, the two records and their sheets, as its records.
fn written() -> Records {
    let dir = TestDir::new("areas-write");
    let mut library = Library::at(&dir);
    let painting = Artwork::new(CanvasSize::new(64, 48).expect("size"));
    library.create(&painting, &untitled()).expect("a painting");
    for (record, title) in [(session(), "Practice 3"), (drill(), "Drill 1")] {
        let title = Title::new(title).expect("title");
        let pairs = record.exercises.len();
        let id = start(&mut library, (&sheet(pairs), &title), &record).expect("started");
        let kept = SessionRecord {
            canvas: id.get(),
            ..record
        };
        save(&mut library, &kept).expect("saved");
    }
    drop(library);
    records(&dir)
}

/// The frozen sample's bytes, written first when missing (the test then fails).
fn frozen() -> Records {
    let path = sample_path();
    let Ok(bytes) = std::fs::read(&path) else {
        std::fs::write(&path, dump::encode(&written())).expect("write sample");
        panic!(
            "wrote a new sample at {}; review it and commit it",
            path.display()
        );
    };
    dump::decode(&bytes)
}

#[test]
fn the_areas_sample_is_what_this_build_writes() {
    assert!(
        written() == frozen(),
        "storing the generated combinations' records no longer gives the frozen bytes"
    );
}

/// The book, the time, the seals and the session's summary as the records give them, and each
/// combination's original its generated picture again.
#[test]
fn generated_combinations_count_and_show_as_they_did() {
    let dir = TestDir::new("areas-read");
    write_records(&dir, &frozen());
    let mut library = Library::at(&dir);
    let records = sessions(&mut library).expect("practice");
    assert_eq!(
        records,
        [session(), drill()].map(|r| SessionRecord {
            canvas: r.id + 1,
            ..r
        })
    );
    let listed = library.list().expect("list");
    assert_eq!(listed.len(), 3, "the painting and the two sheets");
    for record in &records {
        let id = openpaint_app::ProjectId::new(record.canvas);
        assert_eq!(library.artwork(id), Ok(sheet(record.exercises.len())));
    }

    let book = Book::of(&records, 2, 480);
    assert_eq!((book.sessions, book.seconds), (1, 1600 + 420));
    assert_eq!(book.road.done_in(2), 1);
    assert_eq!(book.days.len(), 2);
    let tally = Tally::of(&records);
    assert_eq!(
        (tally.sessions, tally.stages[1], tally.lines, tally.colours),
        (1, 1, 2, 0)
    );
    assert_eq!(book.seals.len(), tally.seals().len());

    let rows = summary(&records[0], &[]);
    let areas = rows.iter().find(|row| row.family == Family::Areas);
    let areas = areas.expect("combinations in the summary");
    assert_eq!((areas.checked, areas.held), (2, 2));
    let delta = areas
        .rows
        .iter()
        .find(|(metric, ..)| *metric == Metric::AreasDelta);
    let mean = delta.and_then(|(_, this, _)| *this).expect("a mean");
    assert!((mean - 0.0515).abs() < 1e-12, "{mean}");
    assert_eq!(records[1].drilled(), Some(Family::Areas));

    for exercise in records.iter().flat_map(|record| &record.exercises) {
        let Kind::Areas(arrangement) = exercise.kind else {
            continue;
        };
        let generated = GeneratedAreas::generate(arrangement, exercise.seed);
        assert_eq!(exercise.original(), Original::Areas(generated));
    }
}
