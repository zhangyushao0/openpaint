//! The schedule (S122): the First-prefix rule against the earlier product's drills (a kind's run
//! of exercises before S122), what the first level and the earlier variants lay kept through the
//! harder variants appended (S142), the next lesson's order, a lesson chosen with its redo.

use openpaint_practice::{
    BoxView, Carve, Combination, EllipseGuide, Exercise, Guide, Horizon, Kind, Learner, Lightness,
    LineTier, Outcome, PerspectiveBox, Pictures, Purity, ScaleTone, SkillLevel, Standing, Tone,
    derived, schedule,
};

use crate::{CLEAN, after, ended, skill};

/// Each kind's first drill before S122 (the earlier product's word), its kinds in order.
fn before_s122() -> Vec<(&'static str, Vec<Kind>)> {
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
fn at_the_first_level_a_lesson_of_one_topic_starts_as_before_s122() {
    let (pictures, learner) = (Pictures::default(), Learner::default());
    let standing = Standing {
        pictures: &pictures,
        learner: &learner,
    };
    for (key, earlier) in before_s122() {
        let lesson = schedule::lesson(skill(key), SkillLevel::First, 11, &standing);
        let lesson = lesson.expect("a lesson").exercises;
        assert!(!lesson.is_empty() && lesson.len() <= earlier.len(), "{key}");
        for (index, ((exercise, _), kind)) in lesson.iter().zip(&earlier).enumerate() {
            let old = Exercise::generated(*kind, derived(11, index as u64));
            assert_eq!(*exercise, old, "{key}");
        }
    }
    // A second lesson turns as a second drill did before S122: a box three-point first, an
    // object round.
    let boxed = after(&learner, "box", SkillLevel::First, &[]);
    let objects = after(&learner, "objects", SkillLevel::First, &[]);
    let views = [BoxView::ThreePoint, BoxView::TwoPoint].map(Kind::Box);
    assert_eq!(first_kinds(&boxed, "box"), views);
    assert_eq!(
        first_kinds(&objects, "objects"),
        [Kind::Object(Carve::Round)]
    );
}

/// `value` folded into the FNV-1a hash `hash`, byte by byte.
fn fnv(hash: &mut u64, value: u64) {
    for byte in value.to_le_bytes() {
        *hash ^= u64::from(byte);
        *hash = hash.wrapping_mul(0x100_0000_01B3);
    }
}

/// `b` folded into `hash`: its corners, horizon, vanishing points (1 for one it lacks), front.
fn boxed(hash: &mut u64, b: &PerspectiveBox) {
    for c in b.corners {
        fnv(hash, c[0].to_bits());
        fnv(hash, c[1].to_bits());
    }
    fnv(hash, b.horizon.to_bits());
    for v in b.vanishing {
        match v {
            Some(p) => {
                fnv(hash, p[0].to_bits());
                fnv(hash, p[1].to_bits());
            }
            None => fnv(hash, 1),
        }
    }
    fnv(hash, b.front as u64);
}

/// S142 appended harder variants to the boxes', free pages' and ellipses' lists: every variant
/// there before lays what it laid at f100347, seed for seed, and so does every first-level lesson
/// over eight lessons (the hashes taken there, of these same walks).
#[test]
fn the_earlier_variants_and_the_first_level_lay_what_they_laid_before_s142() {
    const START: u64 = 0xCBF2_9CE4_8422_2325;
    let mut boxes = START;
    for view in [BoxView::TwoPoint, BoxView::ThreePoint] {
        (0..300).for_each(|seed| boxed(&mut boxes, &PerspectiveBox::generate(view, seed)));
    }
    let mut pages = START;
    for horizon in [Horizon::Low, Horizon::Middle, Horizon::High] {
        (0..300).for_each(|seed| boxed(&mut pages, &PerspectiveBox::on_horizon(horizon, seed)));
    }
    let mut ellipses = START;
    for guide in [Guide::Frame, Guide::Plane] {
        for seed in 0..300 {
            let g = EllipseGuide::generate(guide, seed);
            let e = g.exact;
            let points = g.corners.iter().chain(&g.touches).flatten();
            let exact = [e.centre[0], e.centre[1], e.axes[0], e.axes[1], e.angle];
            points
                .chain(&exact)
                .for_each(|v| fnv(&mut ellipses, v.to_bits()));
        }
    }
    let hashes = [boxes, pages, ellipses];
    assert_eq!(
        hashes,
        [
            0xe0a1_c663_dd23_bfd9,
            0xa8eb_3c6c_664d_7558,
            0x64b3_bb7a_2c45_7296
        ],
        "{hashes:x?}"
    );
    let pictures = Pictures::default();
    let lessons = ["box", "ellipse", "free-boxes"].map(|key| {
        let (mut learner, mut hash) = (Learner::default(), START);
        for turn in 0..8 {
            let standing = Standing {
                pictures: &pictures,
                learner: &learner,
            };
            let planned = schedule::lesson(skill(key), SkillLevel::First, 11 + turn, &standing);
            let planned = planned.expect("a lesson");
            for (exercise, _) in &planned.exercises {
                let (code, variant) = exercise.kind.code();
                for value in [u64::from(code), u64::from(variant), exercise.seed] {
                    fnv(&mut hash, value);
                }
            }
            learner = learner.with(&ended(planned, &[Outcome::Near; 6]));
        }
        hash
    });
    assert_eq!(
        lessons,
        [
            0x5383_e461_ccc5_e330,
            0x9ea6_2b3e_b3f1_faeb,
            0xbd6a_5c7c_dfa7_847f
        ],
        "{lessons:x?}"
    );
}

/// S142's levels: boxes at 熟练 a three-point box and a long one, at 稳 one drawn through beside a
/// long or a three-point one; free boxes at 熟练 a page looking down or up with its guides, at 稳
/// the same bare by turns with a path page (S156l), one page a lesson; ellipses at 稳 one in a
/// turned frame and a funnel (S156l). Lesson by lesson.
#[test]
fn boxes_pages_and_ellipses_lay_their_harder_variants_at_the_upper_levels() {
    use BoxView::{Long, ThreePoint, Through};
    let pictures = Pictures::default();
    let kinds = |key: &str, level, lessons: usize| -> Vec<Vec<Kind>> {
        let mut learner = Learner::default();
        (0..lessons)
            .map(|_| {
                let standing = Standing {
                    pictures: &pictures,
                    learner: &learner,
                };
                let planned = schedule::lesson(skill(key), level, 11, &standing);
                let kinds = (planned.iter().flat_map(|p| &p.exercises))
                    .map(|(e, _)| e.kind)
                    .collect();
                learner = after(&learner, key, level, &[]);
                kinds
            })
            .collect()
    };
    let boxes = |views: &[[BoxView; 2]]| -> Vec<Vec<Kind>> {
        views.iter().map(|v| v.map(Kind::Box).to_vec()).collect()
    };
    let second = boxes(&[[ThreePoint, Long], [Long, ThreePoint], [ThreePoint, Long]]);
    assert_eq!(kinds("box", SkillLevel::Second, 3), second);
    let third = boxes(&[[Through, Long], [ThreePoint, Through], [Through, Long]]);
    assert_eq!(kinds("box", SkillLevel::Third, 3), third);
    let pages = |horizons: [Horizon; 2]| -> Vec<Vec<Kind>> {
        (0..3)
            .map(|k| vec![Kind::FreeBoxes(horizons[k % 2])])
            .collect()
    };
    let guided = pages([Horizon::HighDown, Horizon::LowUp]);
    assert_eq!(kinds("free-boxes", SkillLevel::Second, 3), guided);
    // At 稳 the bare pages by turns with a path page (S156l), the third lesson's page as before.
    let bare: Vec<Vec<Kind>> = [Horizon::LowUpBare, Horizon::Path, Horizon::HighDownBare]
        .map(|horizon| vec![Kind::FreeBoxes(horizon)])
        .to_vec();
    assert_eq!(kinds("free-boxes", SkillLevel::Third, 3), bare);
    let (plane, turned) = (Kind::Ellipse(Guide::Plane), Kind::Ellipse(Guide::Turned));
    let funnel = Kind::Ellipse(Guide::Funnel);
    let third = vec![plane, turned, funnel, plane];
    assert_eq!(
        kinds("ellipse", SkillLevel::Third, 2),
        [third.clone(), third]
    );
    let second = kinds("ellipse", SkillLevel::Second, 1).concat();
    assert!(!second.contains(&turned), "熟练 unchanged: {second:?}");
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
