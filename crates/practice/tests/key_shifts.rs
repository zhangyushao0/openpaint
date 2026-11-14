//! Key shifts (S107): tendencies kept in their kind's code, warm or cool and never near the crop's
//! own hue; the rule a painter shifts by (lightness kept, greys moving most, opposites a little,
//! the lightest barely, near hues turning the short way, neighbours kept apart); the three checks
//! on shifts made by rule and made wrong on purpose; a lesson at each level; the How.
//! The checks are judged by eye on real crops too: `openpaint-harness key-shifts`.

use std::sync::Arc;

use openpaint_practice::{
    APART, AWAY, Areas, Arrangement, CELLS, GeneratedAreas, KeyCheck, Kind, LIGHTNESS_OFF, Lean,
    Learner, MERGED, Metric, Needs, Pack, PackItem, PictureKind, Pictures, Random, Rgb8, Skill,
    SkillLevel, Standing, Tendency, Tier, delta_e, from_oklab, main_hue, oklab, oklch, schedule,
    toward, turn,
};

fn lab(colour: Rgb8) -> [f64; 3] {
    oklab(colour)
}

/// Generated combinations of known areas, standing in for crops.
fn examples() -> Vec<Areas> {
    (0..12u64)
        .map(|seed| {
            let arrangement = Arrangement::ALL[seed as usize % 3];
            GeneratedAreas::generate(arrangement, 0x5EED_0107 + seed).areas
        })
        .collect()
}

#[test]
fn a_tendency_reads_back_from_its_code_and_only_its_lean_s_hues_do() {
    for lean in Lean::ALL {
        let mut random = Random::new(7);
        for _ in 0..40 {
            let tendency = Tendency::chosen(lean, None, &mut random);
            assert_eq!(Tendency::of_code(tendency.code()), Some(tendency));
            assert_eq!(
                Kind::of_code(19, tendency.code()),
                Some(Kind::KeyShift(tendency))
            );
            let warm = tendency.hue <= 110 || tendency.hue >= 350;
            assert_eq!(warm, lean.warm(), "{tendency:?}");
        }
    }
    // A warm lean's code with a cool hue names no tendency.
    let cool_hue = Lean::ALL.iter().position(|l| *l == Lean::WarmPure).unwrap() as u8 * 36 + 20;
    assert_eq!(Tendency::of_code(cool_hue), None);
    assert_eq!(
        Kind::KeyShift(Tendency::of_code(0).unwrap()).facts().key,
        "key-shift"
    );
}

#[test]
fn a_tendency_keeps_away_from_the_crop_s_main_hue() {
    for main in (0..360).step_by(5) {
        for lean in Lean::ALL {
            let mut random = Random::new(u64::from(main as u16));
            for _ in 0..20 {
                let tendency = Tendency::chosen(lean, Some(f64::from(main)), &mut random);
                let off = turn(f64::from(tendency.hue), f64::from(main)).abs();
                assert!(off >= AWAY, "{lean:?} {main}: {tendency:?}");
            }
        }
    }
    // A crop of rust oranges has an orange main hue; a grey one none.
    let rust = Areas::of_map([2, 1], vec![0, 1], vec![[180, 90, 40], [140, 70, 40]]).unwrap();
    let hue = main_hue(&rust).expect("a main hue");
    assert!((30.0..80.0).contains(&hue), "{hue}");
    let grey = Areas::of_map([2, 1], vec![0, 1], vec![[120; 3], [200; 3]]).unwrap();
    assert_eq!(main_hue(&grey), None);
}

