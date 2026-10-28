//! Stage 3 (S90): sessions of estimates on the flat and points in space, V大's five tricks in
//! turn, two shapes of the medium and hard tiers, a colour combination, and a warm-up of three
//! from the kinds met.

use openpaint_practice::{
    Before, Estimate, Family, ItemRef, Kind, LineTier, Pack, PackItem, PictureKind, Pictures,
    STAGES_READY, Tier, Trick, WARM_UP_TWO, stage_three,
};

fn pictures(tiers: &[Tier]) -> Pictures {
    let item = |id: String, tier: Tier| PackItem {
        id,
        kind: PictureKind::Shape,
        tier,
        size: [800, 600],
        title: "test".to_owned(),
        credit: None,
        outline: None,
    };
    let pack = Pack {
        id: "test-shapes".to_owned(),
        title: "Test shapes".to_owned(),
        items: (tiers.iter())
            .flat_map(|tier| (0..4).map(move |n| item(format!("{}-{n}", tier.name()), *tier)))
            .collect(),
    };
    Pictures::of(&[pack])
}

fn before<'a>(met: &'a [Kind], pictures: &'a Pictures, used: &'a [ItemRef]) -> Before<'a> {
    Before {
        met,
        pictures,
        used,
    }
}

fn tricks(kinds: &[Kind]) -> Vec<Trick> {
    (kinds.iter())
        .filter_map(|kind| match kind {
            Kind::Space(trick) => Some(*trick),
            _ => None,
        })
        .collect()
}

#[test]
fn a_first_stage_three_session_is_estimates_points_in_space_shapes_and_a_combination() {
    assert_eq!(STAGES_READY, 3);
    let all = pictures(&Tier::ALL);
    let plan = stage_three(1, 0x5EED_0090, &before(&[], &all, &[]));
    assert_eq!(plan, stage_three(1, 0x5EED_0090, &before(&[], &all, &[])));
    assert_eq!(plan.warm_up, 0, "nothing met");
    let kinds: Vec<Kind> = plan.exercises.iter().map(|e| e.kind).collect();
    let flat = [Estimate::Point, Estimate::Proportion, Estimate::Angle].map(Kind::Flat);
    assert_eq!(kinds[..6], [flat, flat].concat());
    assert_eq!(
        tricks(&kinds),
        [Trick::Centre, Trick::Halves, Trick::Thirds]
    );
    assert_eq!(
        kinds[9..11],
        [Kind::Shape(Tier::Medium), Kind::Shape(Tier::Hard)]
    );
    for exercise in &plan.exercises[9..11] {
        let picture = exercise.picture.as_ref().expect("a picture");
        let Kind::Shape(tier) = exercise.kind else {
            panic!("a shape")
        };
        assert!(picture.item.starts_with(tier.name()));
    }
    assert_eq!(kinds[11].family(), Family::Areas);
    assert_eq!(kinds.len(), 12);
    // Without a pack's pictures, no shapes.
    let none = pictures(&[]);
    let bare = plan_of(1, &[], &none);
    assert_eq!(bare.len(), 10);
    assert!(bare.iter().all(|kind| kind.family() != Family::Shape));
    // `plan` gives stage 3 its own.
    let planned = openpaint_practice::plan(3, 1, 0x5EED_0090, &before(&[], &all, &[]));
    assert_eq!(
        planned,
        stage_three(1, 0x5EED_0090, &before(&[], &all, &[]))
    );
}

fn plan_of(number: u32, met: &[Kind], pictures: &Pictures) -> Vec<Kind> {
    let plan = stage_three(number, 7, &before(met, pictures, &[]));
    plan.exercises.iter().map(|e| e.kind).collect()
}

#[test]
fn the_five_tricks_come_in_turn_and_their_codes_hold() {
    let none = pictures(&[]);
    let mut seen = Vec::new();
    for number in 1..=5 {
        seen.extend(tricks(&plan_of(number, &[], &none)));
    }
    assert_eq!(&seen[3..6], [Trick::Step, Trick::Mirror, Trick::Centre]);
    for trick in Trick::ALL {
        assert_eq!(seen.iter().filter(|t| **t == trick).count(), 3, "{trick:?}");
        let (code, variant) = Kind::Space(trick).code();
        assert_eq!(code, 12);
        assert_eq!(Kind::of_code(code, variant), Some(Kind::Space(trick)));
        assert!(trick.family().drawn() && trick.family().geometric());
    }
    for estimate in Estimate::ALL {
        let (code, variant) = Kind::Flat(estimate).code();
        assert_eq!(code, 11);
        assert_eq!(Kind::of_code(code, variant), Some(Kind::Flat(estimate)));
        assert!(estimate.family().drawn() && estimate.family().geometric());
    }
    for family in [Family::Line, Family::Combined, Family::Shape] {
        assert!(family.drawn() && !family.geometric(), "{family:?}");
    }
    for family in [Family::Colour, Family::Areas, Family::Scale, Family::Study] {
        assert!(!family.drawn() && !family.geometric(), "{family:?}");
    }
}

#[test]
fn the_warm_up_holds_three_from_the_kinds_met_at_most_one_long() {
    let all = pictures(&Tier::ALL);
    let met = [
        Kind::Line(LineTier::Arc),
        Kind::Shape(Tier::Simple),
        Kind::Flat(Estimate::Angle),
        Kind::Space(Trick::Mirror),
        Kind::Areas(openpaint_practice::Arrangement::Strata),
    ];
    for number in 2..12 {
        let plan = stage_three(number, 11, &before(&met, &all, &[]));
        assert_eq!(plan.warm_up as usize, WARM_UP_TWO, "{number}");
        let warm = &plan.exercises[..WARM_UP_TWO];
        assert!(
            warm.iter()
                .all(|e| met.iter().any(|k| k.family() == e.kind.family()))
        );
        let long = (warm.iter())
            .filter(|e| matches!(e.kind.family(), Family::Shape | Family::Areas))
            .count();
        assert_eq!(long, 1, "{number}");
        let rest = &plan.exercises[WARM_UP_TWO..];
        assert_eq!(rest.len(), 12, "the mix whatever the warm-up");
    }
}
