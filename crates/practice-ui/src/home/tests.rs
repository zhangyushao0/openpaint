//! One headless pass of the home's column and the path page on a fixture path: their controls
//! registered by name and on the screen, and the column's Start asking a lesson.

use egui::Rect;
use openpaint_practice::{Offer, Skill, SkillLevel};
use openpaint_practice_app::{OptionsIntent, PathProjection, PracticeLog, Start};
use openpaint_ui::HomeHook;

use super::{HomePart, PathHook};
use crate::passes::{assert_on_screen, context, pass, settled, tap};
use crate::text::{self as t, Language};

/// Where the home lays the column, in points.
const COLUMN: Rect = Rect::from_min_max(egui::pos2(32.0, 56.0), egui::pos2(632.0, 868.0));

/// A fresh learner's path, and one with a next lesson and a review waiting.
fn paths() -> (PathProjection, PathProjection) {
    let fresh = PracticeLog::new(Vec::new(), Vec::new(), Vec::new()).path();
    let skill = |key| Skill::of_key(key).expect("a skill");
    let offer = |key, exercises, redo| Offer {
        skill: skill(key),
        level: SkillLevel::Second,
        exercises,
        minutes: 4,
        redo,
    };
    let mut going = fresh.clone();
    (going.first, going.next, going.review) = (
        false,
        Some(offer("combined", 5, false)),
        Some(offer("line", 2, true)),
    );
    (fresh, going)
}

#[test]
fn the_column_offers_start_all_units_and_the_book_and_start_asks_the_next_lesson() {
    let (fresh, going) = paths();
    assert!(fresh.first, "a fresh learner's path");
    let ctx = context();
    let mut part = HomePart::default();
    let mut asked: Vec<OptionsIntent> = Vec::new();
    let mut column = |ui: &mut egui::Ui, path: &PathProjection, asked: &mut Vec<OptionsIntent>| {
        let mut taps = Vec::new();
        part.column(ui, COLUMN, (path, true), (&mut taps, asked));
    };
    for language in [Language::English, Language::Chinese] {
        let map = pass(&ctx, (1.0, language, Vec::new()), |ui| {
            column(ui, &fresh, &mut asked);
        });
        for name in [
            t::START.id(),
            t::ALL_UNITS.id(),
            t::PRACTICE_BOOK.id(),
            t::PRACTICE_OPTIONS.id(),
        ] {
            assert_on_screen(&map, name);
        }
        assert!(
            map.find(t::REVIEW.id()).is_err(),
            "no review while none waits"
        );
    }
    let map = pass(&ctx, (2.0, Language::English, Vec::new()), |ui| {
        column(ui, &going, &mut asked);
    });
    assert_on_screen(&map, t::REVIEW.id());
    let [press, release] = tap(&map, t::START.id());
    for (time, events) in [(2.1, press), (2.2, release)] {
        pass(&ctx, (time, Language::English, events), |ui| {
            column(ui, &going, &mut asked);
        });
    }
    assert_eq!(asked, [OptionsIntent::Lesson(Start::Next)]);
}

#[test]
fn the_path_page_shows_the_units_the_skills_and_the_offers() {
    let (_, going) = paths();
    let ctx = context();
    let mut part = HomePart::default();
    part.path.open(
        Rect::from_min_size(egui::pos2(40.0, 800.0), egui::vec2(120.0, 48.0)),
        1,
    );
    for (from, language) in [(10.0, Language::English), (30.0, Language::Chinese)] {
        let map = settled(&ctx, (from, language), |ui| {
            let (mut taps, mut asked) = (Vec::new(), Vec::new());
            let mut hook = PathHook {
                part: &mut part,
                path: &going,
                notice: None,
                taps: &mut asked,
            };
            hook.page(ui.ctx(), &mut taps);
        });
        let named = [
            t::HOME.id(),
            t::LINES_SKILL.id(),
            t::COMBINED_SKILL.id(),
            t::COLOURS_SKILL.id(),
            t::START.id(),
            t::REVIEW.id(),
            t::FULL_LESSON.id(),
            t::PRACTICE_BOOK.id(),
        ];
        for unit in [1, 8] {
            assert_on_screen(&map, &t::unit_number(Language::English, unit));
        }
        for name in named {
            assert_on_screen(&map, name);
        }
    }
    assert!(part.path.covers(), "the page lies over the whole home");
}
