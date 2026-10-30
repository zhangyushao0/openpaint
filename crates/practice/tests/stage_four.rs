//! Stage 4 (S94): sessions of three objects simplified into boxes (their carves in turn), a solid
//! in its box in turn, a shape of the medium tier then the hard one in turn (none without a pack),
//! a colour combination in turn, after a warm-up of three from the kinds met, never an object or
//! a solid.

use openpaint_practice::{
    Arrangement, Before, Carve, Family, Kind, LineTier, Metric, Pack, PackItem, PictureKind,
    Pictures, STAGES_READY, Solid, Tier, WARM_UP_TWO, kinds_of, plan, stage_four,
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

fn before<'a>(met: &'a [Kind], pictures: &'a Pictures) -> Before<'a> {
    Before {
        met,
        pictures,
        used: &[],
    }
}

fn kinds(number: u32, met: &[Kind], pictures: &Pictures) -> Vec<Kind> {
    let plan = stage_four(number, 0x5EED_0094, &before(met, pictures));
    plan.exercises.iter().map(|e| e.kind).collect()
}

#[test]
fn a_first_stage_four_session_is_three_objects_a_solid_a_shape_and_a_combination() {
    assert_eq!(STAGES_READY, 4);
    assert_eq!(kinds_of(4), [Family::Object]);
    let all = pictures(&Tier::ALL);
    let first = stage_four(1, 0x5EED_0094, &before(&[], &all));
    assert_eq!(first, plan(4, 1, 0x5EED_0094, &before(&[], &all)));
    assert_eq!(first.warm_up, 0, "nothing met");
    let held: Vec<Kind> = first.exercises.iter().map(|e| e.kind).collect();
    assert_eq!(
        held,
        [
            Kind::Object(Carve::Block),
            Kind::Object(Carve::Cut),
            Kind::Object(Carve::Taper),
            Kind::Solid(Solid::Cylinder),
            Kind::Shape(Tier::Medium),
            Kind::Areas(Arrangement::ALL[0]),
        ]
    );
    assert!(first.exercises[4].picture.is_some(), "the shape's picture");
    let none = pictures(&[]);
    let bare = kinds(1, &[], &none);
    assert_eq!(bare.len(), 5, "no shape without a picture");
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
        let arrangement = Arrangement::ALL[index % Arrangement::ALL.len()];
        assert!(session.contains(&Kind::Areas(arrangement)));
        let objects = session.iter().filter(|k| k.family() == Family::Object);
        assert_eq!(objects.count(), 3, "session {}", index + 1);
    }
}

#[test]
fn the_warm_up_is_three_of_the_kinds_met_never_an_object_or_a_solid() {
    let all = pictures(&Tier::ALL);
    let met = [
        Kind::Line(LineTier::Arc),
        Kind::Object(Carve::Cut),
        Kind::Solid(Solid::Cone),
        Kind::Shape(Tier::Medium),
        Kind::Areas(Arrangement::ALL[1]),
    ];
    for number in 1..=6 {
        let plan = stage_four(number, 0x5EED_0094 + u64::from(number), &before(&met, &all));
        assert_eq!(plan.warm_up as usize, WARM_UP_TWO, "session {number}");
        let warm_up = &plan.exercises[..WARM_UP_TWO];
        assert!(warm_up.iter().all(|e| {
            let family = e.kind.family();
            family != Family::Object && !matches!(family, Family::Cylinder | Family::Cone)
        }));
        let long = (warm_up.iter())
            .filter(|e| matches!(e.kind.family(), Family::Shape | Family::Areas))
            .count();
        assert!(long <= 1, "session {number}: at most one long");
    }
    // An object's first error is kept by its own code.
    assert_eq!(Metric::ObjectOff.code(), 48);
    assert_eq!(Metric::of_code(48), Some(Metric::ObjectOff));
    let kind = Kind::Object(Carve::Round);
    assert_eq!(kind.code(), (14, 3));
    assert_eq!(Kind::of_code(14, 3), Some(kind));
}
