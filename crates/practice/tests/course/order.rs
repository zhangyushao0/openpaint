//! The schedule (S122): the First-prefix rule against the drills before S122, the next lesson's
//! order, a lesson chosen with its redo, a unit's full lesson.

use openpaint_practice::{
    BoxView, Carve, Combination, Exercise, Guide, Kind, Learner, Lightness, LineTier, Outcome,
    Picked, Pictures, Purity, Purpose, ScaleTone, Sheet, Skill, SkillLevel, Standing, Tone,
    derived, schedule,
};

use crate::{CLEAN, NOON, after, ended, skill};

/// Each kind's first drill before S122, its kinds in order.
fn drills() -> Vec<(&'static str, Vec<Kind>)> {
    use Combination::{Branched, Crossing, Joined, Near};
    use LineTier::{Arc, Hook, Straight, Wave};
    let tone = |purity, lightness| Kind::Colour(Tone { purity, lightness });
    let (two, three) = (Kind::Box(BoxView::TwoPoint), Kind::Box(BoxView::ThreePoint));
    let (frame, plane) = (Kind::Ellipse(Guide::Frame), Kind::Ellipse(Guide::Plane));
    let lines = [Straight, Arc, Wave, Arc, Hook, Wave].map(Kind::Line);
    let groups = [Joined, Crossing, Branched, Near].map(Kind::Combined);
    let objects = [Carve::Block, Carve::Cut, Carve::Taper].map(Kind::Object);
    let colours = vec![
        tone(Purity::Muted, Lightness::Middle),
        tone(Purity::Grey, Lightness::Light),
        tone(Purity::Middle, Lightness::Dark),
        tone(Purity::Muted, Lightness::Light),
        tone(Purity::Vivid, Lightness::Middle),
        tone(Purity::Grey, Lightness::Dark),
    ];
    vec![
        ("line", lines.to_vec()),
        ("combined", groups.to_vec()),
        ("colour", colours),
        (
            "scale",
            ScaleTone::ALL.map(|tone| Kind::Scale(tone, 5)).to_vec(),
        ),
        ("box", vec![two, three, two]),
        ("ellipse", vec![frame, plane, frame, plane]),
        ("objects", objects.to_vec()),
    ]
}

/// The kinds of the first-level lesson of the skill keyed `key` for `learner`, seeded 11.
fn first_kinds(learner: &Learner, key: &str) -> Vec<Kind> {
    let pictures = Pictures::default();
    let standing = Standing {
        pictures: &pictures,
        learner,
    };
    let planned = schedule::lesson(skill(key), SkillLevel::First, 11, &standing);
    let planned = planned.expect("a lesson");
    planned.exercises.iter().map(|(e, _)| e.kind).collect()
}

#[test]
fn at_the_first_level_a_lesson_of_one_topic_starts_as_its_drill_did() {
    let (pictures, learner) = (Pictures::default(), Learner::default());
    let standing = Standing {
        pictures: &pictures,
        learner: &learner,
    };
    for (key, drilled) in drills() {
        let lesson = schedule::lesson(skill(key), SkillLevel::First, 11, &standing);
        let lesson = lesson.expect("a lesson").exercises;
        assert!(!lesson.is_empty() && lesson.len() <= drilled.len(), "{key}");
        for (index, ((exercise, _), kind)) in lesson.iter().zip(&drilled).enumerate() {
            let old = Exercise::generated(*kind, derived(11, index as u64));
            assert_eq!(*exercise, old, "{key}");
        }
    }
    // A second lesson turns as a second drill did: a box three-point first, an object round.
    let boxed = after(&learner, "box", SkillLevel::First, &[]);
    let objects = after(&learner, "objects", SkillLevel::First, &[]);
    let views = [BoxView::ThreePoint, BoxView::TwoPoint].map(Kind::Box);
    assert_eq!(first_kinds(&boxed, "box"), views);
    assert_eq!(
        first_kinds(&objects, "objects"),
        [Kind::Object(Carve::Round)]
    );
}