#[test]
fn the_rule_keeps_lightness_and_moves_greys_most_opposites_a_little_the_lightest_barely() {
    let orange = Tendency::of_code(6).unwrap();
    assert_eq!((orange.lean, orange.hue), (Lean::WarmPure, 60));
    let from = |l: f64, c: f64, h: f64| openpaint_practice::in_gamut([l, c, h]);
    let grey = from(0.6, 0.01, 240.0);
    let blue = from(0.5, 0.1, 250.0);
    let red = from(0.55, 0.08, 20.0);
    let cream = from(0.94, 0.02, 95.0);
    let moved = |c: Rgb8| delta_e(c, orange.shift(c));
    for colour in [grey, blue, red, cream] {
        let off = (lab(orange.shift(colour))[0] - lab(colour)[0]).abs();
        assert!(off < 0.01, "lightness kept: {off}");
        assert!(toward(colour, orange.shift(colour), orange) > 0.0);
    }
    assert!(moved(grey) > moved(blue), "the grey moves most");
    assert!(
        oklch(orange.shift(blue))[1] < oklch(blue)[1],
        "the opposite greys"
    );
    assert!(
        moved(cream) < 0.5 * moved(grey),
        "the lightest barely moves"
    );
    let [_, _, before] = oklch(red);
    let [_, _, after] = oklch(orange.shift(red));
    assert!(
        after > before && after < 60.0,
        "red turns toward orange: {after}"
    );
}

#[test]
fn shifts_made_by_rule_are_marked_nowhere_and_neighbours_stay_apart() {
    for areas in examples() {
        for (k, lean) in Lean::ALL.into_iter().enumerate() {
            let tendency = Tendency::chosen(lean, main_hue(&areas), &mut Random::new(k as u64));
            let shifted: Vec<Option<Rgb8>> =
                tendency.shifted(&areas).into_iter().map(Some).collect();
            let check = KeyCheck::of(&areas, &shifted, tendency);
            assert_eq!(check.off_lightness, Vec::<usize>::new(), "{tendency:?}");
            assert_eq!(check.unmoved, Vec::<usize>::new(), "{tendency:?}");
            assert!(check.merged.is_empty(), "{tendency:?}");
            for &(a, b) in &areas.neighbours {
                if delta_e(areas.colours[a], areas.colours[b]) >= APART {
                    let apart = delta_e(shifted[a].unwrap(), shifted[b].unwrap());
                    assert!(apart >= MERGED, "{a} {b}: {apart}");
                }
            }
            let counts: Vec<f64> = check.metrics().iter().map(|(_, v)| *v).collect();
            assert_eq!(counts, [0.0; 3]);
        }
    }
}

#[test]
fn a_broken_lightness_an_area_left_behind_and_merged_neighbours_are_each_marked() {
    let areas = &examples()[0];
    let tendency = Tendency::chosen(Lean::CoolPure, main_hue(areas), &mut Random::new(3));
    let shifted = tendency.shifted(areas);
    let middle = (0..areas.count())
        .find(|a| (0.3..0.75).contains(&lab(shifted[*a])[0]))
        .expect("an area of middle lightness");
    // Lightness broken past the tolerance, then kept within it.
    for (off, marked) in [(LIGHTNESS_OFF + 0.03, true), (LIGHTNESS_OFF - 0.03, false)] {
        let mut copy: Vec<Option<Rgb8>> = shifted.iter().copied().map(Some).collect();
        let [l, a, b] = lab(shifted[middle]);
        copy[middle] = Some(from_oklab([l - off, a, b]));
        let check = KeyCheck::of(areas, &copy, tendency);
        assert_eq!(check.off_lightness.contains(&middle), marked, "{off}");
        assert_eq!(check.lighter_darker().1, usize::from(marked));
    }
    // An area painted as it was: not toward the tendency.
    let mut copy: Vec<Option<Rgb8>> = shifted.iter().copied().map(Some).collect();
    copy[middle] = Some(areas.colours[middle]);
    let check = KeyCheck::of(areas, &copy, tendency);
    assert!(check.unmoved.contains(&middle) && check.marked(middle));
    assert_eq!(
        check.metrics()[1],
        (Metric::KeyUnmoved, check.unmoved.len() as f64)
    );
    // Two neighbours apart in the original painted one colour: merged, one block of two.
    let &(a, b) = (areas.neighbours.iter())
        .find(|(a, b)| delta_e(areas.colours[*a], areas.colours[*b]) >= APART)
        .expect("a pair apart");
    let mut copy: Vec<Option<Rgb8>> = shifted.iter().copied().map(Some).collect();
    copy[b] = copy[a];
    let check = KeyCheck::of(areas, &copy, tendency);
    assert_eq!(check.merged, [(a, b)]);
    assert_eq!(check.merged_blocks(), (2, 1));
    // An area not painted is judged by nothing.
    let mut copy: Vec<Option<Rgb8>> = shifted.iter().copied().map(Some).collect();
    copy[middle] = None;
    let check = KeyCheck::of(areas, &copy, tendency);
    assert!(!check.marked(middle) && check.lightness[middle].is_none());
}

