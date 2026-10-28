//! Drills (S91): one kind on its own sheet, the counts the slice gives each kind, the variants in
//! turn by the drill's number, shapes from the pictures there are (none without), and a drill's
//! record: the kinds met and its time and lines count, never as a session, on the road or in the
//! first picks.

use openpaint_practice::{
    Before, Book, DRILL, ExerciseResult, Family, ItemRef, Kind, Metric, Pack, PackItem,
    PictureKind, Pictures, STAGES, STAGES_READY, Seal, SessionRecord, Span, Tally, Tier, drill,
    kinds_of, met, sealed,
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
    Pictures::of(&[Some(pack), mine].into_iter().flatten().collect::<Vec<_>>())
}

fn before<'a>(pictures: &'a Pictures, used: &'a [ItemRef]) -> Before<'a> {
    Before {
        met: &[],
        pictures,
        used,
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
}

#[test]
fn variants_come_in_turn_by_the_drill_s_number() {
    let none = pictures(&[], None);
    let combinations: Vec<Kind> = (1..=3)
        .flat_map(|n| kinds(Family::Areas, n, &none))
        .collect();
    for arrangement in openpaint_practice::Arrangement::ALL {
        let held = combinations
            .iter()
            .filter(|k| **k == Kind::Areas(arrangement));
        assert_eq!(held.count(), 2, "{arrangement:?}");
    }
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

/// Drill `id` of `family`, its first exercise compared at `at`, practised from `at` for `minutes`.
fn drilled(id: u64, family: Family, at: u64, minutes: u64) -> SessionRecord {
    let none = pictures(&[], None);
    let plan = drill(family, 1, id, &before(&none, &[])).expect("a drill");
    let mut record = SessionRecord::drill(id, 1, (id, at), plan);
    record.canvas = 40 + id;
    record.results.push(ExerciseResult::first(
        0,
        at + 60,
        9,
        vec![(Metric::FirstDelta, 0.08)],
    ));
    record.time = vec![Span {
        from: at,
        to: at + minutes * 60,
    }];
    record.finished = Some(at + minutes * 60);
    record
}

#[test]
fn a_drill_counts_its_time_kinds_and_lines_never_as_a_session() {
    let line = drilled(1, Family::Line, NOON, 10);
    let colour = drilled(2, Family::Colour, NOON + 3600, 7);
    assert!(line.is_drill() && !line.is_session() && !line.is_project());
    assert_eq!((line.stage, line.drilled()), (DRILL, Some(Family::Line)));
    let session = SessionRecord::stage_one(3, 1, 9, NOON, &[]);
    assert!(session.is_session() && session.drilled().is_none());
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
