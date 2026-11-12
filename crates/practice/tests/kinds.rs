//! Each kind's description (S109, S122): stored codes read back, every family described once with
//! a key of its own, what each family's facts say of it, and every skill's lesson at every level:
//! laid, a few minutes long, each exercise generating, printing and compared, the levels apart.

use std::sync::Arc;

use openpaint_practice::{
    Areas, Arrangement, Asked, BoxView, Carve, Combination, Copied, DrawnCopy, Estimate, Exercise,
    Family, Guide, Horizon, ItemRef, Kind, Lean, Learner, Level, Lightness, LineTier, Metric, Pack,
    PackItem, PictureFit, PictureKind, Pictures, Point, Purity, Rgb8, ScaleTone, SheetPicture,
    Skill, SkillLevel, Solid, Standing, Tendency, Tier, Tone, Tools, Trick, ValueMasses, schedule,
};

/// A kind of every variant the stores may hold.
fn every_kind() -> Vec<Kind> {
    let mut kinds: Vec<Kind> = LineTier::ALL.map(Kind::Line).to_vec();
    kinds.extend(Combination::ALL.map(Kind::Combined));
    kinds.extend(Arrangement::ALL.map(Kind::Areas));
    kinds.extend(BoxView::ALL.map(Kind::Box));
    kinds.extend(Horizon::ALL.map(Kind::FreeBoxes));
    kinds.extend(Guide::ALL.map(Kind::Ellipse));
    kinds.extend(Estimate::ALL.map(Kind::Flat));
    kinds.extend(Trick::ALL.map(Kind::Space));
    kinds.extend(Solid::ALL.map(Kind::Solid));
    kinds.extend(Carve::ALL.map(Kind::Object));
    for tier in Tier::ALL {
        let tiered = [Kind::Shape, Kind::Crop, Kind::Division, Kind::WholeCopy];
        kinds.extend(tiered.map(|kind| kind(tier)));
        kinds.push(Kind::ValueMasses(tier));
    }
    for tone in ScaleTone::ALL {
        kinds.extend([5, 7, 9].map(|cells| Kind::Scale(tone, cells)));
    }
    kinds.push(Kind::Colour(Tone {
        purity: Purity::Muted,
        lightness: Lightness::Middle,
    }));
    kinds.push(Kind::KeyShift(Tendency {
        lean: Lean::WarmPure,
        hue: 30,
    }));
    kinds.push(Kind::Study);
    kinds.push(Kind::Free);
    kinds
}

#[test]
fn every_kind_reads_back_from_its_stored_code_and_codes_run_one_to_twenty() {
    let mut codes = Vec::new();
    for kind in every_kind() {
        let (code, variant) = kind.code();
        assert_eq!(Kind::of_code(code, variant), Some(kind), "{kind:?}");
        codes.push(code);
    }
    codes.sort_unstable();
    codes.dedup();
    assert_eq!(codes, (1..=20).collect::<Vec<u8>>());
    // The first lesson's free scribble (S122) is kind 20, its one variant 0.
    assert_eq!(Kind::Free.code(), (20, 0));
    assert_eq!(Kind::of_code(21, 0), None);
    // Codes pinned for good: points in space, estimates, objects, compositions, crops.
    assert_eq!(Kind::Space(Trick::Mirror).code(), (12, 4));
    assert_eq!(Kind::Flat(Estimate::Angle).code(), (11, 2));
    assert_eq!(Kind::Object(Carve::Round).code(), (14, 3));
    assert_eq!(Kind::Division(Tier::Simple).code(), (15, 2));
    assert_eq!(Kind::WholeCopy(Tier::Medium).code(), (16, 3));
    assert_eq!(Kind::ValueMasses(Tier::Medium).code(), (17, 3));
    assert_eq!(Kind::of_code(18, 3), Some(Kind::Crop(Tier::Medium)));
    assert_eq!((Kind::of_code(18, 0), Kind::of_code(20, 2)), (None, None));
    assert_eq!(
        (Metric::ObjectOff.code(), Metric::MassesMatched.code()),
        (48, 55)
    );
}

#[test]
fn every_family_is_described_once_with_a_key_of_its_own() {
    let mut keys = Vec::new();
    for family in Family::ALL {
        let facts = family.facts();
        assert_eq!(facts.family, family);
        assert_eq!(Family::of_key(facts.key), Some(family));
        let lowered = |c: char| c.is_ascii_lowercase() || c == '-';
        assert!(facts.key.chars().all(lowered), "{}", facts.key);
        keys.push(facts.key);
    }
    keys.sort_unstable();
    keys.dedup();
    assert_eq!(keys.len(), Family::ALL.len());
    // The retired generated combinations go by the combinations' family.
    assert_eq!(Kind::Areas(Arrangement::ALL[0]).family(), Family::Areas);
}

