//! Drills (S91): one kind on its own sheet, the counts the slice gives each kind (stage 4's objects
//! among them, S94), the variants in turn by the drill's number, shapes from the pictures there are
//! (none without), and a drill's record: the kinds met and its time and lines count, never as a
//! session, on the road or in the first picks.

use std::sync::Arc;

use openpaint_practice::{
    Areas, Before, Book, Family, First, ItemRef, Kind, Metric, Pack, PackItem, Picked, PictureKind,
    Pictures, STAGES, STAGES_READY, Seal, Sheet, Span, Tally, Tier, ValueMasses, drill, kinds_of,
    met, sealed, stage_one,
};

/// 2026-10-05 12:00 UTC, a Monday.
const NOON: u64 = 1_791_201_600;

fn pictures(tiers: &[Tier], personal: Option<Tier>) -> Pictures {
    let item = |id: String, tier: Tier| PackItem {
        id,
        kind: PictureKind::Shape,
        tier,
        size: [800, 600],
        title: "test".to_owned(),
        credit: None,
        outline: None,
        masses: None,
        areas: None,
    };
    let pack = Pack {
        id: "test-shapes".to_owned(),
        title: "Test shapes".to_owned(),
        items: (tiers.iter())
            .flat_map(|tier| (0..4).map(move |n| item(format!("{}-{n}", tier.name()), *tier)))
            .collect(),
    };
    let mine = personal.map(|tier| Pack {
        id: openpaint_practice::PERSONAL.to_owned(),
        title: "My pictures".to_owned(),
        items: vec![item("mine-1".to_owned(), tier)],
    });
    // Compositions (S95) and crops (S101) only where there are shapes: the tests without pictures
    // have none.
    let compositions = (!tiers.is_empty()).then(|| compositions(&[Tier::Simple, Tier::Medium]));
    let crops = (!tiers.is_empty()).then(|| crops(4));
    let packs: Vec<Pack> = [Some(pack), mine, compositions, crops]
        .into_iter()
        .flatten()
        .collect();
    Pictures::of(&packs)
}

/// A composition pack of four pictures of each of `tiers`, the last of each whose value masses do
/// not read (S96).
fn compositions(tiers: &[Tier]) -> Pack {
    let masses = ValueMasses::checked([2, 2], vec![0, 1, 2, 1]).map(Arc::new);
    let item = |id: String, tier: Tier| PackItem {
        masses: masses.clone().filter(|_| !id.ends_with("-3")),
        id,
        kind: PictureKind::Composition,
        tier,
        size: [800, 600],
        title: "test".to_owned(),
        credit: None,
        outline: None,
        areas: None,
    };
    Pack {
        id: "test-compositions".to_owned(),
        title: "Test compositions".to_owned(),
        items: (tiers.iter())
            .flat_map(|tier| (0..4).map(move |n| item(format!("{}-{n}", tier.name()), *tier)))
            .collect(),
    }
}

/// A pack of `count` crops (S101), each of three areas.
fn crops(count: usize) -> Pack {
    let areas = Areas::of_map(
        [2, 2],
        vec![0, 1, 2, 0],
        vec![[90, 60, 40], [96, 66, 44], [200; 3]],
    );
    let item = |n: usize| PackItem {
        id: format!("crop-{n}"),
        kind: PictureKind::Crop,
        tier: Tier::Simple,
        size: [600, 600],
        title: "test".to_owned(),
        credit: None,
        outline: None,
        masses: None,
        areas: areas.clone().map(Arc::new),
    };
    Pack {
        id: "test-crops".to_owned(),
        title: "Test crops".to_owned(),
        items: (0..count).map(item).collect(),
    }
}

fn before<'a>(pictures: &'a Pictures, used: &'a [ItemRef]) -> Before<'a> {
    Before {
        met: &[],
        pictures,
        used,
        recent: used,
        scales: 0,
    }
}

fn kinds(family: Family, number: u32, pictures: &Pictures) -> Vec<Kind> {
    let plan = drill(family, number, 0x5EED_0091, &before(pictures, &[])).expect("a drill");
    assert_eq!(plan.warm_up, 0, "{family:?}: no warm-up");
    plan.exercises.iter().map(|e| e.kind).collect()
}

