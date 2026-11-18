//! The generators: the same seed gives the same exercise (the variants before S141 as they were),
//! exercises stay in their frames and vary, lines' and line groups' lessons ask more at each
//! level (S141), and a colour lesson gives each exercise its own seed, its colours mostly
//! low in saturation (the sheet: `sheet.rs`).

use openpaint_practice::{
    BoxView, Combination, CombinedLines, Exercise, Guide, Horizon, ItemRef, Kind, Learner,
    LineExercise, LineTier, Pictures, Purpose, Sheet, SkillLevel, Standing, Tier, Tone, hsv,
    length, schedule,
};

#[test]
fn the_same_seed_gives_the_same_line_and_another_seed_another() {
    for tier in LineTier::ALL {
        for seed in [0, 1, 0x5EED_0045, u64::MAX] {
            assert_eq!(
                LineExercise::generate(tier, seed),
                LineExercise::generate(tier, seed),
                "{tier:?} {seed}"
            );
        }
        let a = LineExercise::generate(tier, 11);
        let b = LineExercise::generate(tier, 12);
        assert_ne!(a.points, b.points, "{tier:?}");
    }
}

#[test]
fn lines_keep_clear_of_the_frame_edge() {
    for tier in LineTier::ALL {
        for seed in 0..200 {
            let line = LineExercise::generate(tier, seed);
            assert_eq!(line.points.len(), 481, "{tier:?} {seed}");
            for p in &line.points {
                assert!(
                    p.iter().all(|v| (0.119..=0.881).contains(v)),
                    "{tier:?} {seed}: {p:?}"
                );
            }
        }
    }
}

/// The ends of the wave seeded `0x5EED_0045`, as its generator lays them.
const GENERATED: [[f64; 2]; 2] = [
    [0.373_935_229_852_733_36, 0.154_197_245_368_887_51],
    [0.686_650_806_083_420_9, 0.783_084_490_731_614_1],
];

#[test]
fn a_generator_keeps_giving_the_same_line() {
    // A kind code names one generator for good (D98): a seed's lines come back as they were laid
    // (`alike`); a changed generator is a new code.
    let line = LineExercise::generate(LineTier::Wave, 0x5EED_0045);
    let ends = [line.points[0], line.points[480]];
    assert!(alike(&ends, &GENERATED), "{ends:?}");
}

/// `a` and `b` alike but for the last bits each platform's maths rounds its own way (`sin`,
/// `exp`: Windows' and Linux's differ in the last place, S162).
fn alike(a: &[[f64; 2]], b: &[[f64; 2]]) -> bool {
    let close = |(x, y): (&f64, &f64)| (x - y).abs() < 1e-12;
    a.len() == b.len() && a.iter().zip(b).all(|(p, q)| p.iter().zip(q).all(close))
}

#[test]
fn combined_lines_are_the_same_per_seed_and_two_to_six_lines_in_the_frame() {
    for combination in Combination::ALL {
        let mut counts = [0; 7];
        let held = match combination {
            Combination::Enclosed => 4..=6,
            Combination::Cluster => 5..=6,
            _ => 2..=4,
        };
        for seed in 0..120 {
            let group = CombinedLines::generate(combination, seed);
            assert_eq!(group, CombinedLines::generate(combination, seed));
            let lines = group.lines.len();
            assert!(held.contains(&lines), "{combination:?} {seed}");
            counts[lines] += 1;
            for line in &group.lines {
                assert!(length(line) >= 0.16, "{combination:?} {seed}: a short line");
                for p in line {
                    let inside = p.iter().all(|v| (0.119..=0.881).contains(v));
                    assert!(inside, "{combination:?} {seed}: {p:?}");
                }
            }
        }
        // Every arrangement varies its count of lines.
        assert!(
            counts.iter().filter(|n| **n > 0).count() >= 2,
            "{combination:?} {counts:?}"
        );
        let (a, b) = (
            CombinedLines::generate(combination, 11),
            CombinedLines::generate(combination, 12),
        );
        assert_ne!(a, b, "{combination:?}");
    }
}

/// The first line's ends of the crossing seeded `0x5EED_0045`, as its generator lays them.
const CROSSED: [[f64; 2]; 2] = [
    [0.760_361_213_999_229_2, 0.817_755_011_264_494_7],
    [0.287_639_662_086_821_24, 0.198_030_470_927_827_77],
];

#[test]
fn a_generator_keeps_giving_the_same_group() {
    // As for single lines: code 3's groups come back to the last bit.
    let group = CombinedLines::generate(Combination::Crossing, 0x5EED_0045);
    let first = &group.lines[0];
    let ends = [first[0], first[first.len() - 1]];
    assert!(alike(&ends, &CROSSED), "{ends:?}");
}

