//! The path's contract (S122): the units' skills and their keys, the dot rule, what a redo is and
//! when it is answered, what opens a unit, the first lesson; the schedule's order, the First-prefix
//! rule in `order.rs`; how a lesson's pictures turn in `pictures.rs`.

mod order;
mod pictures;
mod retest;

use openpaint_practice::{
    DOTS, Judge, Kind, Learner, Lesson, LineTier, Needs, Outcome, Picked, Pictures, Planned,
    Purpose, Sheet, Skill, SkillLevel, Standing, TileMark, UNITS, derived, schedule,
};

/// 2026-10-05 12:00 UTC.
const NOON: u64 = 1_791_201_600;
const CLEAN: [Outcome; 6] = [Outcome::Clean; 6];

fn skill(key: &str) -> Skill {
    Skill::of_key(key).expect("a skill of that key")
}

/// `planned` as a lesson, its first exercises ended as `outcomes` say.
fn ended(planned: Planned, outcomes: &[Outcome]) -> Lesson {
    let mut sheet = Sheet::begun(1, (Purpose::Day, NOON), planned);
    let mut lesson = sheet.lessons.remove(0);
    for (laid, outcome) in lesson.exercises.iter_mut().zip(outcomes) {
        laid.result = Some((*outcome, Judge::App));
    }
    lesson
}

/// `learner` after a lesson of the skill keyed `key` at `level`, ended as `outcomes` say.
fn after(learner: &Learner, key: &str, level: SkillLevel, outcomes: &[Outcome]) -> Learner {
    let pictures = Pictures::default();
    let standing = Standing {
        pictures: &pictures,
        learner,
    };
    let planned = schedule::lesson(skill(key), level, 7, &standing).expect("a lesson");
    learner.with(&ended(planned, outcomes))
}

#[test]
fn the_units_hold_thirty_one_skills_each_once_under_a_key_of_its_own() {
    let skills: Vec<Skill> = Skill::all().collect();
    assert_eq!(skills.len(), 31);
    let held: usize = UNITS.iter().map(|unit| unit.skills.len()).sum();
    assert_eq!(held, 31);
    let mut keys: Vec<&str> = skills.iter().map(|skill| skill.key()).collect();
    for (skill, key) in skills.iter().zip(&keys) {
        assert_eq!(Skill::of_key(key), Some(*skill), "{key}");
        let lined = |mark: &TileMark| matches!(mark, TileMark::Line(_));
        assert!(skill.glyph().iter().any(lined), "{key}");
    }
    keys.sort_unstable();
    keys.dedup();
    assert_eq!(keys.len(), 31);
    // A kind is its folder's skill; a long colour copy and the free scribble are none.
    assert_eq!(Skill::of(Kind::Line(LineTier::Hook)), Some(skill("line")));
    assert_eq!(
        (Skill::of(Kind::Study), Skill::of(Kind::Free)),
        (None, None)
    );
    // Unit 1 opens with ghosted lines (S156), then lines, combined lines and colours.
    let first: Vec<&str> = UNITS[0].skills.iter().map(|skill| skill.key()).collect();
    assert_eq!(first, ["ghosted-lines", "line", "combined", "colour"]);
    assert_eq!(skill("estimates").needs(), Needs::Nothing);
    // Without a pack a single shape lacks its pictures: never offered.
    let (pictures, learner) = (Pictures::default(), Learner::default());
    let standing = Standing {
        pictures: &pictures,
        learner: &learner,
    };
    assert_eq!(skill("shape").lacks(&pictures), Some(Needs::Pictures));
    assert_eq!(schedule::offer(skill("shape"), &standing), None);
}