#[test]
fn each_kind_s_drill_holds_that_kind_only_as_many_as_the_slice_says() {
    let all = pictures(&Tier::ALL, None);
    let counts = [
        (Family::Line, 6),
        (Family::Combined, 4),
        (Family::Colour, 6),
        (Family::Shape, 3),
        (Family::Areas, 2),
        (Family::Scale, 2),
        (Family::Box, 3),
        (Family::FreeBoxes, 1),
        (Family::Ellipse, 4),
        (Family::Point, 5),
        (Family::Proportion, 5),
        (Family::Angle, 5),
        (Family::Centre, 3),
        (Family::Halves, 3),
        (Family::Thirds, 3),
        (Family::Step, 3),
        (Family::Mirror, 3),
        (Family::Cylinder, 3),
        (Family::Cone, 3),
        (Family::Sphere, 3),
        (Family::Object, 3),
        (Family::Division, 3),
        (Family::ValueMasses, 3),
        (Family::WholeCopy, 1),
        (Family::KeyShift, 2),
    ];
    for (family, count) in counts {
        let drilled = kinds(family, 1, &all);
        assert_eq!(drilled.len(), count, "{family:?}");
        assert!(
            drilled.iter().all(|kind| kind.family() == family),
            "{family:?}"
        );
        let again = drill(family, 1, 0x5EED_0091, &before(&all, &[]));
        let plan = drill(family, 1, 0x5EED_0091, &before(&all, &[]));
        assert_eq!(again, plan, "{family:?}: the same per seed");
    }
    assert_eq!(drill(Family::Study, 1, 1, &before(&all, &[])), None);
    // Every kind drilled lies on exactly one ready stage's tiles; stages not ready hold none.
    let tiles: Vec<Family> = (1..=STAGES_READY)
        .flat_map(|s| kinds_of(s).iter().copied())
        .collect();
    assert_eq!(tiles, counts.map(|(family, _)| family));
    assert!((STAGES_READY + 1..=STAGES).all(|stage| kinds_of(stage).is_empty()));
    // Value masses only of pictures whose masses read (S96), in four drills of three.
    for number in 1..=4 {
        let plan = drill(Family::ValueMasses, number, 7, &before(&all, &[])).expect("masses");
        let items = plan.exercises.iter().filter_map(|e| e.picture.as_ref());
        assert!(items.clone().count() == 3 && items.into_iter().all(|i| !i.item.ends_with("-3")));
    }
}

#[test]
fn variants_come_in_turn_by_the_drill_s_number() {
    let none = pictures(&[], None);
    let horizons: Vec<Kind> = (1..=3)
        .flat_map(|n| kinds(Family::FreeBoxes, n, &none))
        .collect();
    let mut distinct = horizons.clone();
    distinct.dedup();
    assert_eq!(distinct.len(), 3, "a horizon each in turn: {horizons:?}");
    let first = kinds(Family::Ellipse, 1, &none);
    assert!(
        first.windows(2).all(|pair| pair[0] != pair[1]),
        "frame, then plane"
    );
    // Objects' carves in turn: four drills of three show each carve three times.
    let objects: Vec<Kind> = (1..=4)
        .flat_map(|n| kinds(Family::Object, n, &none))
        .collect();
    for carve in openpaint_practice::Carve::ALL {
        let held = objects.iter().filter(|k| **k == Kind::Object(carve));
        assert_eq!(held.count(), 3, "{carve:?}");
    }
}

#[test]
fn a_shapes_drill_takes_its_tiers_in_turn_the_user_s_own_last_and_none_without_pictures() {
    let all = pictures(&Tier::ALL, None);
    let tiers = |number: u32, pictures: &Pictures| -> Vec<Tier> {
        (kinds(Family::Shape, number, pictures).into_iter())
            .filter_map(|kind| match kind {
                Kind::Shape(tier) => Some(tier),
                _ => None,
            })
            .collect()
    };
    assert_eq!(tiers(1, &all), [Tier::Entry, Tier::Simple, Tier::Medium]);
    assert_eq!(tiers(2, &all), [Tier::Entry, Tier::Simple, Tier::Hard]);
    let seen: Vec<Tier> = (1..=4).flat_map(|n| tiers(n, &all)).collect();
    for tier in Tier::ALL {
        assert_eq!(seen.iter().filter(|t| **t == tier).count(), 3, "{tier:?}");
    }
    let plan = drill(Family::Shape, 1, 5, &before(&all, &[])).expect("shapes");
    assert!(plan.exercises.iter().all(|e| e.picture.is_some()));
    // The user's own of the drill's hardest tier or an easier one comes last.
    let mine = pictures(&Tier::ALL, Some(Tier::Simple));
    let plan = drill(Family::Shape, 1, 5, &before(&mine, &[])).expect("shapes");
    let last = plan.exercises.last().and_then(|e| e.picture.as_ref());
    assert_eq!(last.map(|p| p.item.as_str()), Some("mine-1"));
    // No picture: no shapes drill.
    let none = pictures(&[], None);
    assert_eq!(drill(Family::Shape, 1, 5, &before(&none, &[])), None);
}