/// Each line before S141 seeded `0x5EED_0141`: its first and middle points, as ee393c6 laid them.
const LINES_BEFORE: [(LineTier, [[f64; 2]; 2]); 4] = [
    (
        LineTier::Straight,
        [
            [0.8471355384666874, 0.7182744320892254],
            [0.5002544524841326, 0.5074122383359594],
        ],
    ),
    (
        LineTier::Arc,
        [
            [0.26802940290307087, 0.2589082889396934],
            [0.512770032961778, 0.5103854779914025],
        ],
    ),
    (
        LineTier::Wave,
        [
            [0.8517000470129809, 0.4872615174436666],
            [0.5054771335676116, 0.55454883380176],
        ],
    ),
    (
        LineTier::Hook,
        [
            [0.5459004178611838, 0.33547330200746905],
            [0.5073556187142002, 0.681560768120901],
        ],
    ),
];

/// Each group before S141 seeded `0x5EED_0141`: its count of lines, its first point and its last,
/// as ee393c6 laid them.
const GROUPS_BEFORE: [(Combination, usize, [[f64; 2]; 2]); 4] = [
    (
        Combination::Joined,
        2,
        [
            [0.24676999923107776, 0.8720231433390516],
            [0.32880661651185583, 0.25028714407372765],
        ],
    ),
    (
        Combination::Branched,
        3,
        [
            [0.25885863840161333, 0.17384642178108697],
            [0.43373194369469587, 0.7743776032749385],
        ],
    ),
    (
        Combination::Crossing,
        2,
        [
            [0.12757153879785035, 0.5481643377700782],
            [0.6867029293695798, 0.7866379475658274],
        ],
    ),
    (
        Combination::Near,
        2,
        [
            [0.1985561804242636, 0.5453266047765613],
            [0.798182423186997, 0.5586602347180413],
        ],
    ),
];

#[test]
fn the_variants_before_s141_lay_what_they_laid() {
    // New variants are appended (S141); an existing one's seed still gives its exercise.
    for (tier, pinned) in LINES_BEFORE {
        let line = LineExercise::generate(tier, 0x5EED_0141).points;
        let laid = [line[0], line[240]];
        assert!(alike(&laid, &pinned), "{tier:?} {laid:?}");
    }
    for (combination, count, pinned) in GROUPS_BEFORE {
        let group = CombinedLines::generate(combination, 0x5EED_0141).lines;
        let last = group.last().map_or([0.0; 2], |line| line[line.len() - 1]);
        let laid = [group[0][0], last];
        assert_eq!(group.len(), count, "{combination:?}");
        assert!(alike(&laid, &pinned), "{combination:?} {laid:?}");
    }
}

#[test]
fn colours_follow_their_tone_and_the_seed() {
    for purity in Tone::PURITIES {
        for lightness in Tone::LIGHTNESSES {
            let tone = Tone { purity, lightness };
            assert_eq!(Tone::of_code(tone.code()), Some(tone));
            for seed in 0..100 {
                let colour = tone.colour(seed);
                assert_eq!(colour, tone.colour(seed));
                let [_, saturation, value] = hsv(colour);
                // Within the bands, give or take the 8-bit rounding.
                let [s_low, s_high] = match purity {
                    openpaint_practice::Purity::Grey => [0.0, 0.17],
                    openpaint_practice::Purity::Muted => [0.12, 0.36],
                    openpaint_practice::Purity::Middle => [0.32, 0.64],
                    openpaint_practice::Purity::Vivid => [0.6, 0.97],
                };
                assert!((s_low..=s_high).contains(&saturation), "{tone:?} {seed}");
                assert!((0.17..=0.96).contains(&value), "{tone:?} {seed}");
            }
        }
    }
}

#[test]
fn a_colour_lesson_holds_each_exercise_its_own_seed_its_colours_mostly_low() {
    let (pictures, learner) = (Pictures::default(), Learner::default());
    let standing = Standing {
        pictures: &pictures,
        learner: &learner,
    };
    let colour = openpaint_practice::Skill::of_key("colour").expect("colours");
    let lesson = |seed| schedule::lesson(colour, SkillLevel::First, seed, &standing);
    let planned = lesson(0x5EED_0045).expect("a colour lesson");
    assert_eq!(Some(&planned), lesson(0x5EED_0045).as_ref());
    assert_ne!(Some(&planned), lesson(1).as_ref());
    let lesson: Vec<&Exercise> = planned.exercises.iter().map(|(e, _)| e).collect();
    let colours: Vec<&&Exercise> = (lesson.iter())
        .filter(|e| e.kind.facts().key == "colour")
        .collect();
    let low = (colours.iter())
        .filter(|e| match e.original().swatch() {
            Some(colour) => hsv(colour)[1] < 0.36,
            None => false,
        })
        .count();
    assert!(
        3 * low >= 2 * colours.len(),
        "{low} of {} low",
        colours.len()
    );
    let seeds: std::collections::HashSet<u64> = lesson.iter().map(|e| e.seed).collect();
    assert_eq!(seeds.len(), lesson.len(), "every exercise its own seed");
    for exercise in lesson {
        let (kind, variant) = exercise.kind.code();
        assert_eq!(Kind::of_code(kind, variant), Some(exercise.kind));
    }
}

