//! Stage 2 (S80): sessions of single shapes by tier with the user's own pictures, the warm-up from
//! every kind met, and a shape's outline aligned onto a copy with lines inside it.

use openpaint_practice::{
    Arrangement, Before, BoxView, Combination, Exercise, Family, Fit, Guide, Horizon, ItemRef,
    Kind, LineTier, PERSONAL, Pack, PackItem, PictureKind, Pictures, Point, ScaleTone, Tier, Tone,
    WARM_UP_TWO, align_shape, plan, stage_two,
};

fn item(id: &str, tier: Tier) -> PackItem {
    PackItem {
        id: id.to_owned(),
        kind: PictureKind::Shape,
        tier,
        size: [800, 600],
        title: id.to_owned(),
        credit: None,
        outline: None,
    }
}

/// An official pack of `per_tier` pictures in each of `tiers` (credits left out: the plan reads
/// ids and tiers only), and the user's own `mine`, simple ones.
fn pictures(tiers: &[Tier], per_tier: usize, mine: usize) -> Pictures {
    with_own(tiers, per_tier, &vec![Tier::Simple; mine])
}

/// As `pictures`, the user's own of the tiers `mine`.
fn with_own(tiers: &[Tier], per_tier: usize, mine: &[Tier]) -> Pictures {
    let official = Pack {
        id: "test-shapes".to_owned(),
        title: "Test shapes".to_owned(),
        items: (tiers.iter())
            .flat_map(|tier| {
                (0..per_tier).map(move |n| item(&format!("{}-{n}", tier.name()), *tier))
            })
            .collect(),
    };
    let personal = Pack {
        id: PERSONAL.to_owned(),
        title: "My pictures".to_owned(),
        items: (mine.iter().enumerate())
            .map(|(n, tier)| item(&format!("mine-{n}"), *tier))
            .collect(),
    };
    Pictures::of(&[official, personal])
}

fn before<'a>(met: &'a [Kind], pictures: &'a Pictures, used: &'a [ItemRef]) -> Before<'a> {
    Before {
        met,
        pictures,
        used,
    }
}

fn shapes(exercises: &[Exercise]) -> Vec<(Tier, ItemRef)> {
    exercises
        .iter()
        .filter_map(|e| match e.kind {
            Kind::Shape(tier) => Some((tier, e.picture.clone().expect("a picture"))),
            _ => None,
        })
        .collect()
}

#[test]
fn a_first_stage_two_session_is_four_shapes_of_the_easy_tiers_then_colour_work_and_perspective() {
    let all = pictures(&Tier::ALL, 5, 0);
    let plan = stage_two(1, 0x5EED_0045, &before(&[], &all, &[]));
    assert_eq!(
        plan,
        stage_two(1, 0x5EED_0045, &before(&[], &all, &[])),
        "the same per seed"
    );
    assert_eq!(plan.warm_up, 0, "nothing met");
    let families: Vec<Family> = plan.exercises.iter().map(|e| e.kind.family()).collect();
    let expected = [
        Family::Shape,
        Family::Shape,
        Family::Shape,
        Family::Shape,
        Family::Areas,
        Family::Scale,
        Family::Box,
        Family::Ellipse,
    ];
    assert_eq!(families, expected);
    let picked = shapes(&plan.exercises);
    let tiers: Vec<Tier> = picked.iter().map(|(tier, _)| *tier).collect();
    assert_eq!(
        tiers,
        [Tier::Entry, Tier::Entry, Tier::Simple, Tier::Simple]
    );
    for (tier, item) in &picked {
        assert!(item.item.starts_with(tier.name()), "{item:?} of {tier:?}");
    }
    let mut distinct: Vec<&ItemRef> = picked.iter().map(|(_, item)| item).collect();
    distinct.dedup();
    assert_eq!(distinct.len(), 4, "no picture twice");
    // The harder tiers come in as sessions go.
    let later = |number| {
        let tiers: Vec<Tier> = shapes(&stage_two(number, 7, &before(&[], &all, &[])).exercises)
            .into_iter()
            .map(|(tier, _)| tier)
            .collect();
        tiers
    };
    assert_eq!(
        later(4),
        [Tier::Entry, Tier::Simple, Tier::Simple, Tier::Medium]
    );
    assert_eq!(
        later(7),
        [Tier::Simple, Tier::Simple, Tier::Medium, Tier::Hard]
    );
    assert_eq!(
        later(14),
        [Tier::Simple, Tier::Medium, Tier::Medium, Tier::Hard]
    );
}

