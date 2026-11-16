//! Every skill's lesson at every level (S122, S123): laid, a few minutes long, each exercise
//! generating, printing and compared, the levels apart; a skill's topics spread over its levels;
//! compositions and shapes harder by level and needing their pictures.

use std::sync::Arc;

use openpaint_practice::{
    Areas, Asked, Copied, DrawnCopy, Exercise, ItemRef, Kind, Learner, Level, MassesShown, Massing,
    Pack, PackItem, PictureFit, PictureKind, Pictures, Point, Rgb8, SheetPicture, Skill,
    SkillLevel, Standing, Tier, ValueMasses, schedule,
};

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

/// Shapes of every tier, compositions and crops of the three tiers they come in.
fn every_picture() -> Pictures {
    let composed = [Tier::Simple, Tier::Medium, Tier::Hard];
    Pictures::of(&[
        pack("test-shapes", PictureKind::Shape, &Tier::ALL),
        pack("test-compositions", PictureKind::Composition, &composed),
        pack("test-crops", PictureKind::Crop, &composed),
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
        DrawnCopy { all: lines() }
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

/// `exercise` generated, printed and compared with a copy held, its picture bare where it has one
/// but for its value masses (a contrast thumbnail prints from them, S154).
fn prints_and_compares(exercise: &Exercise) {
    let original = exercise.original();
    let masses = ValueMasses::checked([2, 2], vec![0, 1, 2, 1]).map(Arc::new);
    let picture = exercise.picture.clone().map(|item| SheetPicture {
        item,
        size: [400, 300],
        fit: PictureFit::of([400, 300]),
        outline: None,
        credit: None,
        masses: masses.map(|answer| MassesShown {
            edges: answer.edges(),
            answer,
        }),
        areas: None,
    });
    let printed = original.printed(picture.as_ref());
    assert!(!printed.is_empty(), "{exercise:?}: givens");
    let asked = Asked {
        exercise,
        original: &*original,
        picture: picture.as_ref(),
        copy: &Held,
        choices: openpaint_practice::Choices::default(),
    };
    assert!(original.compare(&asked).is_some(), "{exercise:?}: compared");
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
                .map(|(e, _)| u32::from(e.kind.seconds()))
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
        // Each level is another lesson but where the kind has nothing harder to offer, or offers
        // it in what its kind does not hold (a key shift's crop by its level, and its third
        // level's far tendency, which the second may draw too: S140).
        let key = skill.key();
        if key != "key-shift" {
            assert_ne!(levels[0], levels[1], "{key}: the second level");
        }
        if key != "key-shift" {
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
    let topics = |key: &str, level| -> Vec<&str> {
        let skill = Skill::of_key(key).expect("a skill");
        let planned = schedule::lesson(skill, level, 3, &standing).expect("a lesson");
        let mut topics: Vec<&str> = (planned.exercises.iter())
            .map(|(e, _)| e.kind.facts().key)
            .collect();
        topics.sort_unstable();
        topics.dedup();
        topics
    };
    let (first, second, third) = (SkillLevel::First, SkillLevel::Second, SkillLevel::Third);
    assert_eq!(topics("estimates", first), ["point"]);
    assert_eq!(topics("estimates", second), ["point", "proportion"]);
    assert_eq!(topics("estimates", third), ["angle", "point", "proportion"]);
    assert_eq!(topics("points-in-space", first), ["centre", "halves"]);
    assert_eq!(topics("points-in-space", second), ["next-face", "thirds"]);
    assert!(topics("points-in-space", third).contains(&"mirror"));
    assert_eq!(topics("solids", first), ["cylinder"]);
    assert_eq!(topics("solids", second), ["cone"]);
    assert!(topics("solids", third).contains(&"sphere"));
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
            Kind::Shape(t)
            | Kind::Division(t)
            | Kind::WholeCopy(t)
            | Kind::ValueMasses(Massing::Three(t) | Massing::Two(t))
            | Kind::Analysis(t) => t,
            other => panic!("{other:?}"),
        };
        (tier, exercise.picture.clone())
    };
    let composed = [
        "division",
        "whole-copy",
        "value-masses",
        "abstract-analysis",
    ];
    for key in std::iter::once("shape").chain(composed) {
        assert_eq!(tier(key, SkillLevel::Second).0, Tier::Medium, "{key}");
        assert_eq!(tier(key, SkillLevel::Third).0, Tier::Hard, "{key}");
        assert!(
            tier(key, SkillLevel::First).1.is_some(),
            "{key}: its picture"
        );
    }
    assert_eq!(tier("shape", SkillLevel::First).0, Tier::Entry);
    assert_eq!(tier("division", SkillLevel::First).0, Tier::Simple);
    assert_eq!(tier("abstract-analysis", SkillLevel::First).0, Tier::Simple);
    let none = Pictures::default();
    let bare = Standing {
        pictures: &none,
        learner: &learner,
    };
    for key in ["shape", "combination", "key-shift", "gamut-mask"]
        .into_iter()
        .chain(composed)
    {
        let skill = Skill::of_key(key).expect("a skill");
        assert!(skill.lacks(&none).is_some(), "{key}");
        assert!(
            schedule::lesson(skill, SkillLevel::First, 5, &bare).is_none(),
            "{key}"
        );
    }
}