#[test]
fn a_combinations_drill_takes_crops_not_shown_before_and_none_without_them() {
    let all = pictures(&Tier::ALL, None);
    let crop = |n: usize| ItemRef {
        pack: "test-crops".to_owned(),
        item: format!("crop-{n}"),
    };
    let plan = drill(Family::Areas, 1, 5, &before(&all, &[])).expect("combinations");
    let items: Vec<&ItemRef> = plan
        .exercises
        .iter()
        .filter_map(|e| e.picture.as_ref())
        .collect();
    assert_eq!(items.len(), 2);
    assert_ne!(items[0], items[1], "two crops, none twice");
    assert!(
        plan.exercises
            .iter()
            .all(|e| e.kind == Kind::Crop(Tier::Simple))
    );
    // Three of the four shown before: the one left comes first, then any.
    let used = [crop(0), crop(1), crop(3)];
    let plan = drill(Family::Areas, 2, 5, &before(&all, &used)).expect("combinations");
    assert_eq!(plan.exercises[0].picture, Some(crop(2)));
    // No crop: no combinations drill.
    let none = pictures(&[], None);
    assert_eq!(drill(Family::Areas, 1, 5, &before(&none, &[])), None);
}

/// Drill `id` of `family`, its first exercise compared at `at`, practised from `at` for `minutes`.
fn drilled(id: u64, family: Family, at: u64, minutes: u64) -> Sheet {
    let none = pictures(&[], None);
    let plan = drill(family, 1, id, &before(&none, &[])).expect("a drill");
    let mut record = Sheet::drill(id, at, plan);
    record.canvas = 40 + id;
    let first = First::of(at + 60, 9, vec![(Metric::FirstDelta, 0.08)]);
    record.laid_mut(0).expect("its first").compared(first);
    record.spans = vec![Span {
        from: at,
        to: at + minutes * 60,
    }];
    record.lessons[0].finished = Some(at + minutes * 60);
    record
}

#[test]
fn a_drill_counts_its_time_kinds_and_lines_never_as_a_session() {
    let line = drilled(1, Family::Line, NOON, 10);
    let colour = drilled(2, Family::Colour, NOON + 3600, 7);
    assert!(line.stage().is_none() && !line.is_copy());
    let picked = line.lesson().map(|lesson| lesson.picked);
    assert_eq!(
        (picked, line.drilled()),
        (Some(Picked::Chosen), Some(Family::Line))
    );
    let session = Sheet::session(3, 1, NOON, stage_one(9, &[]));
    assert!(session.stage().is_some() && session.drilled().is_none());
    // The kinds compared in a drill are met: a later warm-up draws on them.
    let records = [line.clone(), colour.clone()];
    let families: Vec<Family> = met(&records).iter().map(|kind| kind.family()).collect();
    assert_eq!(families, [Family::Line, Family::Colour]);
    // Finished drills are no sessions: no first step, no stage; their lines and time count.
    let tally = Tally::of(&records);
    assert_eq!((tally.sessions, tally.stages), (0, [0; STAGES as usize]));
    assert_eq!((tally.lines, tally.colours, tally.seconds), (1, 1, 17 * 60));
    assert!(!sealed(&Tally::default(), &line).contains(&Seal::FirstSession));
    // The book: the time and the day, never the road, the sessions or the first picks.
    let book = Book::of(&records, 1, 8 * 60);
    assert_eq!((book.sessions, book.seconds), (0, 17 * 60));
    assert_eq!(book.road.done, [0; STAGES as usize]);
    assert!(book.colour.is_empty() && book.lines.is_empty());
    assert_eq!(book.days.len(), 1);
    assert_eq!(book.days[0].records, [1, 2]);
}
