//! Gamut masks (S153): masks kept in their kind's code, keeping their rules seed after seed (the
//! triangle and the diamond hold the greys, the small triangle none; every swatch inside its
//! mask; the apex away from the crop's own hue); a lesson at each level; the How. How a copy is
//! judged: `judged.rs`.

mod judged;

use std::sync::Arc;

use openpaint_practice::{
    AWAY, Aimed, CELLS, FAR_AWAY, INSIDE, ItemRef, Kind, Learner, Mask, MaskShape, Needs, Pack,
    PackItem, PictureKind, Pictures, Point, Purpose, Random, Rgb8, Sheet, Skill, SkillLevel,
    Standing, Tier, gamut_distance, gamut_span, main_hue, oklab, schedule, turn,
};

use judged::quadrants;

fn ab(colour: Rgb8) -> Point {
    let [_, a, b] = oklab(colour);
    [a, b]
}

/// Every mask there is.
fn every_mask() -> Vec<Mask> {
    (0..=u8::MAX).filter_map(Mask::of_code).collect()
}

#[test]
fn a_mask_reads_back_from_its_code_and_every_code_names_one() {
    let masks = every_mask();
    assert_eq!(masks.len(), 108, "3 shapes, 36 hues");
    for mask in &masks {
        assert_eq!(Mask::of_code(mask.code()), Some(*mask));
        assert_eq!(Kind::of_code(24, mask.code()), Some(Kind::GamutMask(*mask)));
        assert_eq!(mask.hue % 10, 0);
    }
    assert_eq!(Mask::of_code(108), None);
    let kind = Kind::GamutMask(masks[0]);
    assert_eq!(kind.facts().key, "gamut-mask");
    assert_eq!(kind.facts().needs, Needs::Crops);
    assert!(!kind.facts().picks.original, "its crop never picked");
}

/// How far the ray from grey toward `hue` (degrees) reaches inside the polygon `corners`.
fn reach(corners: &[Point], hue: u16) -> f64 {
    let h = f64::from(hue).to_radians();
    gamut_span(corners, [h.cos(), h.sin()]).map_or(0.0, |(_, far)| far)
}

#[test]
fn masks_keep_their_rules_seed_after_seed() {
    for mask in every_mask() {
        let corners = mask.corners();
        let count = if mask.shape == MaskShape::Diamond {
            4
        } else {
            3
        };
        assert_eq!(corners.len(), count, "{mask:?}");
        // Convex, counter-clockwise.
        for k in 0..count {
            let (a, b, c) = (
                corners[k],
                corners[(k + 1) % count],
                corners[(k + 2) % count],
            );
            let turned = (b[0] - a[0]) * (c[1] - b[1]) - (b[1] - a[1]) * (c[0] - b[0]);
            assert!(turned > 0.0, "{mask:?}");
        }
        // The apex points to its hue.
        let apex = corners[0][1].atan2(corners[0][0]).to_degrees();
        assert!(turn(apex, f64::from(mask.hue)).abs() < 1e-9, "{mask:?}");
        // The triangle and the diamond hold every grey up to a step of chroma at every hue; the
        // small triangle no grey.
        let grey = gamut_distance(&corners, [0.0, 0.0]);
        let greys = (0..36).all(|k| {
            let hue = f64::from(k * 10).to_radians();
            mask.holds([0.03 * hue.cos(), 0.03 * hue.sin()])
        });
        match mask.shape {
            MaskShape::Triangle | MaskShape::Diamond => assert!(grey == 0.0 && greys, "{mask:?}"),
            MaskShape::Small => assert!(grey > INSIDE && !mask.holds([0.0, 0.0]), "{mask:?}"),
        }
        assert_eq!(gamut_distance(&corners, mask.middle()), 0.0);
        // Gurney's triangle reaches a pure hue; the diamond two opposite ones, narrow.
        let pure = match mask.shape {
            MaskShape::Triangle => 0.16,
            MaskShape::Diamond => 0.14,
            MaskShape::Small => 0.12,
        };
        assert!(reach(&corners, mask.hue) >= pure, "{mask:?}");
        if mask.shape == MaskShape::Diamond {
            assert!(reach(&corners, (mask.hue + 180) % 360) >= pure, "{mask:?}");
            assert!(reach(&corners, (mask.hue + 90) % 360) < 0.05, "{mask:?}");
        }
        // Every corner's swatch lies inside its mask.
        let swatches = mask.swatches();
        assert_eq!(swatches.len(), count);
        for swatch in swatches {
            assert!(mask.holds(ab(swatch)), "{mask:?}: {swatch:?}");
        }
    }
}

#[test]
fn a_mask_keeps_its_least_turn_from_the_crop_s_main_hue() {
    for main in (0..360).step_by(5) {
        let main = f64::from(main);
        for (shape, least) in [
            (MaskShape::Triangle, AWAY),
            (MaskShape::Diamond, AWAY),
            (MaskShape::Small, FAR_AWAY),
        ] {
            let mut random = Random::new(main as u64);
            for _ in 0..12 {
                let mask = Mask::chosen(shape, Some(main), least, &mut random);
                assert_eq!(mask.shape, shape);
                let apex = turn(f64::from(mask.hue), main).abs();
                let off = match shape {
                    MaskShape::Diamond => apex.min(180.0 - apex),
                    _ => apex,
                };
                assert!(off >= least, "{shape:?} {main}: {mask:?}");
            }
        }
    }
    // Without a main hue, any.
    let mut random = Random::new(3);
    let hues: Vec<u16> = (0..300)
        .map(|_| Mask::chosen(MaskShape::Small, None, FAR_AWAY, &mut random).hue)
        .collect();
    assert!((0..36).all(|k| hues.contains(&(k * 10))));
}

