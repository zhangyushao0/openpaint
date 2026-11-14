//! The Pomodoro (S124) on a clock the test hands in: the count only while a day's or a test's
//! sheet is in front, held everywhere else, pause, continue and stop, the minutes shown and the
//! frames the timer wants, the writes, a held block dropped on a later day, a restart
//! (`clock.rs`); the chain of lessons, the time's end and End's cut, the break and its chime,
//! Another pomodoro (`block.rs`); the screen at Time, the app hidden during the break
//! (`away.rs`).

use std::sync::Arc;

use openpaint_app::{
    BrushLibrary, Contact, ContactPhase, Device, InputSample, Intent, Session, StrokeIntent, Title,
    Toolbox, Viewport,
};
use openpaint_core::{Artwork, CanvasSize, Color, StrokeStyle};
use openpaint_practice::{
    Exercise, Given, Kind, LineTier, Picked, Planned, Purpose, Sheet, SkillLevel, local_day,
};
use openpaint_practice_app::{
    BlockShown, PracticeIntent, PracticeLog, PracticePens, PracticeProjection, PracticeSheet,
    open_sheet, practise,
};

mod away;
mod block;
mod clock;

const VIEWPORT: Viewport = Viewport {
    origin: [0.0, 84.0],
    size: [2880.0, 1716.0],
    pixels_per_point: 1.75,
};
/// 2026-10-05 12:00 UTC, and a day.
const NOON: u64 = 1_791_201_600;
const DAY: u64 = 86_400;

/// The log of `sheets` on the clock at `now`, UTC.
fn log_of(sheets: Vec<Sheet>, now: u64) -> PracticeLog {
    let mut log = PracticeLog::new(sheets, Vec::new(), Vec::new());
    log.set_clock(now, 0);
    log
}

/// The frame's clock at `now`, `front` the record of the sheet in front, as the runtime ticks.
fn tick(log: &mut PracticeLog, now: u64, front: Option<&Sheet>) {
    log.set_clock(now, 0);
    log.tick(now, front);
}

/// The local day of `at`, UTC.
fn day(at: u64) -> i64 {
    local_day(at, 0)
}

/// A lesson of six lines (two rows), the next sheet of `log` begun at `started` on project
/// `canvas`, for `purpose`.
fn six_lines(log: &PracticeLog, (started, canvas): (u64, u64), purpose: Purpose) -> Sheet {
    let exercises = (0..6)
        .map(|seed| Exercise::generated(Kind::Line(LineTier::Arc), seed))
        .map(|exercise| (exercise, SkillLevel::First))
        .collect();
    let planned = Planned {
        picked: Picked::Chosen,
        exercises,
    };
    let lesson = Sheet::begun(log.next_id(), (purpose, started), planned);
    Sheet { canvas, ..lesson }
}

/// `record` open on a canvas of `size` as `log` opens it.
fn practising(record: &Sheet, size: [u32; 2], log: &PracticeLog) -> Session {
    let sheet = Artwork::new(CanvasSize::new(size[0], size[1]).expect("size"));
    let library = BrushLibrary::built_in().expect("built-in brushes");
    let style = Toolbox::new(library).style().expect("style");
    let brush =
        StrokeStyle::new(Arc::clone(style.recipe()), 8.0, 1.0, Color::BLACK).expect("brush");
    let pens = PracticePens::built_in().expect("pens");
    let printed = Given::of_sheet(record, &[]);
    let opening = log.practising(record, &printed, (size, &Title::untitled()));
    open_sheet(sheet, (brush, pens), opening, &VIEWPORT)
}

fn shown(session: &Session) -> PracticeProjection {
    PracticeProjection::of(session).expect("a sheet")
}

fn record_of(session: &Session) -> Sheet {
    PracticeSheet::of(session)
        .expect("a sheet")
        .record()
        .clone()
}

/// A pen stroke across the shown exercise's copy frame: the exercise begun.
fn drawn(session: &mut Session) {
    let practice = shown(session);
    let frame = practice.sheet.pairs[practice.current.expect("shown")].copy;
    let at = |x: f64| frame.to_canvas([x, 0.5]).map(|v| v as f32);
    let sample = |index: i64, x: f64| InputSample {
        position: at(x),
        pressure: 0.6,
        tilt: 0.2,
        azimuth: 0.0,
        time_ms: 1000 + 8 * index,
    };
    for (phase, samples) in [
        (ContactPhase::Down, vec![sample(0, 0.3)]),
        (ContactPhase::Move, vec![sample(1, 0.5)]),
        (ContactPhase::Up, vec![sample(2, 0.7)]),
    ] {
        let contact = Contact {
            id: 7,
            device: Device::Pen,
            phase,
            samples,
        };
        let intent = Intent::Stroke(StrokeIntent::Contact(contact));
        session.dispatch(intent).expect("dispatch");
    }
}

/// Next at `now`.
fn next(session: &mut Session, now: u64) {
    practise(session, PracticeIntent::Next(VIEWPORT), now).expect("next");
}

/// The block as the frame shows it: the minutes left and whether paused, while it runs.
fn running(log: &PracticeLog) -> Option<(u32, bool)> {
    match log.timer().block? {
        BlockShown::Running { left, paused, .. } => Some((left, paused)),
        _ => None,
    }
}
