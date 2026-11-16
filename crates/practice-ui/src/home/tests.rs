//! One headless pass of the home's column and the path page on a fixture path: their controls
//! registered by name and on the screen, the column's Start asking a lesson, and a sheet left
//! going into Practice book (S144).

use egui::Rect;
use openpaint_practice::{Offer, Skill, SkillLevel};
use openpaint_practice_app::{OptionsIntent, PathProjection, PracticeLog, Start, TimerShown};
use openpaint_ui::HomeHook;

use super::{HomePart, PathHook};
use crate::passes::{assert_on_screen, context, drawn, pass, settled, tap};
use crate::text::{self as t, Language};

/// Where the home lays the column, in points.
pub(super) const COLUMN: Rect =
    Rect::from_min_max(egui::pos2(32.0, 56.0), egui::pos2(632.0, 868.0));

/// A fresh learner's path, and one with a next lesson and a review waiting.
pub(super) fn paths() -> (PathProjection, PathProjection) {
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
    let timer = TimerShown::default();
    let mut column = |ui: &mut egui::Ui, path: &PathProjection, asked: &mut Vec<OptionsIntent>| {
        let (mut taps, mut timed) = (Vec::new(), Vec::new());
        part.column(
            ui,
            COLUMN,
            (path, &timer, true),
            ((&mut taps, asked), &mut timed),
        );
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

/// A sheet left goes back into Practice book (S144): the column gives the book's place, and what
/// the sheet laid over the canvas shrinks with its picture from where it lay to where it is now.
#[test]
fn a_sheet_left_goes_into_practice_book_its_marks_with_it() {
    let (fresh, _) = paths();
    let ctx = context();
    let mut part = HomePart::default();
    let timer = TimerShown::default();
    let (from, at) = (
        Rect::from_min_size(egui::pos2(100.0, 50.0), egui::vec2(800.0, 500.0)),
        Rect::from_min_size(egui::pos2(40.0, 800.0), egui::vec2(80.0, 50.0)),
    );
    let ink = egui::Stroke::new(2.0, egui::Color32::BLACK);
    let laid = [egui::Shape::line_segment([from.min, from.max], ink)];
    let mut place = Rect::NOTHING;
    let (map, shapes) = drawn(&ctx, (1.0, Language::English, Vec::new()), |ui| {
        let (mut taps, mut asked, mut timed) = (Vec::new(), Vec::new(), Vec::new());
        let mut hook = PathHook {
            part: &mut part,
            path: &fresh,
            notice: None,
            taps: &mut asked,
            timer: &timer,
            timer_taps: &mut timed,
            laid: &laid,
        };
        place = hook.column(ui, COLUMN, true, &mut taps);
        let painter = ui.ctx().layer_painter(egui::LayerId::debug());
        hook.carried(&painter, (from, at));
    });
    let [left, top, right, bottom] = map.find(t::PRACTICE_BOOK.id()).expect("the book").rect;
    let book = Rect::from_min_max(egui::pos2(left, top), egui::pos2(right, bottom));
    assert!((place.min - book.min).length() < 0.5 && (place.max - book.max).length() < 0.5);
    let moved = shapes.iter().find_map(|shape| match shape {
        egui::Shape::LineSegment { points, stroke } if stroke.color == ink.color => {
            Some((*points, stroke.width))
        }
        _ => None,
    });
    let (points, width) = moved.expect("the sheet's marks carried");
    assert!((points[0] - at.min).length() < 0.01 && (points[1] - at.max).length() < 0.01);
    assert!((width - 0.2).abs() < 1e-4, "scaled with the sheet: {width}");
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
    let timer = TimerShown::default();
    for (from, language) in [(10.0, Language::English), (30.0, Language::Chinese)] {
        let map = settled(&ctx, (from, language), |ui| {
            let (mut taps, mut asked, mut timed) = (Vec::new(), Vec::new(), Vec::new());
            let mut hook = PathHook {
                part: &mut part,
                path: &going,
                notice: None,
                taps: &mut asked,
                timer: &timer,
                timer_taps: &mut timed,
                laid: &[],
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

/// A path with unit 2's start re-test due and unit 1's end.
fn retests_due() -> PathProjection {
    let (_, going) = paths();
    let skill = |key| Skill::of_key(key).expect("a skill");
    let retest = |unit, end, skills: Vec<Skill>| openpaint_practice::Retest {
        unit,
        end,
        skills,
        minutes: 6,
    };
    PathProjection {
        retest: Some(retest(2, false, vec![skill("box"), skill("ellipse")])),
        retest_end: Some(retest(1, true, vec![skill("line"), skill("colour")])),
        ..going
    }
}

#[test]
fn a_unit_s_start_re_test_takes_the_next_lesson_s_place_and_its_end_a_row_of_its_own() {
    let due = retests_due();
    let ctx = context();
    let mut part = HomePart::default();
    let timer = TimerShown::default();
    let mut taps = Vec::new();
    for language in [Language::English, Language::Chinese] {
        let (map, shapes) = drawn(&ctx, (1.0, language, Vec::new()), |ui| {
            let (mut asked, mut timed) = (Vec::new(), Vec::new());
            part.column(
                ui,
                COLUMN,
                (&due, &timer, true),
                ((&mut taps, &mut asked), &mut timed),
            );
        });
        assert_on_screen(&map, t::START.id());
        assert_on_screen(&map, t::RETEST.id());
        let texts: Vec<String> = (shapes.iter())
            .filter_map(|shape| match shape {
                egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
                _ => None,
            })
            .collect();
        let name = t::retest_name(language, 2);
        assert!(texts.contains(&name), "{name} in {texts:?}");
    }
    let mut asked: Vec<OptionsIntent> = Vec::new();
    let map = settled(&ctx, (5.0, Language::English), |ui| {
        let mut timed = Vec::new();
        part.column(
            ui,
            COLUMN,
            (&due, &timer, true),
            ((&mut taps, &mut asked), &mut timed),
        );
    });
    let [press, release] = tap(&map, t::RETEST.id());
    for (time, events) in [(8.0, press), (8.1, release)] {
        pass(&ctx, (time, Language::English, events), |ui| {
            let mut timed = Vec::new();
            part.column(
                ui,
                COLUMN,
                (&due, &timer, true),
                ((&mut taps, &mut asked), &mut timed),
            );
        });
    }
    assert!(
        asked.contains(&OptionsIntent::Lesson(Start::Retest(1, true))),
        "{asked:?}"
    );
}