#[test]
fn a_family_s_facts_say_what_it_is() {
    let aligned = [Family::Line, Family::Combined, Family::Shape];
    let composed = [Family::Division, Family::WholeCopy, Family::ValueMasses];
    for family in Family::ALL {
        let facts = family.facts();
        assert_eq!(facts.aligned, aligned.contains(&family), "{family:?}");
        assert_eq!(facts.compared, family != Family::Free, "{family:?}");
        assert_eq!(family.composed(), composed.contains(&family), "{family:?}");
        assert!(facts.seconds >= 30, "{family:?}: how long one takes");
        let tools = match family {
            Family::Division => Tools::Dividing,
            Family::WholeCopy => Tools::Copying,
            Family::ValueMasses => Tools::Greys,
            Family::Colour => Tools::Colour,
            Family::Scale => Tools::Cells,
            Family::Areas | Family::KeyShift => Tools::Painting,
            Family::Study => Tools::Full,
            _ => Tools::Drawing,
        };
        assert_eq!(facts.tools, tools, "{family:?}");
    }
    let constructed: Vec<Family> = (Family::ALL.into_iter())
        .filter(|f| f.constructed())
        .collect();
    // The twelve built from given parts, and the free scribble's one frame (S122).
    assert_eq!(constructed.len(), 13, "{constructed:?}");
}

/// A pack of `kind` pictures, four of each of `tiers`, with value masses that read and areas.
fn pack(id: &str, kind: PictureKind, tiers: &[Tier]) -> Pack {
    let masses = ValueMasses::checked([2, 2], vec![0, 1, 2, 1]).map(Arc::new);
    let areas = Areas::of_map(
        [2, 2],
        vec![0, 1, 2, 0],
        vec![[160, 80, 40], [90, 96, 110], [220; 3]],
    );
    let item = |n: usize, tier: Tier| PackItem {
        id: format!("{}-{n}", tier.name()),
        kind,
        tier,
        size: [800, 600],
        title: "test".to_owned(),
        credit: None,
        outline: None,
        masses: masses.clone().filter(|_| kind == PictureKind::Composition),
        areas: areas
            .clone()
            .filter(|_| kind == PictureKind::Crop)
            .map(Arc::new),
    };
    Pack {
        id: id.to_owned(),
        title: "Test".to_owned(),
        items: (tiers.iter())
            .flat_map(|tier| (0..4).map(move |n| item(n, *tier)))
            .collect(),
    }
}

/// Shapes of every tier, compositions of the three tiers they come in, crops.
fn every_picture() -> Pictures {
    let composed = [Tier::Simple, Tier::Medium, Tier::Hard];
    Pictures::of(&[
        pack("test-shapes", PictureKind::Shape, &Tier::ALL),
        pack("test-compositions", PictureKind::Composition, &composed),
        pack("test-crops", PictureKind::Crop, &[Tier::Simple]),
    ])
}

/// A copy with something in it: lines across the frame and a colour laid.
struct Held;

impl Copied for Held {
    fn holds(&self) -> bool {
        true
    }

    fn picks(&self) -> Vec<Rgb8> {
        vec![[120, 90, 60]]
    }

    fn drawn(&self) -> DrawnCopy {
        DrawnCopy {
            all: lines(),
            corrected: None,
        }
    }

    fn division(&self) -> Vec<(Level, Vec<Point>)> {
        lines()
            .into_iter()
            .map(|line| (Level::Main, line))
            .collect()
    }
}

fn lines() -> Vec<Vec<Point>> {
    vec![
        vec![[0.2, 0.3], [0.8, 0.35]],
        vec![[0.5, 0.2], [0.45, 0.8]],
        vec![[0.3, 0.7], [0.6, 0.6], [0.7, 0.4]],
    ]
}

/// `exercise` generated, printed and compared with a copy held, its picture bare where it has one.
fn prints_and_compares(exercise: &Exercise) {
    let original = exercise.original();
    assert!(
        !exercise.givens(&original).is_empty(),
        "{exercise:?}: givens"
    );
    let picture = exercise.picture.clone().map(|item| SheetPicture {
        item,
        size: [400, 300],
        fit: PictureFit::of([400, 300]),
        outline: None,
        credit: None,
        masses: None,
        areas: None,
    });
    let asked = Asked {
        exercise,
        original: &original,
        picture: picture.as_ref(),
        copy: &Held,
    };
    assert!(exercise.compare(&asked).is_some(), "{exercise:?}: compared");
}

