//! The log counts lessons, not sheets (S121b): a sheet holding a drill and a session appended
//! after it counts both on the Today card, in the next session's and drill's numbers, in the
//! summary's earlier lessons and beside the seals a finish stamps.

use openpaint_practice::{Exercise, Givens, Kind, LineTier, Plan, Sheet, stage_one};
use openpaint_practice_app::PracticeLog;

/// 2026-10-05 12:00 UTC.
const NOON: u64 = 1_791_201_600;

/// Sheet 1 on project 40: a drill of two single lines, then a session of stage 1 appended, both
/// finished.
fn day_sheet() -> Sheet {
    let exercises = (0..2)
        .map(|seed| Exercise::generated(Kind::Line(LineTier::Arc), seed))
        .collect();
    let mut sheet = Sheet::drill(
        1,
        NOON,
        Plan {
            exercises,
            warm_up: 0,
        },
    );
    sheet.canvas = 40;
    sheet.lessons[0].finished = Some(NOON + 300);
    let session = Sheet::session(0, 1, NOON + 600, stage_one(5, &[]));
    sheet.append(session.lessons[0].clone(), Givens::default());
    sheet.lessons[1].finished = Some(NOON + 900);
    sheet
}

#[test]
fn a_sheet_s_drill_and_session_count_each_as_a_lesson() {
    let day = day_sheet();
    let log = PracticeLog::new(vec![day.clone()], Vec::new(), Vec::new());
    let today = log.today();
    assert_eq!((today.stage, today.done, today.open), (1, 1, None));
    assert_eq!(log.number(&day), 1, "its lesson in hand: the first session");
    let (next, title) = log.begin((1, None), 6, NOON + 1200);
    assert_eq!(
        (next.id, log.number(&next), title.as_str()),
        (2, 2, "Practice 2")
    );
    assert_eq!(
        log.before(&next),
        [day.lessons[1].clone()],
        "the session, not the drill"
    );
    let (drill, title) =
        (log.begin_drill(openpaint_practice::Family::Line, 7, NOON + 1200)).expect("a drill");
    assert_eq!((log.number(&drill), title.as_str()), (2, "Drill 2"));
    assert_eq!(
        log.before(&drill),
        [day.lessons[0].clone()],
        "the drill, not the session"
    );
    let others = log.tally_beside(&day);
    assert_eq!(others.sessions, 0, "every lesson but the one in hand");
}
