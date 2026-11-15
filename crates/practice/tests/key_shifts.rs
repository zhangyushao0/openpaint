//! Key shifts (S107): tendencies kept in their kind's code, warm or cool and never near the crop's
//! own hue (far from it at the third level, S140); the rule a painter shifts by (lightness kept,
//! greys moving most, opposites a little, the lightest barely, near hues turning the short way,
//! neighbours kept apart); the three checks on shifts made by rule and made wrong on purpose; a
//! lesson at each level (its crop's tier: `crops.rs`); the How.
//! The checks are judged by eye on real crops too: `openpaint-harness key-shifts`.

use std::sync::Arc;

use openpaint_practice::{
    APART, AWAY, Areas, Arrangement, CELLS, FAR_AWAY, GeneratedAreas, ItemRef, KeyCheck, Kind,
    LIGHTNESS_OFF, Lean, Learner, MERGED, Metric, Needs, Pack, PackItem, PictureKind, Pictures,
    Purpose, Random, Rgb8, Sheet, Skill, SkillLevel, Standing, Tendency, Tier, delta_e, from_oklab,
    main_hue, oklab, oklch, schedule, toward, turn,
};

fn lab(colour: Rgb8) -> [f64; 3] {
    oklab(colour)
}

/// `lean`'s hues, as its codes name them.
fn hues(lean: Lean) -> Vec<u16> {
    let at = Lean::ALL.iter().position(|l| *l == lean).unwrap() as u8;
    (0..36)
        .filter_map(|step| Tendency::of_code(at * 36 + step))
        .map(|tendency| tendency.hue)
        .collect()
}