#[test]
fn the_combination_s_arrangement_and_the_scale_s_tone_come_in_turn() {
    let all = pictures(&Tier::ALL, 5, 0);
    let colour_work = |number| -> Vec<Kind> {
        let plan = stage_two(number, 9, &before(&[], &all, &[]));
        plan.exercises[4..6].iter().map(|e| e.kind).collect()
    };
    assert_eq!(
        colour_work(1),
        [
            Kind::Areas(Arrangement::Strata),
            Kind::Scale(ScaleTone::Grey)
        ]
    );
    assert_eq!(
        colour_work(2),
        [
            Kind::Areas(Arrangement::Patches),
            Kind::Scale(ScaleTone::Hue)
        ]
    );
    assert_eq!(colour_work(3)[0], Kind::Areas(Arrangement::Blocks));
}

#[test]
fn perspective_comes_in_turn_a_box_and_an_ellipse_then_free_boxes() {
    let all = pictures(&Tier::ALL, 5, 0);
    let perspective = |number| -> Vec<Kind> {
        let plan = stage_two(number, 13, &before(&[], &all, &[]));
        plan.exercises[6..].iter().map(|e| e.kind).collect()
    };
    let box_and_ellipse = |view, guide| vec![Kind::Box(view), Kind::Ellipse(guide)];
    assert_eq!(
        perspective(1),
        box_and_ellipse(BoxView::TwoPoint, Guide::Frame)
    );
    assert_eq!(perspective(2), [Kind::FreeBoxes(Horizon::Middle)]);
    assert_eq!(
        perspective(3),
        box_and_ellipse(BoxView::ThreePoint, Guide::Plane)
    );
    assert_eq!(perspective(4), [Kind::FreeBoxes(Horizon::High)]);
    assert_eq!(
        perspective(5),
        box_and_ellipse(BoxView::TwoPoint, Guide::Frame)
    );
    assert_eq!(perspective(6), [Kind::FreeBoxes(Horizon::Low)]);
    // Boxes and ellipses met come back in the warm-up as quick kinds, free boxes as the long one.
    let met = [
        Kind::Box(BoxView::TwoPoint),
        Kind::FreeBoxes(Horizon::Middle),
        Kind::Ellipse(Guide::Frame),
    ];
    let plan = stage_two(3, 5, &before(&met, &all, &[]));
    let warm: Vec<Family> = (plan.exercises[..WARM_UP_TWO].iter())
        .map(|e| e.kind.family())
        .collect();
    assert_eq!(warm, [Family::Box, Family::FreeBoxes, Family::Ellipse]);
}

#[test]
fn the_user_s_own_picture_takes_the_last_place_and_pictures_shown_wait_their_turn() {
    let with_mine = pictures(&Tier::ALL, 2, 3);
    let plan = stage_two(1, 11, &before(&[], &with_mine, &[]));
    let picked = shapes(&plan.exercises);
    assert_eq!(picked.len(), 4);
    assert_eq!(
        picked[3].1.pack, PERSONAL,
        "the last shape is the user's own"
    );
    assert_eq!(picked[3].0, Tier::Simple, "the tier chosen for it");
    assert!(
        picked[..3]
            .iter()
            .all(|(_, item)| item.pack == "test-shapes")
    );
    // Entry's two pictures both shown before: a session still takes them (none is fresh).
    let used: Vec<ItemRef> = picked.iter().map(|(_, item)| item.clone()).collect();
    let next = shapes(&stage_two(2, 12, &before(&[], &with_mine, &used)).exercises);
    assert!(
        !used.contains(&next[3].1),
        "another of the user's own while one is fresh"
    );
    let simple_left: Vec<&ItemRef> = (next.iter())
        .filter(|(tier, _)| *tier == Tier::Simple)
        .map(|(_, item)| item)
        .collect();
    assert!(simple_left.iter().all(|item| !used.contains(item)) || simple_left.len() > 1);
}