#[test]
fn every_skill_at_every_level_lays_a_few_minutes_that_generate_print_and_compare() {
    let (pictures, learner) = (every_picture(), Learner::default());
    let standing = Standing {
        pictures: &pictures,
        learner: &learner,
    };
    for skill in Skill::all() {
        let mut levels: Vec<Vec<Kind>> = Vec::new();
        for level in SkillLevel::ALL {
            let planned = schedule::lesson(skill, level, 0x5EED_0122, &standing);
            let planned = planned.unwrap_or_else(|| panic!("{}: {level:?}", skill.key()));
            let held = planned.exercises.len();
            assert!((1..=6).contains(&held), "{}: {level:?} {held}", skill.key());
            let seconds: u32 = (planned.exercises.iter())
                .map(|(e, _)| u32::from(e.kind.facts().seconds))
                .sum();
            assert!(
                (180..=480).contains(&seconds),
                "{}: {level:?} {seconds} s",
                skill.key()
            );
            for (exercise, laid) in &planned.exercises {
                assert_eq!((Skill::of(exercise.kind), *laid), (Some(skill), level));
                prints_and_compares(exercise);
            }
            levels.push(planned.exercises.iter().map(|(e, _)| e.kind).collect());
        }
        // A lesson of one exercise fills its window over three; others over six.
        let window = if levels[0].len() == 1 { 3 } else { 6 };
        assert_eq!(skill.window(), window, "{}", skill.key());
        // Each level is another lesson but where the kind has nothing harder to offer.
        let key = skill.key();
        if key != "combination" {
            assert_ne!(levels[0], levels[1], "{key}: the second level");
        }
        if !["combination", "box"].contains(&key) {
            assert_ne!(levels[1], levels[2], "{key}: the third level");
        }
    }
}

#[test]
fn a_skill_of_several_topics_spreads_them_over_its_levels_in_the_course_s_order() {
    let (pictures, learner) = (Pictures::default(), Learner::default());
    let standing = Standing {
        pictures: &pictures,
        learner: &learner,
    };
    let families = |key: &str, level| -> Vec<Family> {
        let skill = Skill::of_key(key).expect("a skill");
        let planned = schedule::lesson(skill, level, 3, &standing).expect("a lesson");
        let mut families: Vec<Family> = planned
            .exercises
            .iter()
            .map(|(e, _)| e.kind.family())
            .collect();
        families.sort();
        families.dedup();
        families
    };
    let (first, second, third) = (SkillLevel::First, SkillLevel::Second, SkillLevel::Third);
    assert_eq!(families("estimates", first), [Family::Point]);
    assert_eq!(
        families("estimates", second),
        [Family::Point, Family::Proportion]
    );
    assert_eq!(
        families("estimates", third),
        [Family::Point, Family::Proportion, Family::Angle]
    );
    assert_eq!(
        families("points-in-space", first),
        [Family::Centre, Family::Halves]
    );
    assert_eq!(
        families("points-in-space", second),
        [Family::Thirds, Family::Step]
    );
    assert!(families("points-in-space", third).contains(&Family::Mirror));
    assert_eq!(families("solids", first), [Family::Cylinder]);
    assert_eq!(families("solids", second), [Family::Cone]);
    assert!(families("solids", third).contains(&Family::Sphere));
}

#[test]
fn compositions_and_shapes_get_harder_with_the_level_and_need_their_pictures() {
    let (pictures, learner) = (every_picture(), Learner::default());
    let standing = Standing {
        pictures: &pictures,
        learner: &learner,
    };
    let tier = |key: &str, level| -> (Tier, Option<ItemRef>) {
        let skill = Skill::of_key(key).expect("a skill");
        let planned = schedule::lesson(skill, level, 5, &standing).expect("a lesson");
        let (exercise, _) = &planned.exercises[0];
        let tier = match exercise.kind {
            Kind::Shape(t) | Kind::Division(t) | Kind::WholeCopy(t) | Kind::ValueMasses(t) => t,
            other => panic!("{other:?}"),
        };
        (tier, exercise.picture.clone())
    };
    for key in ["shape", "division", "whole-copy", "value-masses"] {
        assert_eq!(tier(key, SkillLevel::Second).0, Tier::Medium, "{key}");
        assert_eq!(tier(key, SkillLevel::Third).0, Tier::Hard, "{key}");
        assert!(
            tier(key, SkillLevel::First).1.is_some(),
            "{key}: its picture"
        );
    }
    assert_eq!(tier("shape", SkillLevel::First).0, Tier::Entry);
    assert_eq!(tier("division", SkillLevel::First).0, Tier::Simple);
    let none = Pictures::default();
    let bare = Standing {
        pictures: &none,
        learner: &learner,
    };
    for key in [
        "shape",
        "combination",
        "division",
        "whole-copy",
        "value-masses",
        "key-shift",
    ] {
        let skill = Skill::of_key(key).expect("a skill");
        assert!(skill.lacks(&none).is_some(), "{key}");
        assert!(
            schedule::lesson(skill, SkillLevel::First, 5, &bare).is_none(),
            "{key}"
        );
    }
}
