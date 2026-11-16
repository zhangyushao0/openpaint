//! How a lesson's pictures turn (S122): a single shape none twice in its tier while another is
//! left, the user's own every second lesson from the second level on, never a photo set aside
//! (S158); a composition others than
//! the recent lessons', so a tier's every picture (an upright one too) comes in turn; a review of a
//! kind whose lesson holds one exercise is its redo alone.

use openpaint_practice::{
    ItemRef, Kind, Learner, Outcome, PERSONAL, Pack, PackItem, PictureKind, Pictures, Planned,
    SkillLevel, Standing, Tier, schedule,
};

use crate::{ended, skill};

/// A pack `id` of `kind` pictures, each `(item, tier)`.
fn pack(id: &str, kind: PictureKind, items: &[(&str, Tier)]) -> Pack {
    let item = |(name, tier): &(&str, Tier)| PackItem {
        id: (*name).to_owned(),
        kind,
        tier: *tier,
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
        items: items.iter().map(item).collect(),
    }
}

/// The lesson of the skill keyed `key` at `level`, seeded the same every time (as the scenarios
/// seed them), for `learner` with `pictures`.
fn laid(learner: &Learner, pictures: &Pictures, key: &str, level: SkillLevel) -> Planned {
    let standing = Standing { pictures, learner };
    schedule::lesson(skill(key), level, 7, &standing).expect("a lesson")
}

/// The pictures of `count` lessons in a row of the skill keyed `key` at `level`, each taken by
/// the learner before the next is laid.
fn in_a_row(pictures: &Pictures, key: &str, level: SkillLevel, count: usize) -> Vec<String> {
    let mut learner = Learner::default();
    let mut shown = Vec::new();
    for _ in 0..count {
        let planned = laid(&learner, pictures, key, level);
        let items: Vec<&ItemRef> = (planned.exercises.iter())
            .filter_map(|(exercise, _)| exercise.picture.as_ref())
            .collect();
        assert_eq!(items.len(), 1, "{key}: one picture a lesson");
        shown.push(items[0].item.clone());
        learner = learner.with(&ended(planned, &[]));
    }
    shown
}

#[test]
fn a_first_level_shape_comes_once_in_its_tier_until_each_came_then_the_next_tier_s() {
    let entries = [
        ("cat", Tier::Entry),
        ("bird", Tier::Entry),
        ("owl", Tier::Entry),
    ];
    let mut items = entries.to_vec();
    items.push(("fish", Tier::Simple));
    let pictures = Pictures::of(&[pack("shapes", PictureKind::Shape, &items)]);
    let shown = in_a_row(&pictures, "shape", SkillLevel::First, 4);
    let mut first = shown[..3].to_vec();
    first.sort();
    assert_eq!(first, ["bird", "cat", "owl"], "{shown:?}: none twice");
    // The first level's tier is the easiest of three in turn: entry three lessons, then simple.
    assert_eq!(shown[3], "fish");
    // A tier of one picture repeats it while its turn lasts (the test frames pack: the vase three
    // times, then the fish); a real pack holds many.
    let frames = [("vase", Tier::Entry), ("fish", Tier::Simple)];
    let pictures = Pictures::of(&[pack("frames", PictureKind::Shape, &frames)]);
    let shown = in_a_row(&pictures, "shape", SkillLevel::First, 4);
    assert_eq!(shown, ["vase", "vase", "vase", "fish"]);
}

#[test]
fn from_the_second_level_the_user_s_own_shape_comes_every_second_lesson() {
    let official = [
        ("lamp", Tier::Medium),
        ("jug", Tier::Medium),
        ("cup", Tier::Medium),
    ];
    let pictures = Pictures::of(&[
        pack("shapes", PictureKind::Shape, &official),
        pack(PERSONAL, PictureKind::Shape, &[("mine", Tier::Simple)]),
    ]);
    // A first-level lesson, then two at the second: the second lesson of the skill is the own.
    let mut learner = Learner::default();
    let mut shown = Vec::new();
    for level in [SkillLevel::First, SkillLevel::Second, SkillLevel::Second] {
        let planned = laid(&learner, &pictures, "shape", level);
        let picture = planned.exercises[0].0.picture.clone().expect("a picture");
        shown.push((picture.pack, picture.item));
        learner = learner.with(&ended(planned, &[]));
    }
    let packs: Vec<&str> = shown.iter().map(|(pack, _)| pack.as_str()).collect();
    assert_eq!(packs, ["shapes", PERSONAL, "shapes"], "{shown:?}");
    assert_eq!(shown[1].1, "mine");
}