/// How far `lean`'s farthest hue lies from `main`, degrees.
fn farthest(lean: Lean, main: f64) -> f64 {
    (hues(lean).into_iter())
        .map(|hue| turn(f64::from(hue), main).abs())
        .fold(0.0, f64::max)
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
            let tendency = Tendency::chosen(lean, None, AWAY, &mut random);
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
fn a_tendency_keeps_its_least_turn_from_the_crop_s_main_hue_or_takes_the_lean_s_farthest() {
    let mut fell_back = 0;
    for main in (0..360).step_by(5) {
        let main = f64::from(main);
        for lean in Lean::ALL {
            // A lean's hues span 120 degrees and more: `AWAY` always finds one.
            let farthest = farthest(lean, main);
            assert!(farthest >= AWAY, "{lean:?} {main}: {farthest}");
            fell_back += usize::from(farthest < FAR_AWAY);
            for least in [AWAY, FAR_AWAY] {
                let mut random = Random::new(main as u64);
                for _ in 0..20 {
                    let tendency = Tendency::chosen(lean, Some(main), least, &mut random);
                    let off = turn(f64::from(tendency.hue), main).abs();
                    assert_eq!(tendency.lean, lean);
                    if farthest >= least {
                        assert!(off >= least, "{lean:?} {main} {least}: {tendency:?}");
                    } else {
                        assert_eq!(off, farthest, "{lean:?} {main}: the farthest");
                    }
                }
            }
        }
    }
    // Some crops have no hue of a lean `FAR_AWAY` from theirs: an orange one none of the warm leans'.
    assert!((1..4 * 72).contains(&fell_back), "{fell_back}");
    let far = Tendency::chosen(Lean::WarmPure, Some(60.0), FAR_AWAY, &mut Random::new(1));
    assert_eq!(
        far.hue, 350,
        "the warm end farthest from orange: a magenta red"
    );
    let cool = Tendency::chosen(Lean::CoolGrey, Some(60.0), FAR_AWAY, &mut Random::new(1));
    assert!(
        turn(f64::from(cool.hue), 60.0).abs() >= FAR_AWAY,
        "{cool:?}"
    );
    // Without a main hue any of the lean's hues, however far asked.
    let mut random = Random::new(5);
    let any: Vec<u16> = (0..200)
        .map(|_| Tendency::chosen(Lean::CoolPure, None, FAR_AWAY, &mut random).hue)
        .collect();
    assert!(hues(Lean::CoolPure).iter().all(|hue| any.contains(hue)));
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
            let random = &mut Random::new(k as u64);
            let tendency = Tendency::chosen(lean, main_hue(&areas), AWAY, random);
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
    let tendency = Tendency::chosen(Lean::CoolPure, main_hue(areas), AWAY, &mut Random::new(3));
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

/// The tendencies and crops of `count` key-shift lessons in a row at `level`, each taken by the
/// learner before the next is laid.
fn in_a_row(pictures: &Pictures, level: SkillLevel, count: usize) -> Vec<(Tendency, ItemRef)> {
    let skill = Skill::of_key("key-shift").expect("the key shifts");
    let mut learner = Learner::default();
    let mut laid = Vec::new();
    for _ in 0..count {
        let standing = Standing {
            pictures,
            learner: &learner,
        };
        let planned = schedule::lesson(skill, level, 0x5EED_0107, &standing).expect("a lesson");
        let [(exercise, _)] = planned.exercises.as_slice() else {
            panic!("{level:?}: one key shift");
        };
        let Kind::KeyShift(tendency) = exercise.kind else {
            panic!("{level:?}: a key shift");
        };
        laid.push((tendency, exercise.picture.clone().expect("its crop")));
        // 2026-10-05 12:00 UTC.
        let mut sheet = Sheet::begun(1, (Purpose::Day, 1_791_201_600), planned);
        learner = learner.with(&sheet.lessons.remove(0));
    }
    laid
}

/// The second level's first four key shifts on `crops(8)` (tendency code, crop) as 49edc6e laid
/// them, before S140.
const PINNED_SECOND: [(u8, &str); 4] = [
    (11, "crop-3"),
    (59, "crop-4"),
    (83, "crop-2"),
    (131, "crop-5"),
];

#[test]
fn a_key_shift_s_lesson_turns_every_lean_at_every_level_its_hue_far_at_the_third() {
    let pictures = Pictures::of(&[crops(8)]);
    let skill = Skill::of_key("key-shift").expect("the key shifts");
    assert_eq!(skill.needs(), Needs::Crops);
    let main = main_hue(&crops(1).items[0].areas.clone().unwrap()).unwrap();
    // Every level every lean in turn (the plan of 2026-11-06; S139, the first level S140): warm
    // and cool every lesson, pure and grey every second, so any three in a row hold both of each.
    for level in SkillLevel::ALL {
        let laid = in_a_row(&pictures, level, 8);
        let turned: Vec<Lean> = laid.iter().map(|(tendency, _)| tendency.lean).collect();
        assert_eq!(turned[..4], Lean::ALL, "{level:?}");
        assert_eq!(turned[4..], Lean::ALL, "{level:?}: round again");
        for three in turned.windows(3) {
            let warm = three.iter().filter(|lean| lean.warm()).count();
            let pure = three.iter().filter(|lean| lean.pure()).count();
            assert!(
                (1..=2).contains(&warm) && (1..=2).contains(&pure),
                "{three:?}"
            );
        }
        // The crops' main hue is orange-brown: no tendency within `AWAY` of it, none within
        // `FAR_AWAY` at the third level but the warm leans' farthest.
        let least = if level == SkillLevel::Third {
            FAR_AWAY
        } else {
            AWAY
        };
        for (tendency, _) in &laid {
            let off = turn(f64::from(tendency.hue), main).abs();
            let want = least.min(farthest(tendency.lean, main));
            assert!(off >= want, "{level:?}: {tendency:?}");
        }
    }
    // Same seed, same exercise: the second level lays what it laid before S140 (its crops of
    // five areas fall back to these simple ones), every tendency an existing variant.
    let laid = in_a_row(&pictures, SkillLevel::Second, 4);
    let second: Vec<(u8, &str)> = (laid.iter())
        .map(|(tendency, crop)| (tendency.code(), crop.item.as_str()))
        .collect();
    assert_eq!(second, PINNED_SECOND);
    // Every level picks its crops alike here: none twice while another is left.
    let crops_of = |level| -> Vec<ItemRef> {
        let laid = in_a_row(&pictures, level, 8);
        laid.into_iter().map(|(_, crop)| crop).collect()
    };
    let picked = crops_of(SkillLevel::First);
    let mut each = picked.clone();
    each.sort_by(|a, b| a.item.cmp(&b.item));
    each.dedup();
    assert_eq!(each.len(), 8, "{picked:?}");
    assert_eq!(crops_of(SkillLevel::Second), picked);
    assert_eq!(crops_of(SkillLevel::Third), picked);
    // Without crops: no key shift.
    let learner = Learner::default();
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
