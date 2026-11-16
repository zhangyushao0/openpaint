//! Progress from the learner's own judgements (S147) on synthetic learners, as the design's
//! prototype tried it: an improving learner's curve rises clear of its band, a flat one's stays
//! within it and is seldom called changed, a level rising reads as no fall, an erratic eye widens
//! the band, a random one seldom calls a fall; and which pair a lesson's end asks. A skill
//! practised with nothing judged is on the page without a day (`practised.rs`, S149).

mod ask;
mod practised;
mod sim;

use openpaint_practice::{Change, Curve, Progress, Skill, SkillFit};
use sim::{Eye, Learned, Truth, learner};

const SEEDS: u64 = 40;

fn lines() -> Skill {
    Skill::of_key("line").expect("lines")
}

fn fitted(learned: &Learned) -> SkillFit {
    SkillFit::of(lines(), &learned.sheets, &learned.pairs, 0).expect("a history")
}

/// The curve's rise from its first day to its last, and its band at the last.
fn rise(curve: &Curve) -> (f64, f64) {
    let last = curve.ability.len() - 1;
    (curve.ability[last], curve.band[last])
}

fn seeded(truth: Truth, eye: Eye) -> impl Iterator<Item = SkillFit> {
    (0..SEEDS).map(move |seed| fitted(&learner(truth, eye, 0x5147_0000 + seed)))
}

#[test]
fn an_improving_learner_s_curve_rises_clear_of_its_band() {
    let (mut rises, mut clear) = (Vec::new(), 0);
    for fit in seeded(Truth::Rising(3.5), Eye::True) {
        let (rose, band) = rise(&fit.curve());
        rises.push(rose);
        clear += usize::from(rose - band > 0.0);
        assert!(fit.steps() < 30, "{} Newton steps", fit.steps());
    }
    let mean = rises.iter().sum::<f64>() / rises.len() as f64;
    assert!(mean > 2.5, "a rise of 3.5 read as {mean:.2}");
    assert!(
        rises.iter().filter(|r| **r > 2.0).count() >= 36,
        "{rises:?}"
    );
    assert!(
        clear >= 38,
        "the band clear of the start on {clear} of {SEEDS}"
    );
}

#[test]
fn a_flat_learner_stays_within_its_band_and_is_seldom_called_changed() {
    let (mut within, mut twice, mut changed) = (0, 0, 0);
    for fit in seeded(Truth::Flat, Eye::True) {
        let (rose, band) = rise(&fit.curve());
        within += usize::from(rose.abs() < band);
        twice += usize::from(rose.abs() < 2.0 * band);
        changed += usize::from(fit.change().is_some());
    }
    // One standard deviation holds about two thirds, two nearly all.
    assert!(within >= 26, "within its band on {within} of {SEEDS}");
    assert!(twice >= 38, "within twice its band on {twice} of {SEEDS}");
    assert!(changed <= 3, "called changed on {changed} of {SEEDS}");
}

#[test]
fn a_level_rising_reads_as_no_fall() {
    let mut fell = 0;
    for fit in seeded(Truth::Forced, Eye::True) {
        let curve = fit.curve();
        assert_eq!(curve.levels.len(), 2, "both rises found");
        for (at, _) in &curve.levels {
            let dropped = curve.ability[*at - 1] - curve.ability[*at];
            assert!(
                dropped < curve.band[*at],
                "a fall of {dropped:.2} at a level's rise"
            );
        }
        let [second, third] = fit.levels();
        assert!(
            second > 0.5 && third > 0.5,
            "harder levels read as {second:.2} {third:.2}"
        );
        fell += usize::from(matches!(fit.change(), Some(Change::Fell { .. })));
    }
    assert!(fell <= 2, "a fall read on {fell} of {SEEDS}");
}

#[test]
fn an_erratic_eye_widens_the_band_and_a_random_one_seldom_calls_a_fall() {
    let bands = |eye: Eye| -> Vec<f64> {
        seeded(Truth::Rising(3.5), eye)
            .map(|fit| rise(&fit.curve()).1)
            .collect()
    };
    let (steady, erratic) = (bands(Eye::True), bands(Eye::Erratic));
    let wider = steady.iter().zip(&erratic).filter(|(s, e)| e > s).count();
    let ratio = erratic.iter().sum::<f64>() / steady.iter().sum::<f64>();
    assert!(wider >= 36, "wider on {wider} of {SEEDS}");
    assert!(ratio > 1.05, "the band {ratio:.2} times as wide");
    let fell = seeded(Truth::Rising(3.5), Eye::Random)
        .filter(|fit| matches!(fit.change(), Some(Change::Fell { .. })))
        .count();
    assert!(fell <= 6, "a random eye's fall on {fell} of {SEEDS}");
}

#[test]
fn the_book_s_curves_come_in_the_course_s_order_with_the_skills_practised() {
    let learned = learner(Truth::Rising(2.0), Eye::True, 7);
    let progress = Progress::of(&learned.sheets, &learned.pairs, 0);
    assert_eq!(progress.curves.len(), 1);
    let curve = progress.curve(lines()).expect("lines practised");
    assert_eq!(curve.ability[0], 0.0, "read from its first day");
    assert_eq!(curve.band[0], 0.0);
    assert_eq!(curve.days.len(), curve.ability.len());
    assert!(curve.days.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(Progress::of(&[], &[], 0).curves.is_empty());
}