/// S158: a photo a drawing from life was drawn against, set aside, is never a shape lesson's,
/// though a pack still holds it.
#[test]
fn a_photo_set_aside_never_comes_in_a_shape_lesson_but_is_still_held() {
    let official = [("lamp", Tier::Medium), ("jug", Tier::Medium)];
    let packs = [
        pack("shapes", PictureKind::Shape, &official),
        pack(PERSONAL, PictureKind::Shape, &[("photo", Tier::Simple)]),
    ];
    let photo = ItemRef {
        pack: PERSONAL.to_owned(),
        item: "photo".to_owned(),
    };
    let pictures = Pictures::of(&packs).photos_aside([&photo]);
    assert!(pictures.holds(&photo) && !pictures.has_personal(Tier::Hard));
    let mut learner = Learner::default();
    for level in [SkillLevel::First, SkillLevel::Second, SkillLevel::Second] {
        let planned = laid(&learner, &pictures, "shape", level);
        let picture = planned.exercises[0].0.picture.clone().expect("a picture");
        assert_eq!(picture.pack, "shapes", "{level:?}");
        learner = learner.with(&ended(planned, &[]));
    }
    // A photo alone: no shape to practise on.
    let alone = Pictures::of(&packs[1..]).photos_aside([&photo]);
    assert!(alone.is_empty() && alone.holds(&photo));
}

#[test]
fn a_division_s_compositions_come_in_turn_an_upright_one_among_them() {
    let items = [
        ("valley-house", Tier::Simple),
        ("ridge-farm", Tier::Simple),
        ("evening-field", Tier::Simple),
        ("high-meadow", Tier::Medium),
        ("hill-cottage", Tier::Medium),
    ];
    let pictures = Pictures::of(&[pack("compositions", PictureKind::Composition, &items)]);
    // The first level takes the simple and the medium tier in turn.
    let first = in_a_row(&pictures, "division", SkillLevel::First, 2);
    let tiers: Vec<bool> = (first.iter())
        .map(|item| item == "high-meadow" || item == "hill-cottage")
        .collect();
    assert_eq!(tiers, [false, true], "{first:?}");
    // The second level's medium pictures, none again while the other is left: the upright one
    // (the test pack's high meadow) by the second lesson.
    let mut second = in_a_row(&pictures, "division", SkillLevel::Second, 2);
    second.sort();
    assert_eq!(second, ["high-meadow", "hill-cottage"]);
}

#[test]
fn a_review_of_a_kind_whose_lesson_holds_one_exercise_is_its_redo_alone() {
    let items = [
        ("valley-house", Tier::Simple),
        ("high-meadow", Tier::Medium),
    ];
    let pictures = Pictures::of(&[pack("compositions", PictureKind::Composition, &items)]);
    let fresh = Learner::default();
    let division = laid(&fresh, &pictures, "division", SkillLevel::First);
    assert_eq!(division.exercises.len(), 1);
    let learner = fresh.with(&ended(division.clone(), &[Outcome::Redo]));
    let standing = Standing {
        pictures: &pictures,
        learner: &learner,
    };
    // The next lesson is Lines; the review on the side the division's redo, about five minutes.
    let review = schedule::review(&standing).expect("a review");
    assert_eq!(review.skill, skill("division"));
    assert_eq!(
        (review.exercises, review.minutes, review.redo),
        (1, 5, true)
    );
    let reviewed = schedule::reviewed(5, &standing).expect("its lesson");
    let kinds: Vec<Kind> = reviewed.exercises.iter().map(|(e, _)| e.kind).collect();
    assert_eq!(kinds, [division.exercises[0].0.kind]);
    let again = &reviewed.exercises[0].0.picture;
    assert_eq!(*again, division.exercises[0].0.picture);
    // A line's review keeps the redo and one more of its kind.
    let lined = laid(&fresh, &pictures, "line", SkillLevel::First);
    let learner = fresh.with(&ended(lined, &[Outcome::Redo]));
    let standing = Standing {
        pictures: &pictures,
        learner: &learner,
    };
    let review = schedule::review(&standing).expect("a review");
    let reviewed = schedule::reviewed(5, &standing).expect("its lesson");
    assert_eq!(review.skill, skill("line"));
    assert_eq!((review.exercises, reviewed.exercises.len()), (2, 2));
}
