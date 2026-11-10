//! Sheets of S82's generated colour combinations (`Kind::Areas`, painted from 1.5.0 to 1.18.1;
//! never planned since S101), frozen (`tests/samples/areas-records.bin`) as S120 writes them
//! beside a painting: a stage-2 session holding two among its mix and a drill of them, each with
//! its project, its lesson's givens, its firsts, results and time. Read by this build, every count
//! the practice book, the time and the seals take from them is what it was, each combination's
//! original its generated picture again. The sample is what an upgrade retiring the kind must
//! keep these facts for (S108).
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
    Arrangement, Book, Combination, Exercise, Family, First, GeneratedAreas, Givens, Judge, Kind,
    Layout, LineTier, Metric, Original, Outcome, Plan, ScaleTone, Sheet, Span, Tally, summary,
};
use openpaint_practice_store::{GIVENS, decode_givens, save, sheets, start};
use openpaint_store::Library;

/// The dump's records, as `tests/sample/mod.rs` names them.
type Records = Vec<(String, Vec<u8>, Vec<u8>)>;

/// 2026-10-27 08:00 UTC, when the session began.
const BEGUN: u64 = 1_793_088_000;

fn sample_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/samples/areas-records.bin")
}

fn artwork(pairs: usize) -> Artwork {
    let [width, height] = Layout::of(&vec![None; pairs]).size;
    Artwork::new(CanvasSize::new(width, height).expect("size"))
}

/// Exercise `index` of `sheet` first compared at `at`, the next stroke `next`, with `metrics`,
/// its result `result`.
fn compared(
    sheet: &mut Sheet,
    (index, at, next): (usize, u64, u64),
    metrics: Vec<(Metric, f64)>,
    result: Option<Outcome>,
) {
    let laid = sheet.laid_mut(index).expect("an exercise");
    laid.compared(First::of(at, next, metrics));
    laid.result = result.map(|outcome| (outcome, Judge::App));
}

fn areas(delta: f64, weak: f64, strong: f64) -> Vec<(Metric, f64)> {
    vec![
        (Metric::AreasDelta, delta),
        (Metric::AreasWeak, weak),
        (Metric::AreasMadeStrong, strong),
    ]
}

/// Session 3 of stage 2: a line as its warm-up, then combined lines, two generated combinations
/// and a value scale, finished; both combinations compared (near, clean), one corrected and
/// compared again.
fn session() -> Sheet {
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
    let mut sheet = Sheet::session(1, 2, BEGUN, plan);
    compared(&mut sheet, (0, BEGUN + 120, 3), Vec::new(), None);
    compared(&mut sheet, (1, BEGUN + 400, 9), Vec::new(), None);
    let near = Some(Outcome::Near);
    compared(
        &mut sheet,
        (2, BEGUN + 900, 21),
        areas(0.062, 3.0, 1.0),
        near,
    );
    let clean = Some(Outcome::Clean);
    compared(
        &mut sheet,
        (3, BEGUN + 1500, 34),
        areas(0.041, 2.0, 0.0),
        clean,
    );
    sheet.laid_mut(3).expect("a combination").compares = 2;
    let lesson = sheet.lesson_mut().expect("its lesson");
    lesson.at = 4;
    lesson.finished = Some(BEGUN + 1900);
    sheet.spans = vec![
        Span {
            from: BEGUN,
            to: BEGUN + 1000,
        },
        Span {
            from: BEGUN + 1300,
            to: BEGUN + 1900,
        },
    ];
    sheet
}