#[test]
fn a_dot_is_earned_from_a_full_window_two_thirds_clean_without_a_redo_and_never_falls() {
    use Outcome::{Clean, Near, Redo};
    let lines = skill("line");
    assert_eq!(lines.window(), 6);
    let fresh = Learner::default();
    assert_eq!(
        (fresh.dots(lines), fresh.level(lines)),
        (0, SkillLevel::First)
    );
    // Three clean of six is under two thirds; four is enough once the window holds them.
    let half = after(
        &fresh,
        "line",
        SkillLevel::First,
        &[Clean, Near, Clean, Near, Clean, Near],
    );
    assert_eq!(half.dots(lines), 0);
    let one = after(
        &fresh,
        "line",
        SkillLevel::First,
        &[Clean, Clean, Clean, Clean, Near, Near],
    );
    assert_eq!((one.dots(lines), one.level(lines)), (1, SkillLevel::Second));
    // Exercises laid at another level than the skill's count for nothing.
    let below = after(&one, "line", SkillLevel::First, &CLEAN);
    assert_eq!(below.dots(lines), 1);
    // A redo in the window holds the dot back until six after it are in.
    let held = after(
        &one,
        "line",
        SkillLevel::Second,
        &[Clean, Clean, Redo, Clean, Clean, Clean],
    );
    assert_eq!(held.dots(lines), 1);
    let two = after(&held, "line", SkillLevel::Second, &CLEAN);
    assert_eq!((two.dots(lines), two.level(lines)), (2, SkillLevel::Third));
    // Dots never fall, and three is all.
    let bad = after(&two, "line", SkillLevel::Third, &[Redo; 6]);
    assert_eq!(bad.dots(lines), 2);
    let three = after(&bad, "line", SkillLevel::Third, &CLEAN);
    let more = after(&three, "line", SkillLevel::Third, &CLEAN);
    assert_eq!((three.dots(lines), more.dots(lines)), (DOTS, DOTS));
    assert_eq!(more.level(lines), SkillLevel::Third);
    assert_eq!((more.lessons(), more.lessons_of(lines)), (6, 6));
    assert_eq!(more.practised(lines), Some(6));
}

#[test]
fn a_redo_waits_until_its_kind_and_picture_are_laid_again_in_a_later_lesson() {
    use Outcome::{Clean, Redo};
    let fresh = Learner::default();
    let lined = after(&fresh, "line", SkillLevel::First, &[Clean, Redo]);
    let learner = after(&lined, "colour", SkillLevel::First, &[Redo]);
    let (lines, colours) = (skill("line"), skill("colour"));
    assert_eq!(learner.redos().count(), 2);
    assert_eq!(learner.redos().next().map(|(of, _)| of), Some(lines));
    let pictures = Pictures::default();
    let standing = Standing {
        pictures: &pictures,
        learner: &learner,
    };
    assert!(schedule::offer(colours, &standing).is_some_and(|offer| offer.redo));
    // The review is the oldest redo of another skill than the one offered next: two exercises,
    // the redo's own kind first.
    let next = schedule::next(&standing).expect("a lesson to offer");
    let review = schedule::review(&standing).expect("a review");
    assert_ne!(review.skill, next.skill);
    assert_eq!((review.exercises, review.redo), (2, true));
    let waiting = learner.redo(review.skill).expect("its redo").clone();
    let reviewed = schedule::reviewed(9, &standing).expect("the review's lesson");
    assert_eq!(reviewed.picked, Picked::Chosen);
    assert_eq!(reviewed.exercises.len(), 2);
    assert_eq!(reviewed.exercises[0].0.kind, waiting.kind);
    assert_eq!(reviewed.exercises[0].0.seed, derived(9, 0));
    // Laid again, it is answered, whatever it ends with; the other still waits.
    let answered = learner.with(&ended(reviewed, &[Redo]));
    assert_eq!(
        answered
            .redos()
            .filter(|(of, _)| *of != review.skill)
            .count(),
        1
    );
    let again: Vec<Kind> = (answered.redos())
        .filter(|(of, _)| *of == review.skill)
        .map(|(_, redo)| redo.kind)
        .collect();
    assert_eq!(
        again,
        [waiting.kind],
        "the one just ended, the old one answered"
    );
}