/// The tier the user chose for a picture of their own: it takes the last place once the session's
/// tiers reach it (simple from the first, medium from the third, hard from the sixth), the
/// shape's kind its tier; before that the place is an official picture's.
#[test]
fn a_picture_of_my_own_comes_once_the_sessions_reach_its_tier() {
    let last = |mine: &[Tier], number| {
        let all = with_own(&Tier::ALL, 3, mine);
        let picked = shapes(&stage_two(number, 21, &before(&[], &all, &[])).exercises);
        picked[3].clone()
    };
    let (tier, item) = last(&[Tier::Hard], 1);
    assert_eq!((tier, item.pack.as_str()), (Tier::Simple, "test-shapes"));
    assert_eq!(last(&[Tier::Hard], 5).1.pack, "test-shapes");
    let (tier, item) = last(&[Tier::Hard], 6);
    assert_eq!((tier, item.pack.as_str()), (Tier::Hard, PERSONAL));
    let (tier, item) = last(&[Tier::Medium], 3);
    assert_eq!((tier, item.pack.as_str()), (Tier::Medium, PERSONAL));
    let (tier, item) = last(&[Tier::Entry, Tier::Hard], 1);
    assert_eq!(
        (tier, item.item.as_str()),
        (Tier::Entry, "mine-0"),
        "the one it reaches"
    );
}

#[test]
fn a_tier_without_pictures_falls_back_and_no_pictures_means_no_shapes() {
    let medium_only = pictures(&[Tier::Medium], 6, 0);
    let picked = shapes(&stage_two(1, 3, &before(&[], &medium_only, &[])).exercises);
    assert_eq!(picked.len(), 4);
    assert!(
        picked.iter().all(|(tier, _)| *tier == Tier::Medium),
        "{picked:?}"
    );
    let none = Pictures::default();
    assert!(none.is_empty());
    let plan = stage_two(1, 3, &before(&[], &none, &[]));
    assert!(shapes(&plan.exercises).is_empty());
    assert_eq!(
        plan.exercises.len(),
        4,
        "the combination, the scale, a box and an ellipse"
    );
    assert_eq!(stage_two(2, 3, &before(&[], &none, &[])).exercises.len(), 3);
}

#[test]
fn the_warm_up_takes_one_long_kind_in_turn_and_quick_ones_in_turn() {
    let met = [
        Kind::Line(LineTier::Arc),
        Kind::Combined(Combination::Near),
        Kind::Shape(Tier::Simple),
        Kind::Colour(Tone::of_code(1).expect("a tone")),
        Kind::Areas(Arrangement::Patches),
        Kind::Scale(ScaleTone::Grey),
    ];
    let all = pictures(&Tier::ALL, 4, 0);
    let warm = |number, seed| -> Vec<Family> {
        let plan = stage_two(number, seed, &before(&met, &all, &[]));
        assert_eq!(plan.warm_up as usize, WARM_UP_TWO);
        (plan.exercises[..WARM_UP_TWO].iter())
            .map(|e| e.kind.family())
            .collect()
    };
    for seed in 0..50 {
        assert_eq!(
            warm(1, seed),
            [Family::Line, Family::Combined, Family::Shape]
        );
        assert_eq!(
            warm(2, seed),
            [Family::Combined, Family::Colour, Family::Areas]
        );
        assert_eq!(warm(3, seed), [Family::Line, Family::Colour, Family::Scale]);
    }
    // Over three sessions every family met comes back, and never two long ones in one.
    let mut seen: Vec<Family> = (1..=3).flat_map(|number| warm(number, 7)).collect();
    seen.sort();
    seen.dedup();
    assert_eq!(seen.len(), 6);
    // Only long kinds met: the one long exercise.
    let long_only = [Kind::Shape(Tier::Simple), Kind::Scale(ScaleTone::Hue)];
    let plan = stage_two(2, 5, &before(&long_only, &all, &[]));
    assert_eq!(plan.warm_up, 1);
    assert_eq!(plan.exercises[0].kind, Kind::Scale(ScaleTone::Hue));
}