/// Drill 1 of colour combinations, two generated ones, the first compared (redo: two weak
/// contrasts made strong).
fn drill() -> Sheet {
    let plan = Plan {
        exercises: vec![
            Exercise::generated(Kind::Areas(Arrangement::Patches), 0x5EED_0108_0011),
            Exercise::generated(Kind::Areas(Arrangement::Strata), 0x5EED_0108_0012),
        ],
        warm_up: 0,
    };
    let started = BEGUN + 86_400;
    let mut drill = Sheet::drill(2, started, plan);
    let metrics = vec![(Metric::AreasDelta, 0.08), (Metric::AreasMadeStrong, 2.0)];
    compared(
        &mut drill,
        (0, started + 300, 6),
        metrics,
        Some(Outcome::Redo),
    );
    drill.lesson_mut().expect("its lesson").at = 1;
    drill.spans = vec![Span {
        from: started,
        to: started + 420,
    }];
    drill
}

/// What `sheet`'s lesson printed.
fn givens(sheet: &Sheet) -> Givens {
    Givens::of_lesson(sheet.lesson().expect("its lesson"), &[])
}

/// A library holding a painting, the two sheets, their projects and givens, as its records.
fn written() -> Records {
    let dir = TestDir::new("areas-write");
    let mut library = Library::at(&dir);
    let painting = Artwork::new(CanvasSize::new(64, 48).expect("size"));
    library.create(&painting, &untitled()).expect("a painting");
    for (sheet, title) in [(session(), "Practice 3"), (drill(), "Drill 1")] {
        let title = Title::new(title).expect("title");
        let pairs = sheet.exercises().count();
        let printed = givens(&sheet);
        let id = start(&mut library, (&artwork(pairs), &title), &sheet, &printed).expect("started");
        let kept = Sheet {
            canvas: id.get(),
            ..sheet
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
        "storing the generated combinations' sheets no longer gives the frozen bytes"
    );
}

/// The book, the time, the seals and the session's summary as the records give them, and each
/// combination's original its generated picture again, as its givens printed it.
#[test]
fn generated_combinations_count_and_show_as_they_did() {
    let dir = TestDir::new("areas-read");
    write_records(&dir, &frozen());
    let mut library = Library::at(&dir);
    let read = sheets(&mut library).expect("practice");
    assert_eq!(
        read,
        [session(), drill()].map(|sheet| Sheet {
            canvas: sheet.id + 1,
            ..sheet
        })
    );
    let listed = library.list().expect("list");
    assert_eq!(listed.len(), 3, "the painting and the two sheets");
    for sheet in &read {
        let id = openpaint_app::ProjectId::new(sheet.canvas);
        assert_eq!(library.artwork(id), Ok(artwork(sheet.exercises().count())));
        let kept = library
            .records(GIVENS, &sheet.canvas.to_be_bytes())
            .expect("givens");
        assert_eq!(kept.len(), 1);
        assert_eq!(decode_givens(&kept[0].1), Ok(givens(sheet)));
    }

    let book = Book::of(&read, 2, 480);
    assert_eq!((book.sessions, book.seconds), (1, 1600 + 420));
    assert_eq!(book.road.done_in(2), 1);
    assert_eq!(book.days.len(), 2);
    let tally = Tally::of(&read);
    assert_eq!(
        (tally.sessions, tally.stages[1], tally.lines, tally.colours),
        (1, 1, 2, 0)
    );
    assert_eq!(book.seals.len(), tally.seals().len());

    let rows = summary(&read[0], &[]);
    let areas = rows.iter().find(|row| row.family == Family::Areas);
    let areas = areas.expect("combinations in the summary");
    assert_eq!((areas.checked, areas.held), (2, 2));
    let delta = areas
        .rows
        .iter()
        .find(|(metric, ..)| *metric == Metric::AreasDelta);
    let mean = delta.and_then(|(_, this, _)| *this).expect("a mean");
    assert!((mean - 0.0515).abs() < 1e-12, "{mean}");
    assert_eq!(read[1].drilled(), Some(Family::Areas));

    for laid in read.iter().flat_map(Sheet::exercises) {
        let Kind::Areas(arrangement) = laid.exercise.kind else {
            continue;
        };
        let generated = GeneratedAreas::generate(arrangement, laid.exercise.seed);
        assert_eq!(laid.exercise.original(), Original::Areas(generated));
    }
}
