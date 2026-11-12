//! What every kind's test takes: the sheet it lays (the exercises it needs, as a full lesson of
//! them), the opening the log makes of it, and the sheet a session holds and its intents, as the
//! runtime reaches them. A lesson of a skill as the path starts it is `Start::At`'s
//! (`PracticeLog::start`).

use openpaint_app::{Session, Title};
use openpaint_practice::{Exercise, Given, Pack, Picked, Planned, Purpose, Sheet, SkillLevel};
use openpaint_practice_app::{PracticeIntent, PracticeLog, PracticeSheet, Practising, Refused};

/// A day's sheet, the first of all, begun at `started`, holding `exercises` as a full lesson of
/// unit 1 without a warm-up, each at the first level: what a kind's test lays.
pub fn laid(exercises: Vec<Exercise>, started: u64) -> Sheet {
    let planned = Planned {
        picked: Picked::Full {
            stage: 1,
            warm_up: 0,
        },
        exercises: (exercises.into_iter())
            .map(|exercise| (exercise, SkillLevel::First))
            .collect(),
    };
    Sheet::begun(1, (Purpose::Day, started), planned)
}

/// `record` opening on a canvas of `size`, untitled, as a log keeping `packs` and `sheets` opens
/// it, what it prints as its generators print it now.
pub fn opening(
    record: &Sheet,
    (packs, sheets): (&[Pack], Vec<Sheet>),
    size: [u32; 2],
) -> Practising {
    let log = PracticeLog::new(sheets, Vec::new(), packs.to_vec());
    let printed = Given::of_sheet(record, packs);
    log.practising(record, &printed, (size, &Title::untitled()))
}

/// The practice sheet a session holds and its intents, as methods of the session.
pub trait OnSheet {
    /// The sheet, which the test opened.
    fn sheet(&mut self) -> &mut PracticeSheet;

    /// `intent` applied to the sheet at `now` (`openpaint_practice_app::practise`).
    fn practise(&mut self, intent: PracticeIntent, now: u64) -> Result<(), Refused>;
}

impl OnSheet for Session {
    fn sheet(&mut self) -> &mut PracticeSheet {
        PracticeSheet::of_mut(self).expect("a practice sheet")
    }

    fn practise(&mut self, intent: PracticeIntent, now: u64) -> Result<(), Refused> {
        openpaint_practice_app::practise(self, intent, now)
    }
}
