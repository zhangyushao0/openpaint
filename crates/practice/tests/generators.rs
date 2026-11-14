//! The generators: the same seed gives the same exercise, exercises stay in their frames and
//! vary, and unit 1's full lesson gives each exercise its own seed, its colours mostly low in
//! saturation (the sheet: `sheet.rs`).

use openpaint_practice::{
    Combination, CombinedLines, Exercise, Family, ItemRef, Kind, Learner, LineExercise, LineTier,
    Original, Pictures, SkillLevel, Standing, Tier, Tone, hsv, length, schedule,
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
    // A kind code names one generator for good (D98): a stored session's lines come back to
    // the last bit; a changed generator is a new code.
    let line = LineExercise::generate(LineTier::Wave, 0x5EED_0045);
    let ends = [line.points[0], line.points[480]];
    assert_eq!(ends, GENERATED);
}

#[test]
fn combined_lines_are_the_same_per_seed_and_two_to_four_lines_in_the_frame() {
    for combination in Combination::ALL {
        let mut counts = [0; 5];
        for seed in 0..120 {
            let group = CombinedLines::generate(combination, seed);
            assert_eq!(group, CombinedLines::generate(combination, seed));
            let lines = group.lines.len();
            assert!((2..=4).contains(&lines), "{combination:?} {seed}");
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
    assert_eq!([first[0], first[first.len() - 1]], CROSSED);
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
        .filter(|e| e.kind.family() == Family::Colour)
        .collect();
    let low = (colours.iter())
        .filter(|e| match e.original() {
            Original::Colour(colour) => hsv(colour)[1] < 0.36,
            _ => false,
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
    assert_eq!(Kind::of_code(9, 3), None);
    assert_eq!(Kind::of_code(10, 2), None);
    assert_eq!(Kind::of_code(11, 3), None);
    assert_eq!(Kind::of_code(12, 5), None);
    assert_eq!(Kind::of_code(13, 3), None);
    assert_eq!(Kind::of_code(14, 4), None);
    assert_eq!(Kind::of_code(15, 0), None);
    assert_eq!(Kind::of_code(3, 4), None);
}

#[test]
fn an_exercise_is_its_kind_and_seed() {
    let exercise = Exercise::generated(Kind::Line(LineTier::Wave), 42);
    assert_eq!(
        exercise.original(),
        Original::Lines(vec![LineExercise::generate(LineTier::Wave, 42).points])
    );
    let group = Exercise::generated(Kind::Combined(Combination::Branched), 42);
    let lines = CombinedLines::generate(Combination::Branched, 42).lines;
    assert_eq!(group.original(), Original::Lines(lines));
    let item = ItemRef {
        pack: "test-shapes".to_owned(),
        item: "teapot".to_owned(),
    };
    let shape = Exercise {
        picture: Some(item.clone()),
        ..Exercise::generated(Kind::Shape(Tier::Simple), 42)
    };
    assert_eq!(shape.original(), Original::Picture(item));
    assert_eq!(shape.kind.code(), (4, 2));
    assert_eq!(Kind::of_code(4, 2), Some(shape.kind));
}