#[test]
fn the_first_lesson_is_a_scribble_a_straight_line_and_an_arc() {
    let first = schedule::first(3);
    assert_eq!(first.picked, Picked::First);
    let kinds: Vec<Kind> = first.exercises.iter().map(|(e, _)| e.kind).collect();
    let lines = [LineTier::Straight, LineTier::Arc].map(Kind::Line);
    assert_eq!(kinds, [Kind::Free, lines[0], lines[1]]);
    for (index, (exercise, level)) in first.exercises.iter().enumerate() {
        assert_eq!(exercise.seed, derived(3, index as u64));
        assert_eq!(*level, SkillLevel::First);
    }
    assert!(Learner::default().fresh());
    let sheet = Sheet::begun(1, (Purpose::Day, NOON), first);
    let lesson = &sheet.lessons[0];
    assert_eq!((lesson.skill(), lesson.retest()), (None, None));
    let learner = Learner::of([&sheet]);
    assert!(!learner.fresh());
    assert_eq!(
        (learner.lessons(), learner.lessons_of(skill("line"))),
        (1, 1)
    );
    // It leaves its lines unpractised: the next lesson is the course's first skill, ghosted lines
    // (S156), then lines, never practised either, before the skills practised since.
    assert_eq!(learner.practised(skill("line")), None);
    let pictures = Pictures::default();
    let next = |learner: &Learner| {
        let standing = Standing {
            pictures: &pictures,
            learner,
        };
        schedule::next(&standing).map(|offer| offer.skill)
    };
    assert_eq!(next(&learner), Some(skill("ghosted-lines")));
    let ghosted = after(
        &learner,
        "ghosted-lines",
        SkillLevel::First,
        &[Outcome::Near; 3],
    );
    assert_eq!(next(&ghosted), Some(skill("line")));
}

#[test]
fn a_unit_opens_by_the_first_dots_of_the_unit_before_it() {
    let pictures = Pictures::default();
    let mut learner = Learner::default();
    assert!(learner.open(1, &pictures) && !learner.open(2, &pictures));
    assert_eq!(learner.unit(&pictures), 1);
    for key in ["ghosted-lines", "line", "combined", "colour"] {
        // A lesson of fewer than six fills the window over two.
        for _ in 0..2 {
            let level = learner.level(skill(key));
            learner = after(&learner, key, level, &CLEAN);
        }
        assert!(learner.dots(skill(key)) >= 1, "{key}");
    }
    assert!(learner.open(2, &pictures) && !learner.open(3, &pictures));
    assert_eq!(learner.unit(&pictures), 2);
    // No unit past the built ones opens, and none there is not.
    assert!(!learner.open(7, &pictures) && !learner.open(9, &pictures));
}

#[test]
fn a_unit_opens_only_after_the_one_before_it_opened_and_never_one_without_a_skill() {
    let pictures = Pictures::default();
    let fresh = Learner::default();
    // Unit 5's colour skills lack their packs; that opens nothing past it on a fresh learner.
    assert!(skill("key-shift").lacks(&pictures).is_some());
    let open = |learner: &Learner| -> Vec<u8> {
        (1..=8)
            .filter(|unit| learner.open(*unit, &pictures))
            .collect()
    };
    assert_eq!(open(&fresh), [1]);
    // Every skill that can be practised without a pack with its first dot: units 1 to 8 open in
    // course order, unit 5 though its colour skills cannot be practised, unit 6 though its gamut
    // masks cannot (S153), unit 8 though its contrast thumbnails cannot (S154): it holds a skill.
    let mut learner = fresh;
    for practised in Skill::all().filter(|skill| skill.lacks(&pictures).is_none()) {
        // Lessons until the window fills: as many as it takes a lesson's size.
        for _ in 0..6 {
            let level = learner.level(practised);
            learner = after(&learner, practised.key(), level, &CLEAN);
        }
        assert!(learner.dots(practised) >= 1, "{}", practised.key());
    }
    assert_eq!(open(&learner), [1, 2, 3, 4, 5, 6, 7, 8]);
    // A unit's dots alone open nothing while the one before it is shut.
    let mut skipped = Learner::default();
    for key in [
        "shape",
        "combination",
        "scale",
        "box",
        "free-boxes",
        "ellipse",
    ] {
        if skill(key).lacks(&pictures).is_some() {
            continue;
        }
        for _ in 0..6 {
            let level = skipped.level(skill(key));
            skipped = after(&skipped, key, level, &CLEAN);
        }
    }
    assert_eq!(open(&skipped), [1]);
}
