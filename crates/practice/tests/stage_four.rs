//! Stage 4 (S94, S95): sessions of three objects simplified into boxes (their carves in turn), then
//! a division and a whole copy of the composition pack's pictures (the simple and medium tiers in
//! turn, none twice in a session, none of the last ten sessions' while others are left); without
//! a composition S94's mix (a solid in turn, a medium or hard shape in turn, a colour combination
//! in turn); after a warm-up of three from the kinds met, never an object, a solid, a division or
//! a whole copy. Divisions and whole copies drilled likewise. Where a picture's value masses read
//! (S96), value masses in the first, third, fifth session in the whole copy's place.

use std::collections::HashSet;
use std::sync::Arc;

use openpaint_practice::{
    Arrangement, Before, Carve, Family, ItemRef, Kind, LineTier, Metric, Pack, PackItem,
    PictureKind, Pictures, STAGES_READY, Solid, Tier, ValueMasses, WARM_UP_TWO, drill, kinds_of,
    plan, stage_four,
};

/// The crops' tiers (S101).
const CROPS: [Tier; 3] = [Tier::Simple, Tier::Medium, Tier::Hard];

fn pack(id: &str, kind: PictureKind, tiers: &[Tier]) -> Pack {
    let item = |id: String, tier: Tier| PackItem {
        id,
        kind,
        tier,
        size: [800, 600],
        title: "test".to_owned(),
        credit: None,
        outline: None,
        masses: None,
        areas: None,
    };
    Pack {
        id: id.to_owned(),
        title: "Test".to_owned(),
        items: (tiers.iter())
            .flat_map(|tier| (0..4).map(move |n| item(format!("{}-{n}", tier.name()), *tier)))
            .collect(),
    }
}

fn pictures(tiers: &[Tier]) -> Pictures {
    let crops = (!tiers.is_empty()).then(|| pack("test-crops", PictureKind::Crop, &CROPS));
    let packs: Vec<Pack> = [Some(pack("test-shapes", PictureKind::Shape, tiers)), crops]
        .into_iter()
        .flatten()
        .collect();
    Pictures::of(&packs)
}

/// Shapes of every tier and compositions of `tiers`.
fn with_compositions(tiers: &[Tier]) -> Pictures {
    Pictures::of(&[
        pack("test-shapes", PictureKind::Shape, &Tier::ALL),
        pack("test-compositions", PictureKind::Composition, tiers),
        pack("test-crops", PictureKind::Crop, &CROPS),
    ])
}

fn before<'a>(met: &'a [Kind], pictures: &'a Pictures) -> Before<'a> {
    Before {
        met,
        pictures,
        used: &[],
        recent: &[],
        scales: 0,
    }
}

fn kinds(number: u32, met: &[Kind], pictures: &Pictures) -> Vec<Kind> {
    let plan = stage_four(number, 0x5EED_0094, &before(met, pictures));
    plan.exercises.iter().map(|e| e.kind).collect()
}

#[test]
fn a_first_stage_four_session_without_compositions_is_s94_s_mix() {
    assert_eq!(STAGES_READY, 4);
    assert_eq!(
        kinds_of(4),
        [
            Family::Object,
            Family::Division,
            Family::ValueMasses,
            Family::WholeCopy
        ]
    );
    let all = pictures(&Tier::ALL);
    let first = stage_four(1, 0x5EED_0094, &before(&[], &all));
    assert_eq!(first, plan(4, 1, 0x5EED_0094, &before(&[], &all)));
    assert_eq!(first.warm_up, 0, "nothing met");
    let held: Vec<Kind> = first.exercises.iter().map(|e| e.kind).collect();
    assert_eq!(
        held[..5],
        [
            Kind::Object(Carve::Block),
            Kind::Object(Carve::Cut),
            Kind::Object(Carve::Taper),
            Kind::Solid(Solid::Cylinder),
            Kind::Shape(Tier::Medium),
        ]
    );
    assert!(matches!(held[5], Kind::Crop(_)), "a combination of a crop");
    assert!(first.exercises[4].picture.is_some(), "the shape's picture");
    let crop = first.exercises[5].picture.as_ref().map(|p| p.pack.as_str());
    assert_eq!(crop, Some("test-crops"));
    let none = pictures(&[]);
    let bare = kinds(1, &[], &none);
    assert_eq!(
        bare.len(),
        4,
        "no shape without a picture, no combination without a crop"
    );
}