#[test]
fn a_code_past_a_kind_s_variants_reads_as_none() {
    assert_eq!(Kind::of_code(4, 0), None);
    assert_eq!(Kind::of_code(5, 3), None);
    assert_eq!(Kind::of_code(6, 2), None);
    assert_eq!(Kind::of_code(7, 1), None);
    assert_eq!(Kind::of_code(9, 8), None);
    assert_eq!(Kind::of_code(10, 4), None);
    assert_eq!(Kind::of_code(11, 3), None);
    assert_eq!(Kind::of_code(12, 5), None);
    assert_eq!(Kind::of_code(13, 3), None);
    assert_eq!(Kind::of_code(14, 4), None);
    assert_eq!(Kind::of_code(15, 0), None);
    assert_eq!(Kind::of_code(3, 6), None);
    assert_eq!(Kind::of_code(1, 6), None);
    // S141's variants appended: their codes come after those stored before.
    assert_eq!(Kind::Line(LineTier::Layered).code(), (1, 5));
    assert_eq!(Kind::Combined(Combination::Cluster).code(), (3, 5));
    // S142's: a box drawn through, a bare page looking up, a turned frame.
    assert_eq!(Kind::Box(BoxView::Through).code(), (8, 3));
    assert_eq!(Kind::FreeBoxes(Horizon::LowUpBare).code(), (9, 6));
    assert_eq!(Kind::Ellipse(Guide::Turned).code(), (10, 2));
    // S156l's: a path page, a funnel.
    assert_eq!(Kind::FreeBoxes(Horizon::Path).code(), (9, 7));
    assert_eq!(Kind::Ellipse(Guide::Funnel).code(), (10, 3));
}

/// The kinds of the `turn`th lesson of the skill keyed `key` at `level` (each lesson before it
/// taken by the learner).
fn lesson_kinds(key: &str, level: SkillLevel, turn: usize) -> Vec<Kind> {
    let skill = openpaint_practice::Skill::of_key(key).expect("a skill");
    let pictures = Pictures::default();
    let mut learner = Learner::default();
    let mut kinds = Vec::new();
    for _ in 0..=turn {
        let standing = Standing {
            pictures: &pictures,
            learner: &learner,
        };
        let planned = schedule::lesson(skill, level, 0x5EED_0141, &standing).expect("a lesson");
        kinds = planned.exercises.iter().map(|(e, _)| e.kind).collect();
        let mut sheet = Sheet::begun(1, (Purpose::Day, 1_791_201_600), planned);
        learner = learner.with(&sheet.lessons.remove(0));
    }
    kinds
}

#[test]
fn lines_and_line_groups_ask_more_key_points_and_relations_at_the_upper_levels() {
    use Combination::{Branched, Cluster, Crossing, Enclosed, Joined, Near};
    use LineTier::{Arc, Hook, Layered, Meander, Straight, Wave};
    use SkillLevel::{First, Second, Third};
    let lines = |level| lesson_kinds("line", level, 0);
    let [first, second, third] = [First, Second, Third].map(lines);
    assert_eq!(
        first,
        [Straight, Arc, Wave, Arc, Hook, Wave].map(Kind::Line)
    );
    assert_eq!(
        second,
        [Arc, Wave, Meander, Hook, Wave, Meander].map(Kind::Line)
    );
    let third_tiers = [Meander, Layered, Hook, Layered, Meander, Layered];
    assert_eq!(third, third_tiers.map(Kind::Line));
    // Groups: the second level three, in turn by the lesson; the third two larger ones, their
    // order in turn, so the lesson keeps its minutes.
    let groups = |level, turn| lesson_kinds("combined", level, turn);
    assert_eq!(
        groups(First, 1),
        [Joined, Crossing, Branched].map(Kind::Combined)
    );
    assert_eq!(
        groups(Second, 0),
        [Near, Enclosed, Branched].map(Kind::Combined)
    );
    assert_eq!(
        groups(Second, 1),
        [Enclosed, Branched, Near].map(Kind::Combined)
    );
    assert_eq!(groups(Third, 0), [Cluster, Enclosed].map(Kind::Combined));
    assert_eq!(groups(Third, 1), [Enclosed, Cluster].map(Kind::Combined));
}

#[test]
fn an_exercise_is_its_kind_and_seed() {
    let exercise = Exercise::generated(Kind::Line(LineTier::Wave), 42);
    assert_eq!(
        exercise.original().lines(),
        Some(&[LineExercise::generate(LineTier::Wave, 42).points][..])
    );
    let group = Exercise::generated(Kind::Combined(Combination::Branched), 42);
    let lines = CombinedLines::generate(Combination::Branched, 42).lines;
    assert_eq!(group.original().lines(), Some(&lines[..]));
    let item = ItemRef {
        pack: "test-shapes".to_owned(),
        item: "teapot".to_owned(),
    };
    let shape = Exercise {
        picture: Some(item.clone()),
        ..Exercise::generated(Kind::Shape(Tier::Simple), 42)
    };
    assert_eq!(shape.original().picture(), Some(&item));
    assert_eq!(shape.kind.code(), (4, 2));
    assert_eq!(Kind::of_code(4, 2), Some(shape.kind));
}
