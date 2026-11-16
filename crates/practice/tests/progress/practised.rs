//! A learner with unit 1's skills mixed in a lesson on three days (S149), their lines and
//! combined lines compared but never judged, their colours judged by the app; then a day of
//! lessons of lines and of combined lines laid and left; no pair. Every skill practised is on the Progress page in the course's order, one with
//! nothing judged without a day; a lesson of lines judged by eye gives lines their curve. The first
//! lesson alone puts nothing there.

use openpaint_practice::{
    Exercise, First, Judge, Kind, Outcome, Picked, Planned, Progress, Purpose, Sheet, Skill,
    SkillLevel, local_day,
};

use crate::sim::{DAY, NOON};

/// What became of an exercise laid.
#[derive(Debug, Clone, Copy)]
enum Done {
    Laid,
    Compared,
    Judged(Outcome, Judge),
}

/// A kind of the skill stored as `code`: lines 1, colours 2, combined lines 3.
fn of(code: u8) -> Kind {
    Kind::of_code(code, 0).expect("a kind")
}

/// Sheet `id` on day `day`: one lesson picked `picked` laying `laid`, each as `Done` says.
fn sheet(id: u64, day: u64, picked: Picked, laid: &[(u8, Done)]) -> Sheet {
    let at = NOON + day * DAY;
    let exercises = (laid.iter().enumerate())
        .map(|(i, (code, _))| {
            (
                Exercise::generated(of(*code), id * 32 + i as u64),
                SkillLevel::First,
            )
        })
        .collect();
    let mut sheet = Sheet::begun(id, (Purpose::Day, at), Planned { picked, exercises });
    sheet.canvas = id;
    for (index, (_, done)) in laid.iter().enumerate() {
        let exercise = sheet.laid_mut(index).expect("an exercise");
        let first = First::of(at + 60 * index as u64, 1, Vec::new());
        match *done {
            Done::Laid => {}
            Done::Compared => exercise.compared(first),
            Done::Judged(outcome, judge) => {
                exercise.compared(first);
                exercise.result = Some((outcome, judge));
            }
        }
    }
    sheet
}

/// Three days of unit 1's skills mixed in a lesson, then a day of lessons left.
fn learner() -> Vec<Sheet> {
    let mixed = Picked::Chosen;
    let app = |outcome| Done::Judged(outcome, Judge::App);
    let (line, colour, combined) = (1, 2, 3);
    let first = [
        (line, Done::Compared),
        (line, Done::Compared),
        (line, Done::Laid),
        (combined, Done::Compared),
        (colour, app(Outcome::Near)),
        (colour, app(Outcome::Clean)),
        (colour, app(Outcome::Redo)),
    ];
    let second = [
        (line, Done::Compared),
        (combined, Done::Compared),
        (colour, app(Outcome::Near)),
    ];
    vec![
        sheet(1, 0, mixed, &first),
        sheet(2, 1, mixed, &second),
        sheet(
            3,
            5,
            mixed,
            &[(line, Done::Compared), (colour, app(Outcome::Clean))],
        ),
        sheet(4, 24, Picked::Chosen, &[(line, Done::Laid); 6]),
        sheet(5, 24, Picked::Chosen, &[(combined, Done::Laid); 3]),
    ]
}

fn day(day: u64) -> i64 {
    local_day(NOON + day * DAY, 0)
}

fn keys(progress: &Progress) -> Vec<&'static str> {
    progress
        .curves
        .iter()
        .map(|curve| curve.skill.key())
        .collect()
}

fn skill(key: &str) -> Skill {
    Skill::of_key(key).expect("a skill")
}

#[test]
fn every_skill_practised_is_on_the_page_in_the_course_s_order_judged_or_not() {
    let progress = Progress::of(&learner(), &[], 0);
    assert_eq!(keys(&progress), ["line", "combined", "colour"]);
    for bare in ["line", "combined"] {
        let curve = progress.curve(skill(bare)).expect("practised");
        assert!(curve.days.is_empty(), "{bare}: {:?}", curve.days);
        assert!(curve.ability.is_empty() && curve.band.is_empty() && curve.levels.is_empty());
        assert_eq!(curve.retest, None);
    }
    let colours = progress.curve(skill("colour")).expect("colours");
    assert_eq!(colours.days, [day(0), day(1), day(5)]);
}

#[test]
fn a_lesson_judged_by_eye_gives_its_skill_a_curve_in_its_place() {
    let mut sheets = learner();
    let eye = |outcome| Done::Judged(outcome, Judge::Eye);
    let judged = [
        (1, eye(Outcome::Near)),
        (1, eye(Outcome::Clean)),
        (1, Done::Compared),
        (1, Done::Laid),
    ];
    sheets.push(sheet(6, 26, Picked::Chosen, &judged));
    let progress = Progress::of(&sheets, &[], 0);
    assert_eq!(keys(&progress), ["line", "combined", "colour"]);
    let lines = progress.curve(skill("line")).expect("lines");
    assert_eq!(lines.days, [day(26)], "only what was judged");
    assert_eq!(
        (lines.ability.as_slice(), lines.band.as_slice()),
        (&[0.0][..], &[0.0][..])
    );
    sheets.push(sheet(7, 30, Picked::Chosen, &[(1, eye(Outcome::Clean)); 2]));
    let progress = Progress::of(&sheets, &[], 0);
    let lines = progress.curve(skill("line")).expect("lines");
    assert_eq!(lines.days, [day(26), day(30)]);
    assert!(lines.ability[1] > 0.0, "two clean read as a rise");
    let combined = progress.curve(skill("combined")).expect("combined lines");
    assert!(combined.days.is_empty());
}

#[test]
fn the_first_lesson_alone_puts_no_skill_on_the_page() {
    let first = Sheet::begun(
        1,
        (Purpose::Day, NOON),
        Planned {
            picked: Picked::First,
            exercises: vec![
                (Exercise::generated(Kind::Free, 1), SkillLevel::First),
                (Exercise::generated(of(1), 2), SkillLevel::First),
            ],
        },
    );
    assert!(Progress::of(&[first], &[], 0).curves.is_empty());
}