#[test]
fn with_compositions_a_session_is_three_objects_a_division_and_a_whole_copy() {
    let all = with_compositions(&[Tier::Simple, Tier::Medium, Tier::Hard]);
    for number in 1..=8 {
        let plan = stage_four(number, 0x5EED_0095 + u64::from(number), &before(&[], &all));
        let held: Vec<Kind> = plan.exercises.iter().map(|e| e.kind).collect();
        let turn = (number - 1) as usize;
        let carves = [0, 1, 2].map(|k| Kind::Object(Carve::ALL[(3 * turn + k) % 4]));
        let [first, second] = [turn, turn + 1].map(|k| [Tier::Simple, Tier::Medium][k % 2]);
        let mix = [
            carves[0],
            carves[1],
            carves[2],
            Kind::Division(first),
            Kind::WholeCopy(second),
        ];
        assert_eq!(held, mix, "session {number}");
        let shown: Vec<&ItemRef> = (plan.exercises[3..].iter())
            .map(|e| e.picture.as_ref().expect("a composition"))
            .collect();
        assert_ne!(shown[0], shown[1], "session {number}: no picture twice");
        assert!(shown.iter().all(|item| item.pack == "test-compositions"));
    }
    // Without the simple tier, the nearest kept.
    let medium = with_compositions(&[Tier::Medium]);
    let held = kinds(1, &[], &medium);
    assert_eq!(
        &held[3..],
        [Kind::Division(Tier::Medium), Kind::WholeCopy(Tier::Medium)]
    );
}

#[test]
fn a_composition_of_the_last_ten_sessions_comes_back_only_when_none_other_is_left() {
    let all = with_compositions(&[Tier::Simple, Tier::Medium]);
    let simple = |n: usize| ItemRef {
        pack: "test-compositions".to_owned(),
        item: format!("simple-{n}"),
    };
    // Three of the four simple ones shown lately: the fourth is the one left.
    let recent = [simple(0), simple(1), simple(2)];
    for seed in 0..40 {
        let lately = Before {
            recent: &recent,
            ..before(&[], &all)
        };
        let plan = stage_four(1, seed, &lately);
        let division = plan.exercises[3].picture.clone();
        assert_eq!(division, Some(simple(3)), "seed {seed}");
    }
    // All four lately: one of them again, never the same twice in a session.
    let every = [simple(0), simple(1), simple(2), simple(3)];
    let mut seen = HashSet::new();
    for seed in 0..40 {
        let lately = Before {
            recent: &every,
            ..before(&[], &all)
        };
        let plan = drill(Family::Division, 1, seed, &lately).expect("divisions");
        let shown: Vec<ItemRef> = (plan.exercises.iter())
            .filter_map(|e| e.picture.clone())
            .collect();
        assert_eq!(shown.len(), 3);
        assert_ne!(shown[0], shown[2], "seed {seed}: two simple, apart");
        seen.extend(shown);
    }
    assert!(seen.len() > 4, "simple and medium both picked");
}

#[test]
fn divisions_and_whole_copies_are_drilled_by_tier_in_turn_and_never_without_a_composition() {
    let all = with_compositions(&[Tier::Simple, Tier::Medium]);
    let tiers = |family: Family, number: u32| -> Vec<Tier> {
        let plan = drill(family, number, 0x5EED_0095, &before(&[], &all)).expect("a drill");
        assert_eq!(plan.warm_up, 0);
        (plan.exercises.iter())
            .map(|e| match e.kind {
                Kind::Division(tier) | Kind::WholeCopy(tier) => tier,
                other => panic!("{other:?}"),
            })
            .collect()
    };
    let [s, m] = [Tier::Simple, Tier::Medium];
    assert_eq!(tiers(Family::Division, 1), [s, m, s]);
    assert_eq!(tiers(Family::Division, 2), [m, s, m]);
    assert_eq!(tiers(Family::WholeCopy, 1), [s]);
    assert_eq!(tiers(Family::WholeCopy, 2), [m]);
    let shapes_only = pictures(&Tier::ALL);
    for family in [Family::Division, Family::WholeCopy] {
        assert_eq!(drill(family, 1, 9, &before(&[], &shapes_only)), None);
    }
}

#[test]
fn objects_solids_shapes_and_combinations_come_in_turn() {
    let all = pictures(&Tier::ALL);
    let sessions: Vec<Vec<Kind>> = (1..=12).map(|n| kinds(n, &[], &all)).collect();
    // Carves in turn: every four sessions show each three times.
    for carve in Carve::ALL {
        let held = (sessions[..4].iter().flatten())
            .filter(|k| **k == Kind::Object(carve))
            .count();
        assert_eq!(held, 3, "{carve:?}");
    }
    for (index, session) in sessions.iter().enumerate() {
        let solid = Solid::ALL[index % Solid::ALL.len()];
        assert!(
            session.contains(&Kind::Solid(solid)),
            "session {}",
            index + 1
        );
        let tier = [Tier::Medium, Tier::Hard][index % 2];
        assert!(
            session.contains(&Kind::Shape(tier)),
            "session {}",
            index + 1
        );

        assert!(session.iter().any(|k| matches!(k, Kind::Crop(_))), "a crop");
        let objects = session.iter().filter(|k| k.family() == Family::Object);
        assert_eq!(objects.count(), 3, "session {}", index + 1);
    }
}