/// A pack of `count` crops of each tier, the quadrants' areas.
fn crops(count: usize) -> Pack {
    let areas = Arc::new(quadrants());
    let item = |n: usize, tier: Tier| PackItem {
        id: format!("{}-{n}", tier.name()),
        kind: PictureKind::Crop,
        tier,
        size: [600, 600],
        title: "test".to_owned(),
        credit: None,
        outline: None,
        masses: None,
        areas: Some(Arc::clone(&areas)),
    };
    Pack {
        id: "test-crops".to_owned(),
        title: "Test crops".to_owned(),
        items: [Tier::Simple, Tier::Medium, Tier::Hard]
            .into_iter()
            .flat_map(|tier| (0..count).map(move |n| item(n, tier)))
            .collect(),
    }
}

/// The masks and crops of `count` gamut mask lessons in a row at `level`.
fn in_a_row(pictures: &Pictures, level: SkillLevel, count: usize) -> Vec<(Mask, ItemRef)> {
    let skill = Skill::of_key("gamut-mask").expect("gamut masks");
    let mut learner = Learner::default();
    let mut laid = Vec::new();
    for _ in 0..count {
        let standing = Standing {
            pictures,
            learner: &learner,
        };
        let planned = schedule::lesson(skill, level, 0x5EED_0153, &standing).expect("a lesson");
        let [(exercise, _)] = planned.exercises.as_slice() else {
            panic!("{level:?}: one gamut mask");
        };
        let Kind::GamutMask(mask) = exercise.kind else {
            panic!("{level:?}: a gamut mask");
        };
        laid.push((mask, exercise.picture.clone().expect("its crop")));
        // 2026-10-05 12:00 UTC.
        let mut sheet = Sheet::begun(1, (Purpose::Day, 1_791_201_600), planned);
        learner = learner.with(&sheet.lessons.remove(0));
    }
    laid
}

#[test]
fn a_lesson_lays_one_crop_its_mask_harder_by_level_and_away_from_the_crop_s_hue() {
    let pictures = Pictures::of(&[crops(4)]);
    let skill = Skill::of_key("gamut-mask").expect("gamut masks");
    assert_eq!(skill.needs(), Needs::Crops);
    assert_eq!(skill.window(), 3, "a lesson of one");
    let main = main_hue(&quadrants()).expect("a main hue");
    for (level, shape, tier, least) in [
        (SkillLevel::First, MaskShape::Triangle, "simple", AWAY),
        (SkillLevel::Second, MaskShape::Diamond, "medium", AWAY),
        (SkillLevel::Third, MaskShape::Small, "hard", FAR_AWAY),
    ] {
        let laid = in_a_row(&pictures, level, 4);
        let mut crops: Vec<&str> = laid.iter().map(|(_, crop)| crop.item.as_str()).collect();
        assert!(crops.iter().all(|crop| crop.starts_with(tier)), "{crops:?}");
        crops.sort_unstable();
        crops.dedup();
        assert_eq!(crops.len(), 4, "none twice while another is left");
        for (mask, _) in &laid {
            assert_eq!(mask.shape, shape);
            let apex = turn(f64::from(mask.hue), main).abs();
            let off = if shape == MaskShape::Diamond {
                apex.min(180.0 - apex)
            } else {
                apex
            };
            assert!(off >= least, "{level:?}: {mask:?}");
        }
    }
    let learner = Learner::default();
    let none = Pictures::of(&[]);
    let bare = Standing {
        pictures: &none,
        learner: &learner,
    };
    assert_eq!(skill.lacks(&none), Some(Needs::Crops));
    assert!(schedule::lesson(skill, SkillLevel::First, 7, &bare).is_none());
}

#[test]
fn the_how_paints_its_example_inside_the_mask_step_by_step_and_compares_it_in_grey() {
    let mask = Mask::of_code(6).expect("a mask");
    let demo = Kind::GamutMask(mask).how(CELLS).expect("a demonstration");
    assert_eq!(demo.steps.len(), 5, "look, fill, big, small, Compare");
    assert!((4.5..=10.3).contains(&demo.length), "{}", demo.length);
    let strip = demo.strip.as_ref().expect("its mask and chips");
    let Aimed::Mask(corners) = &strip.aimed else {
        panic!("the mask under the copy")
    };
    assert_eq!(strip.chips.len(), 5, "a chip an area");
    assert!(strip.looked < strip.chips[0].0);
    for (_, [original, masked]) in &strip.chips {
        let kept = (oklab(*original)[0] - oklab(*masked)[0]).abs();
        assert!(kept < 0.01, "lightness kept");
        assert!(
            gamut_distance(corners, ab(*masked)) <= INSIDE / 2.0,
            "inside"
        );
    }
    let outside =
        (strip.chips.iter()).filter(|(_, [o, _])| gamut_distance(corners, ab(*o)) > INSIDE);
    assert!(outside.count() >= 2, "the example moved into the mask");
    assert_eq!(demo.areas.as_ref().expect("its example").areas.count(), 5);
    let again = Kind::GamutMask(mask).how(CELLS);
    assert_eq!(again, Some(demo), "the same every time");
}