#[test]
fn the_next_lesson_is_the_fewest_dots_then_the_longest_unpractised_then_the_course_s_order() {
    use Outcome::Near;
    let pictures = Pictures::default();
    let next = |learner: &Learner| {
        let standing = Standing {
            pictures: &pictures,
            learner,
        };
        schedule::next(&standing).map(|offer| offer.skill.key())
    };
    let fresh = Learner::default();
    assert_eq!(next(&fresh), Some("line"), "the course's order");
    // Lines practised: the skills never practised come first, in the course's order.
    let lined = after(&fresh, "line", SkillLevel::First, &[Near; 4]);
    assert_eq!(next(&lined), Some("combined"));
    let grouped = after(&lined, "combined", SkillLevel::First, &[Near; 3]);
    let coloured = after(&grouped, "colour", SkillLevel::First, &[Near; 3]);
    assert_eq!(next(&coloured), Some("line"), "the longest unpractised");
    // A dot puts a skill behind the others.
    let dotted = after(&coloured, "line", SkillLevel::First, &CLEAN);
    assert_eq!(dotted.dots(skill("line")), 1);
    assert_eq!(next(&dotted), Some("combined"));
    let standing = Standing {
        pictures: &pictures,
        learner: &dotted,
    };
    let offer = schedule::offer(skill("line"), &standing).expect("an offer");
    let shown = (offer.level, offer.exercises, offer.minutes);
    assert_eq!(shown, (SkillLevel::Second, 6, 5));
    // Without a pack, unit 2's first skill it can practise is a value scale.
    let mut learner = dotted;
    for key in ["combined", "colour"] {
        for _ in 0..2 {
            learner = after(&learner, key, SkillLevel::First, &CLEAN);
        }
    }
    assert_eq!(learner.unit(&pictures), 2);
    assert_eq!(next(&learner), Some("scale"));
}

#[test]
fn a_lesson_chosen_lays_its_redo_first_and_keeps_its_size() {
    use Outcome::{Clean, Redo};
    let pictures = Pictures::default();
    let fresh = Learner::default();
    let learner = after(&fresh, "colour", SkillLevel::First, &[Clean, Redo, Clean]);
    let standing = Standing {
        pictures: &pictures,
        learner: &learner,
    };
    let colours = skill("colour");
    let waiting = learner.redo(colours).expect("a redo").clone();
    let chosen = schedule::chosen(colours, 4, &standing).expect("a lesson");
    let plain = schedule::lesson(colours, SkillLevel::First, 4, &standing).expect("a lesson");
    assert_eq!(chosen.exercises.len(), plain.exercises.len());
    assert_eq!(chosen.exercises[0].0.kind, waiting.kind);
    assert_eq!(chosen.exercises[0].0.seed, derived(4, 0));
    // Laid again, it is answered.
    let answered = learner.with(&ended(chosen, &[Clean]));
    assert!(answered.redo(colours).is_none());
}

#[test]
fn a_full_lesson_warms_up_on_three_skills_longest_unpractised_then_holds_its_unit() {
    let pictures = Pictures::default();
    let mut learner = Learner::default();
    for key in ["line", "combined", "colour"] {
        learner = after(&learner, key, SkillLevel::First, &CLEAN);
    }
    let standing = Standing {
        pictures: &pictures,
        learner: &learner,
    };
    // Unit 2 without a pack: its scales, boxes, free boxes and ellipses after a warm-up of unit 1.
    let full = schedule::full(2, 6, &standing).expect("unit 2's");
    let picked = Picked::Full {
        stage: 2,
        warm_up: 3,
    };
    assert_eq!(full.picked, picked);
    let skills: Vec<&str> = (full.exercises.iter())
        .map(|(e, _)| Skill::of(e.kind).expect("a skill").key())
        .collect();
    let warm = ["line", "combined", "colour"];
    assert_eq!(skills[..3], warm, "the longest unpractised first");
    let mut unit: Vec<&str> = skills[3..].to_vec();
    unit.dedup();
    assert_eq!(unit, ["scale", "box", "free-boxes", "ellipse"]);
    let seconds: u32 = (full.exercises.iter())
        .map(|(e, _)| u32::from(e.kind.facts().seconds))
        .sum();
    assert!((900..=2400).contains(&seconds), "{seconds} s");
    for (index, (exercise, _)) in full.exercises.iter().enumerate() {
        assert_eq!(exercise.seed, derived(6, index as u64));
    }
    let sheet = Sheet::begun(1, (Purpose::Day, NOON), full);
    let lesson = &sheet.lessons[0];
    assert_eq!((lesson.unit(), lesson.warm_up()), (Some(2), 3));
    // A unit not built has none.
    assert!(schedule::full(6, 6, &standing).is_none());
}