#[test]
fn the_warm_up_is_three_of_the_kinds_met_never_an_object_a_solid_or_a_division() {
    let all = with_compositions(&[Tier::Simple, Tier::Medium]);
    let met = [
        Kind::Line(LineTier::Arc),
        Kind::Object(Carve::Cut),
        Kind::Solid(Solid::Cone),
        Kind::Shape(Tier::Medium),
        Kind::Areas(Arrangement::ALL[1]),
        Kind::Division(Tier::Simple),
        Kind::WholeCopy(Tier::Medium),
    ];
    for number in 1..=6 {
        let plan = stage_four(number, 0x5EED_0094 + u64::from(number), &before(&met, &all));
        assert_eq!(plan.warm_up as usize, WARM_UP_TWO, "session {number}");
        let warm_up = &plan.exercises[..WARM_UP_TWO];
        assert!(warm_up.iter().all(|e| {
            let family = e.kind.family();
            family != Family::Object
                && !family.divided()
                && !matches!(family, Family::Cylinder | Family::Cone)
        }));
        let long = (warm_up.iter())
            .filter(|e| matches!(e.kind.family(), Family::Shape | Family::Areas))
            .count();
        assert!(long <= 1, "session {number}: at most one long");
    }
    // An object's first error is kept by its own code; a division's counts and lengths by theirs.
    assert_eq!(Metric::ObjectOff.code(), 48);
    assert_eq!(Metric::of_code(48), Some(Metric::ObjectOff));
    let kind = Kind::Object(Carve::Round);
    assert_eq!(kind.code(), (14, 3));
    assert_eq!(Kind::of_code(14, 3), Some(kind));
    for (kind, code) in [
        (Kind::Division(Tier::Simple), (15, 2)),
        (Kind::WholeCopy(Tier::Medium), (16, 3)),
    ] {
        assert_eq!(kind.code(), code);
        assert_eq!(Kind::of_code(code.0, code.1), Some(kind));
        assert!(kind.family().drawn() && kind.family().divided());
        assert!(!kind.family().geometric() && !kind.family().painted());
    }
    let masses = Kind::ValueMasses(Tier::Medium);
    assert_eq!(masses.code(), (17, 3));
    assert_eq!(Kind::of_code(17, 3), Some(masses));
    assert!(masses.family().painted() && !masses.family().drawn() && masses.family().composed());
    assert_eq!(Kind::of_code(18, 3), Some(Kind::Crop(Tier::Medium)));
    assert_eq!((Kind::of_code(18, 0), Kind::of_code(19, 2)), (None, None));
    assert_eq!(Metric::MassesMatched.code(), 55);
}

/// Shapes of every tier and compositions of the simple and medium tiers whose value masses read
/// (S96), but the last of each.
fn with_masses() -> Pictures {
    let mut compositions = pack(
        "test-compositions",
        PictureKind::Composition,
        &[Tier::Simple, Tier::Medium],
    );
    let masses = ValueMasses::checked([2, 2], vec![0, 1, 2, 1]).map(Arc::new);
    for item in &mut compositions.items {
        item.masses = masses.clone().filter(|_| !item.id.ends_with("-3"));
    }
    Pictures::of(&[
        pack("test-shapes", PictureKind::Shape, &Tier::ALL),
        compositions,
        pack("test-crops", PictureKind::Crop, &CROPS),
    ])
}

#[test]
fn value_masses_and_whole_copies_come_in_turn_after_the_division() {
    let all = with_masses();
    let met = [Kind::ValueMasses(Tier::Simple), Kind::Line(LineTier::Arc)];
    for number in 1..=8 {
        let plan = stage_four(number, 0x5EED_0096 + u64::from(number), &before(&met, &all));
        let warmed = plan.warm_up as usize;
        let warm_up = &plan.exercises[..warmed];
        assert!(warm_up.iter().all(|e| !e.kind.family().composed()));
        let last = &plan.exercises[warmed + 4];
        let (turn, medium) = ((number - 1) as usize, [Tier::Simple, Tier::Medium]);
        let tier = medium[(turn + 1) % 2];
        let expected = if number % 2 == 1 {
            Kind::ValueMasses(tier)
        } else {
            Kind::WholeCopy(tier)
        };
        assert_eq!(last.kind, expected, "session {number}");
        let item = last.picture.as_ref().expect("a composition");
        if number % 2 == 1 {
            assert!(!item.item.ends_with("-3"), "only masses that read");
        }
    }
}
