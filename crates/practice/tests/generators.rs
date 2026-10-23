//! The generators: the same seed gives the same exercise, exercises stay in their frames and
//! vary, a session holds what stage 1 asks, and the sheet keeps its pairs apart.

use openpaint_practice::{
    Exercise, FRAME, Kind, LineExercise, LineTier, Original, Sheet, Tone, chroma, hsv, stage_one,
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
fn lines_keep_clear_of_the_frame_edge_and_have_their_key_points() {
    for tier in LineTier::ALL {
        for seed in 0..200 {
            let line = LineExercise::generate(tier, seed);
            for p in &line.points {
                assert!(
                    p.iter().all(|v| (0.119..=0.881).contains(v)),
                    "{tier:?} {seed}: {p:?}"
                );
            }
            let keys = line.key_points.len();
            let expected = match tier {
                LineTier::Straight => 2..=3,
                LineTier::Arc | LineTier::Hook => 3..=4,
                LineTier::Wave => 3..=5,
            };
            assert!(
                expected.contains(&keys),
                "{tier:?} {seed}: {keys} key points"
            );
            assert_eq!(line.key_points[0], line.points[0], "an end first");
        }
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
fn a_stage_one_session_is_ten_lines_then_eight_colours_mostly_low_in_saturation() {
    let session = stage_one(0x5EED_0045);
    assert_eq!(session, stage_one(0x5EED_0045));
    assert_ne!(session, stage_one(1));
    assert_eq!(session.len(), 18);
    assert!(session[..10].iter().all(|e| e.kind.is_line()));
    assert!(session[10..].iter().all(|e| !e.kind.is_line()));
    assert_eq!(session[0].kind, Kind::Line(LineTier::Straight));
    let low = session[10..]
        .iter()
        .filter(|e| match e.original() {
            Original::Colour(colour) => hsv(colour)[1] < 0.36 || chroma(colour) < 0.1,
            Original::Line(_) => false,
        })
        .count();
    assert!(low >= 5, "{low} of 8 low in saturation");
    let seeds: std::collections::HashSet<u64> = session.iter().map(|e| e.seed).collect();
    assert_eq!(seeds.len(), 18, "every exercise its own seed");
    for exercise in &session {
        let (kind, variant) = exercise.kind.code();
        assert_eq!(Kind::of_code(kind, variant), Some(exercise.kind));
    }
    assert_eq!(Kind::of_code(3, 0), None);
}

#[test]
fn an_exercise_is_its_kind_and_seed() {
    let exercise = Exercise {
        kind: Kind::Line(LineTier::Wave),
        seed: 42,
    };
    assert_eq!(
        exercise.original(),
        Original::Line(LineExercise::generate(LineTier::Wave, 42))
    );
}

#[test]
fn the_sheet_lays_pairs_three_a_row_apart_and_inside() {
    let sheet = Sheet::of(18);
    assert_eq!(sheet.size, [2920, 3160]);
    assert_eq!(sheet.pairs.len(), 18);
    let [width, height] = sheet.size.map(f64::from);
    let frames: Vec<_> = sheet
        .pairs
        .iter()
        .flat_map(|pair| [pair.original, pair.copy])
        .collect();
    for (index, a) in frames.iter().enumerate() {
        assert_eq!(a.side, FRAME);
        assert!(a.origin[0] >= 0.0 && a.origin[0] + a.side <= width);
        assert!(a.origin[1] >= 0.0 && a.origin[1] + a.side <= height);
        for b in &frames[index + 1..] {
            let apart = (0..2).any(|axis| {
                a.origin[axis] + a.side < b.origin[axis] || b.origin[axis] + b.side < a.origin[axis]
            });
            assert!(apart, "{a:?} and {b:?} overlap");
        }
    }
    let pair = sheet.pairs[4];
    assert!(pair.copy.origin[0] > pair.original.origin[0]);
    assert_eq!(pair.copy.origin[1], pair.original.origin[1]);
    let centre = pair.centre();
    assert!(pair.original.contains(centre, 30.0) && pair.copy.contains(centre, 30.0));
}