#[test]
fn the_warm_up_takes_shapes_once_met_with_pictures_of_their_tiers() {
    let all = pictures(&Tier::ALL, 4, 0);
    let met = [
        Kind::Line(LineTier::Arc),
        Kind::Combined(Combination::Near),
        Kind::Shape(Tier::Simple),
        Kind::Colour(Tone::of_code(1).expect("a tone")),
    ];
    for seed in 0..100 {
        let plan = stage_two(5, seed, &before(&met, &all, &[]));
        assert_eq!(plan.warm_up as usize, WARM_UP_TWO);
        let warm: Vec<Family> = (plan.exercises[..WARM_UP_TWO].iter())
            .map(|e| e.kind.family())
            .collect();
        // Session 5: the shape, and the quick families from the fifth turn on.
        assert_eq!(warm, [Family::Combined, Family::Shape, Family::Colour]);
        let shape = &plan.exercises[1];
        assert_eq!(shape.kind, Kind::Shape(Tier::Simple));
        assert!(
            shape
                .picture
                .as_ref()
                .is_some_and(|p| p.item.starts_with("simple"))
        );
        let all_pictures = shapes(&plan.exercises);
        let mut items: Vec<&ItemRef> = all_pictures.iter().map(|(_, i)| i).collect();
        items.sort();
        items.dedup();
        assert_eq!(items.len(), all_pictures.len(), "{seed}: no picture twice");
    }
    // Stage 1 revisited keeps its own mix; a shape met is no stage-1 warm-up.
    let stage_one = plan(1, 3, 9, &before(&met, &all, &[]));
    assert!(stage_one.exercises.iter().all(|e| e.picture.is_none()));
    assert!((stage_one.exercises.iter()).all(|e| e.kind.family() != Family::Shape));
}

/// A closed outline in frame units: a vase-like shape with a neck.
fn vase() -> Vec<Point> {
    let mut points = Vec::new();
    for k in 0..=120 {
        let t = f64::from(k) / 120.0 * std::f64::consts::TAU;
        let neck = 0.12 + 0.05 * (2.0 * t).cos().abs();
        let r = if t.sin() < -0.5 { neck } else { 0.3 };
        points.push([0.5 + r * t.cos(), 0.5 + 0.35 * t.sin()]);
    }
    points
}

fn moved(points: &[Point], fit: Fit) -> Vec<Point> {
    points.iter().map(|p| fit.apply(*p)).collect()
}

#[test]
fn a_shape_s_outline_lands_on_a_moved_and_sized_copy_whatever_lies_inside() {
    let outline = vase();
    let truth = Fit {
        scale: 0.8,
        shift: [0.07, -0.04],
    };
    let copy = moved(&outline, truth);
    // Lines inside the copy: a fold down the middle, a band across, a small mark.
    let fold: Vec<Point> = (0..20)
        .map(|k| [0.47 + 0.002 * f64::from(k), 0.3 + 0.02 * f64::from(k)])
        .collect();
    let band: Vec<Point> = (0..20)
        .map(|k| [0.35 + 0.015 * f64::from(k), 0.5])
        .collect();
    let mark: Vec<Point> = vec![[0.5, 0.62], [0.52, 0.64]];
    let inside: Vec<Vec<Point>> = [fold, band, mark].iter().map(|l| moved(l, truth)).collect();
    let mut lines: Vec<&[Point]> = vec![copy.as_slice()];
    lines.extend(inside.iter().map(Vec::as_slice));
    let fit = align_shape(&[&outline], &lines).expect("a fit");
    assert!((fit.scale - truth.scale).abs() < 0.02, "{fit:?}");
    let error = (fit.shift[0] - truth.shift[0]).hypot(fit.shift[1] - truth.shift[1]);
    assert!(error < 0.015, "{fit:?}");
    // Drawn in three pieces with a gap, it still lands.
    let pieces: Vec<&[Point]> = vec![&copy[..40], &copy[45..90], &copy[95..]];
    let fit = align_shape(&[&outline], &pieces).expect("a fit");
    assert!((fit.scale - truth.scale).abs() < 0.03, "{fit:?}");
    // A copy turned 12 degrees is never turned back: about its size, no turn in a fit.
    let (cos, sin) = (12f64.to_radians().cos(), 12f64.to_radians().sin());
    let turned: Vec<Point> = (outline.iter())
        .map(|p| {
            let [x, y] = [p[0] - 0.5, p[1] - 0.5];
            [0.5 + cos * x - sin * y, 0.5 + sin * x + cos * y]
        })
        .collect();
    let fit = align_shape(&[&outline], &[&turned]).expect("a fit");
    assert!((fit.scale - 1.0).abs() < 0.1, "{fit:?}");
    assert_eq!(
        align_shape(&[&outline], &[&[[0.5, 0.5]]]),
        None,
        "no extent"
    );
}
