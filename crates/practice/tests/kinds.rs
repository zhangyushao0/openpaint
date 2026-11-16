//! Each kind's description (S109, S122, S123): stored codes read back, every topic described once
//! with a key of its own, what each topic's facts say of it, and every skill's lesson at every
//! level: laid, a few minutes long, each exercise generating, printing and compared, the levels
//! apart.

use std::sync::Arc;

use openpaint_practice::{
    Areas, Asked, BoxView, Carve, Combination, Copied, CurvedRotation, DrawnCopy, Estimate,
    Exercise, Facts, Fill, Guide, Horizon, ItemRef, Kind, Lean, Learner, Level, Lightness,
    LineTier, Lit, Mask, MaskShape, Metric, Pack, PackItem, Pen, PictureFit, PictureKind, Pictures,
    Placement, Point, Purity, Rgb8, Rotation, ScaleTone, SheetPicture, Skill, SkillLevel, Solid,
    Standing, Tendency, Throw, Tier, Tone, Trick, ValueMasses, schedule,
};

/// A kind of every variant the stores may hold.
fn every_kind() -> Vec<Kind> {
    let mut kinds: Vec<Kind> = LineTier::ALL.map(Kind::Line).to_vec();
    kinds.extend(Combination::ALL.map(Kind::Combined));
    kinds.extend(BoxView::ALL.map(Kind::Box));
    kinds.extend(Horizon::ALL.map(Kind::FreeBoxes));
    kinds.extend(Guide::ALL.map(Kind::Ellipse));
    kinds.extend(Estimate::ALL.map(Kind::Flat));
    kinds.extend(Trick::ALL.map(Kind::Space));
    kinds.extend(Solid::ALL.map(Kind::Solid));
    kinds.extend(Carve::ALL.map(Kind::Object));
    kinds.extend(Rotation::ALL.map(Kind::Rotation));
    for tier in Tier::ALL {
        let tiered = [Kind::Shape, Kind::Crop, Kind::Division, Kind::WholeCopy];
        kinds.extend(tiered.map(|kind| kind(tier)));
        kinds.push(Kind::ValueMasses(tier));
        kinds.push(Kind::Analysis(tier));
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
    kinds.push(Kind::Observation);
    kinds.extend(MaskShape::ALL.map(|shape| Kind::GamutMask(Mask { shape, hue: 60 })));
    kinds.extend(Placement::ALL.map(Kind::Placement));
    kinds.extend(Lit::ALL.map(Kind::Light));
    kinds.extend(Throw::ALL.map(Kind::Shadows));
    kinds.extend(CurvedRotation::ALL.map(Kind::CurvedRotation));
    kinds
}

#[test]
fn every_kind_reads_back_from_its_stored_code_and_codes_run_one_to_twenty_eight_but_five() {
    let mut codes = Vec::new();
    for kind in every_kind() {
        let (code, variant) = kind.code();
        assert_eq!(Kind::of_code(code, variant), Some(kind), "{kind:?}");
        codes.push(code);
    }
    codes.sort_unstable();
    codes.dedup();
    let expected: Vec<u8> = (1..=28).filter(|code| *code != 5).collect();
    assert_eq!(codes, expected);
    assert_eq!(Kind::of_code(5, 0), None, "code 5 is free");
    // The first lesson's free scribble (S122) is kind 20, its one variant 0.
    assert_eq!(Kind::Free.code(), (20, 0));
    // A drawing from life (S151) is kind 23; a gamut mask (S153) 24, its shape times 36 and its
    // hue's step; a placement (S153) 25, light on form 26, cast shadows 27, a curved rotation
    // (S154f) 28; no kind is 29.
    assert_eq!(Kind::Observation.code(), (23, 0));
    let small = Mask {
        shape: MaskShape::Small,
        hue: 60,
    };
    assert_eq!(Kind::GamutMask(small).code(), (24, 2 * 36 + 6));
    assert_eq!(Kind::of_code(24, 108), None);
    assert_eq!(Kind::of_code(29, 0), None);
    assert_eq!(Kind::Analysis(Tier::Hard).code(), (21, 4));
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
        (Metric::SpaceOff.code(), Metric::MassesMatched.code()),
        (46, 55)
    );
}

/// Every topic's facts, by their keys in order: what every kind's facts are.
fn topics() -> Vec<&'static Facts> {
    let mut topics: Vec<&Facts> = every_kind().into_iter().map(Kind::facts).collect();
    topics.sort_by_key(|facts| facts.key);
    topics.dedup_by_key(|facts| facts.key);
    topics
}

#[test]
fn every_topic_is_described_once_with_a_key_of_its_own() {
    for kind in every_kind() {
        let facts = kind.facts();
        let lowered = |c: char| c.is_ascii_lowercase() || c == '-';
        assert!(facts.key.chars().all(lowered), "{}", facts.key);
        let alike = every_kind()
            .into_iter()
            .filter(|k| k.facts().key == facts.key);
        assert!(alike.into_iter().all(|k| k.facts() == facts));
    }
    // Free boxes looking down and up are topics of their own (S142).
    assert_eq!(topics().len(), 39);
}

#[test]
fn a_topic_s_facts_say_what_it_is() {
    let aligned = ["line", "combined", "shape", "observation"];
    for facts in topics() {
        let key = facts.key;
        assert_eq!(facts.aligned, aligned.contains(&key), "{key}");
        assert_eq!(facts.compared, key != "free", "{key}");
        assert!(facts.seconds >= 30, "{key}: how long one takes");
        let tools = &facts.tools;
        let pens: &[Pen] = match key {
            "division" | "abstract-analysis" => &Pen::DIVIDING,
            "whole-copy" => &Pen::COPYING,
            "value-masses" | "light-halftone" => &Pen::VALUES,
            "light-families" => &Pen::TWO_TONES,
            "light-core" => &Pen::FOUR_TONES,
            "colour" | "scale" | "combination" | "key-shift" | "study" | "gamut-mask" => &[],
            _ => &Pen::DRAWING,
        };
        assert_eq!(tools.pens, pens, "{key}");
        let painted = ["whole-copy", "combination", "key-shift", "gamut-mask"];
        assert_eq!(tools.paints, painted.contains(&key), "{key}");
        let fill = match key {
            "scale" => Some(Fill::Cell),
            "colour" | "combination" | "key-shift" | "whole-copy" | "value-masses"
            | "gamut-mask" => Some(Fill::Frame),
            "light-families" | "light-halftone" | "light-core" => Some(Fill::Frame),
            _ => None,
        };
        assert_eq!(tools.fill, fill, "{key}");
        assert_eq!(
            (tools.full, tools.layered),
            (key == "study", key == "whole-copy")
        );
    }
    let constructed = topics().into_iter().filter(|facts| facts.constructed);
    // The eighteen built from given parts (light on form's three and cast shadows, S153), the free
    // scribble's one frame (S122) and abstract analysis's, its picture under the marks (S112).
    assert_eq!(constructed.count(), 20);
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

/// `exercise` generated, printed and compared with a copy held, its picture bare where it has one.
fn prints_and_compares(exercise: &Exercise) {
    let original = exercise.original();
    assert!(!original.givens().is_empty(), "{exercise:?}: givens");
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
            | Kind::ValueMasses(t)
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
