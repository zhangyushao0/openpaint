//! Colour under light's model and rules (S154j), validated as key shifts' were (S107) on scenes
//! of every level: the model's own render breaks no rule; each copy made wrong on purpose breaks
//! the rule it is made for and, but where the design says, no other; a region left bare reads as
//! not painted.

use super::check::{DARKER, Rule, VALUE_OFF};
use super::regions::Region;
use super::scene::{ColourTask, LitColour, Palette};
use crate::maths::colour::{Rgb8, oklab};

/// The side the copies are painted at, pixels; seeds looked at a variant (more where its hue is
/// drawn).
const SIDE: u32 = 400;
const SEEDS: u64 = 3;
const HUED: u64 = 8;
/// The sheet's paper.
const PAPER: Rgb8 = [255; 3];

/// Scenes of every variant, `SEEDS` each, `HUED` where the object's or the lamp's hue is drawn.
fn scenes() -> Vec<ColourTask> {
    (LitColour::ALL.into_iter())
        .flat_map(|variant| {
            let hued = matches!(variant.palette, Palette::Local | Palette::Bounce);
            let seeds = if hued { HUED } else { SEEDS };
            (0..seeds).map(move |seed| ColourTask::generate(variant, seed))
        })
        .collect()
}

/// `task`'s copy whose pixels are `rgba` as read: each region asked and its colour.
fn read(task: &ColourTask, rgba: &[u8]) -> Vec<(Region, Option<Rgb8>)> {
    task.read([SIDE; 2], rgba, PAPER)
}

/// The rules `task`'s copy `rgba` breaks, each once.
fn broken(task: &ColourTask, rgba: &[u8]) -> Vec<Rule> {
    let mut rules: Vec<Rule> = (task.broken(&read(task, rgba)).into_iter())
        .map(|(rule, _)| rule)
        .collect();
    rules.dedup();
    rules.sort_by_key(|rule| Rule::ALL.iter().position(|r| r == rule));
    rules.dedup();
    rules
}

#[test]
fn every_scene_keeps_its_rules_and_the_model_s_render_breaks_none() {
    for task in scenes() {
        assert!(task.keeps(1.0), "{:?}", task.variant);
        assert_eq!(broken(&task, &task.render(SIDE)), [], "{:?}", task.variant);
        assert_eq!(
            broken(&task, &task.flat(&[], SIDE)),
            [],
            "{:?}",
            task.variant
        );
        // The families apart, the light and the sky apart round the wheel.
        let lightness = |region| oklab(task.model(region).expect("held"))[0];
        assert!(lightness(Region::Light) - lightness(Region::Shadow) >= 0.2);
        let hue = |c: Rgb8| crate::maths::colour::oklch(c)[2];
        let turned = (hue(task.colours.light) - hue(task.colours.sky) + 180.0).rem_euclid(360.0);
        assert!((turned - 180.0).abs() >= 60.0, "{:?}", task.variant);
    }
}

#[test]
fn each_copy_made_wrong_on_purpose_breaks_its_rule() {
    let mut made = 0;
    for task in scenes() {
        for rule in task.rules() {
            let Some(wrong) = task.wrong(rule) else {
                continue;
            };
            made += 1;
            let rules = broken(&task, &task.flat(&wrong, SIDE));
            // At 稳 the shadow family darkened only lacks the ground's bounce too.
            let bounce = task.variant.palette == Palette::Bounce;
            if rule == Rule::ShadowColour && bounce {
                assert_eq!(rules, [rule, Rule::Bounce], "{:?}", task.variant);
                continue;
            }
            if rule != Rule::Darker {
                assert_eq!(rules, [rule], "{:?} {rule:?}", task.variant);
                continue;
            }
            // A shadow nearly as light as the light leaves the model's value too where the
            // model's families lie further apart than half a step and two values' bands; at the
            // border, as the 8-bit colours round.
            let lightness = |region| oklab(task.model(region).expect("held"))[0];
            let gap = lightness(Region::Light) - lightness(Region::Shadow);
            let border = DARKER / 2.0 + 2.0 * VALUE_OFF - 0.01;
            if gap < border - 0.01 {
                assert_eq!(rules, [Rule::Darker], "{:?}", task.variant);
            } else if gap > border + 0.01 {
                assert_eq!(rules, [Rule::Darker, Rule::Value], "{:?}", task.variant);
            } else {
                assert!(rules.starts_with(&[Rule::Darker]) && rules.len() <= 2);
            }
        }
    }
    assert!(made >= 60, "{made} wrong copies");
}

#[test]
fn a_region_left_bare_is_not_painted() {
    let task = ColourTask::generate(LitColour::ALL[7], 1);
    let mut copy = task.flat(&[(Region::Cast, PAPER)], SIDE);
    let bare = read(&task, &copy);
    assert!(
        bare.iter()
            .all(|(region, colour)| colour.is_none() == (*region == Region::Cast))
    );
    // Painted, it reads.
    copy = task.flat(&[], SIDE);
    assert!(
        read(&task, &copy)
            .iter()
            .all(|(_, colour)| colour.is_some())
    );
}