/// A pack of `count` crops of a few colours.
fn crops(count: usize) -> Pack {
    let areas = Areas::of_map(
        [2, 2],
        vec![0, 1, 2, 0],
        vec![[160, 80, 40], [90, 96, 110], [220; 3]],
    );
    Pack {
        id: "test-crops".to_owned(),
        title: "Test crops".to_owned(),
        items: (0..count)
            .map(|n| PackItem {
                id: format!("crop-{n}"),
                kind: PictureKind::Crop,
                tier: Tier::Simple,
                size: [600, 600],
                title: "test".to_owned(),
                credit: None,
                outline: None,
                masses: None,
                areas: areas.clone().map(Arc::new),
            })
            .collect(),
    }
}

#[test]
fn a_key_shift_s_lesson_is_one_crop_its_tendency_by_level_never_near_its_own_hue() {
    let pictures = Pictures::of(&[crops(8)]);
    let learner = Learner::default();
    let standing = Standing {
        pictures: &pictures,
        learner: &learner,
    };
    let skill = Skill::of_key("key-shift").expect("the key shifts");
    assert_eq!(skill.needs(), Needs::Crops);
    // The first level warm, the second cool, the third grey.
    let leans = [Lean::WarmPure, Lean::CoolPure, Lean::CoolGrey];
    let main = main_hue(&crops(1).items[0].areas.clone().unwrap()).unwrap();
    for (level, lean) in SkillLevel::ALL.into_iter().zip(leans) {
        let planned = schedule::lesson(skill, level, 0x5EED_0107, &standing).expect("a lesson");
        let [(exercise, _)] = planned.exercises.as_slice() else {
            panic!("{level:?}: one key shift");
        };
        let Kind::KeyShift(tendency) = exercise.kind else {
            panic!("{level:?}: a key shift");
        };
        assert_eq!(tendency.lean, lean, "{level:?}");
        assert!(exercise.picture.is_some(), "its crop");
        // The crops' main hue is orange-brown: no tendency lies within `AWAY` of it.
        assert!(turn(f64::from(tendency.hue), main).abs() >= AWAY);
    }
    // Without crops: no key shift.
    let none = Pictures::of(&[]);
    let bare = Standing {
        pictures: &none,
        learner: &learner,
    };
    assert_eq!(skill.lacks(&none), Some(Needs::Crops));
    assert!(schedule::lesson(skill, SkillLevel::First, 7, &bare).is_none());
    assert_eq!(skill.window(), 3, "a lesson of one");
}

#[test]
fn the_how_shifts_its_example_step_by_step_and_compares_it_in_grey() {
    let demo = Kind::KeyShift(Tendency::of_code(0).unwrap())
        .how(CELLS)
        .expect("a demonstration");
    assert_eq!(demo.steps.len(), 7, "V大's six steps and Compare");
    let shift = demo.strip.as_ref().expect("its tendency and chips");
    assert_eq!(shift.chips.len(), 5, "a chip an area");
    assert!(shift.looked < shift.chips[0].0);
    let areas = &demo.areas.as_ref().expect("its example").areas;
    assert_eq!(areas.count(), 5);
    // Each chip's shift keeps its lightness.
    for (_, [original, shifted]) in &shift.chips {
        assert!((lab(*original)[0] - lab(*shifted)[0]).abs() < 0.01);
    }
    assert!(demo.length > demo.steps[6]);
}
