//! Headless passes of a day's card in the practice book: a small red seal for each Pomodoro block
//! completed that day, none on a day without (S124); a day of three sheets, one of six lessons,
//! lies within the card over progress, each sheet named within its picture's width (S143); the
//! Progress page's curves named and dated, a skill with nothing judged named with no date, a day
//! alone a dot over its date (S147, S149); a sheet opened from a day coming back to it (S158h).

use std::sync::Arc;

use egui::{Rect, Shape};
use openpaint_app::ProjectId;
use openpaint_practice::{Book, Curve, Day, Progress, Skill, SkillLevel};
use openpaint_practice_app::{BookProjection, BookRecord, LessonRow, SheetRead};

use super::day::{day_card, rows};
use super::layout;
use crate::palette::VERMILION;
use crate::passes::{SCREEN, context, outlined, settled_drawn};
use crate::text::{self as t, Language};

/// A book of one day practised for 25 minutes with `blocks` completed.
fn book(blocks: u32) -> BookProjection {
    let day = Day {
        day: 20_754,
        seconds: 1500,
        records: Vec::new(),
        blocks,
    };
    BookProjection {
        book: Arc::new(Book {
            days: vec![day],
            ..Book::default()
        }),
        today: 20_754,
        ..BookProjection::default()
    }
}

#[test]
fn a_day_s_card_holds_a_red_seal_for_each_block() {
    let cell = Rect::from_min_size(egui::pos2(200.0, 300.0), egui::vec2(64.0, 64.0));
    for (blocks, language) in [(0, Language::English), (3, Language::Chinese)] {
        let ctx = context();
        let shown = book(blocks);
        let (_, shapes) = settled_drawn(&ctx, (1.0, language), |ui| {
            day_card(ui.ctx(), &shown, (20_754, cell), (SCREEN, false));
        });
        assert_eq!(outlined(&shapes, VERMILION), blocks as usize);
    }
}

/// A day like the author's on 15 November: a sheet of lesson 22, one of lessons 23 to 28 (key
/// shifts all), one of lesson 29 (colours), each read at its shape.
fn crowded() -> BookProjection {
    let row = |number, key| LessonRow {
        slot: 0,
        number,
        skill: Skill::of_key(key),
        first: false,
        retest: None,
    };
    let record = |id, rows| BookRecord {
        id,
        sheet: ProjectId::new(id),
        rows,
        copy: None,
    };
    let records = [
        record(1, vec![row(22, "key-shift")]),
        record(2, (23..=28).map(|n| row(n, "key-shift")).collect()),
        record(3, vec![row(29, "colour")]),
    ];
    let read = |size| SheetRead {
        size,
        frames: Vec::new(),
        marks: Vec::new(),
        lines: Vec::new(),
        colours: Vec::new(),
    };
    let reads = [(1, [2920, 1300]), (2, [1460, 2600]), (3, [2920, 1300])];
    let day = Day {
        day: 20_754,
        seconds: 240,
        records: vec![1, 2, 3],
        blocks: 0,
    };
    BookProjection {
        book: Arc::new(Book {
            days: vec![day],
            ..Book::default()
        }),
        today: 20_754,
        records: records.into(),
        read: Arc::new(reads.map(|(id, size)| (id, Arc::new(read(size)))).into()),
        ..BookProjection::default()
    }
}

#[test]
fn a_crowded_day_lies_within_its_card_each_name_under_its_picture() {
    let cell = Rect::from_min_size(egui::pos2(200.0, 300.0), egui::vec2(64.0, 64.0));
    let side = layout(SCREEN)[1];
    for language in [Language::English, Language::Chinese] {
        let ctx = context();
        let shown = crowded();
        let (map, shapes) = settled_drawn(&ctx, (1.0, language), |ui| {
            day_card(ui.ctx(), &shown, (20_754, cell), (SCREEN, false));
        });
        for record in shown.records.iter() {
            let english = t::sheet_lessons(Language::English, &record.rows);
            let name = t::sheet_lessons(language, &record.rows);
            let [left, top, right, bottom] = map.find(&english).expect("each sheet named").rect;
            let picture = Rect::from_min_max(egui::pos2(left, top), egui::pos2(right, bottom));
            assert!(side.contains_rect(picture), "{picture:?} within {side:?}");
            let caption = shapes
                .iter()
                .find_map(|shape| match shape {
                    Shape::Text(text) if text.galley.text() == name => {
                        Some(text.galley.rect.translate(text.pos.to_vec2()))
                    }
                    _ => None,
                })
                .expect("each sheet's caption");
            assert!(caption.left() >= picture.left() - 0.5, "{name}");
            assert!(
                caption.right() <= picture.right() + 0.5,
                "{name}: {caption:?} {picture:?}"
            );
            assert!(caption.top() >= picture.bottom(), "{name}");
        }
    }
}

#[test]
fn sheets_fall_into_rows_as_many_as_fit() {
    assert_eq!(rows(&[224.0, 100.0, 140.0], 500.0), vec![0..3]);
    assert_eq!(rows(&[224.0, 224.0, 224.0], 500.0), vec![0..2, 2..3]);
    assert_eq!(rows(&[600.0, 100.0], 500.0), vec![0..1, 1..2]);
    assert!(rows(&[], 500.0).is_empty());
}

/// A book whose lines were practised on six days, a ten-day gap before the last two, the second
/// level begun on the fourth.
fn curved() -> BookProjection {
    let curve = Curve {
        skill: Skill::of_key("line").expect("lines"),
        days: vec![20_731, 20_732, 20_734, 20_736, 20_746, 20_747],
        ability: vec![0.0, 0.2, 0.3, 0.5, 0.7, 0.8],
        band: vec![0.0, 0.3, 0.35, 0.4, 0.5, 0.5],
        levels: vec![(3, SkillLevel::Second)],
        retest: None,
    };
    BookProjection {
        progress: Some(Arc::new(Progress {
            curves: vec![curve],
        })),
        today: 20_747,
        ..BookProjection::default()
    }
}

#[test]
fn the_progress_page_draws_a_curve_a_skill_named_and_dated_with_no_number() {
    for language in [Language::English, Language::Chinese] {
        let ctx = context();
        let shown = curved();
        let area = layout(SCREEN)[0].union(layout(SCREEN)[1]);
        let (_, shapes) = settled_drawn(&ctx, (1.0, language), |ui| {
            super::progress::curves(ui, area, &shown);
        });
        let texts: Vec<String> = (shapes.iter())
            .filter_map(|shape| match shape {
                Shape::Text(text) => Some(text.galley.text().to_owned()),
                _ => None,
            })
            .collect();
        let name = t::skill_name(Skill::of_key("line").expect("lines")).get(language);
        assert!(texts.iter().any(|text| text == name), "{texts:?}");
        assert!(texts.contains(&t::copy_date(language, 20_731)), "{texts:?}");
        assert!(texts.contains(&t::copy_date(language, 20_747)), "{texts:?}");
        let digit = |text: &&String| text.chars().any(|c| c.is_ascii_digit());
        let numbers: Vec<&String> = texts.iter().filter(digit).collect();
        assert_eq!(
            numbers.len(),
            2,
            "only the two dates hold a number: {numbers:?}"
        );
    }
}

/// The lines' book (`curved`) with combined lines practised and nothing of them judged, and
/// colours judged on one day.
fn practised() -> BookProjection {
    let shown = curved();
    let Some(progress) = &shown.progress else {
        return shown;
    };
    let curve = |key, days: Vec<i64>| Curve {
        skill: Skill::of_key(key).expect("a skill"),
        ability: vec![0.0; days.len()],
        band: vec![0.0; days.len()],
        days,
        levels: Vec::new(),
        retest: None,
    };
    let mut curves = progress.curves.clone();
    curves.extend([curve("combined", Vec::new()), curve("colour", vec![20_740])]);
    BookProjection {
        progress: Some(Arc::new(Progress { curves })),
        ..shown
    }
}

#[test]
fn a_skill_with_nothing_judged_is_named_dateless_and_a_lone_day_is_a_dot_over_its_date() {
    for language in [Language::English, Language::Chinese] {
        let ctx = context();
        let shown = practised();
        let area = layout(SCREEN)[0].union(layout(SCREEN)[1]);
        let (_, shapes) = settled_drawn(&ctx, (1.0, language), |ui| {
            super::progress::curves(ui, area, &shown);
        });
        let texts: Vec<(String, Rect)> = (shapes.iter())
            .filter_map(|shape| match shape {
                Shape::Text(text) => Some((
                    text.galley.text().to_owned(),
                    text.galley.rect.translate(text.pos.to_vec2()),
                )),
                _ => None,
            })
            .collect();
        for key in ["line", "combined", "colour"] {
            let name = t::skill_name(Skill::of_key(key).expect("a skill")).get(language);
            assert!(texts.iter().any(|(text, _)| text == name), "{key}");
        }
        let dated = |text: &&(String, Rect)| text.0.chars().any(|c| c.is_ascii_digit());
        assert_eq!(
            texts.iter().filter(dated).count(),
            3,
            "lines' two, colours' one"
        );
        let date = t::copy_date(language, 20_740);
        let (_, under) = (texts.iter().find(|(text, _)| *text == date)).expect("its date");
        let lone: Vec<_> = (shapes.iter())
            .filter_map(|shape| match shape {
                Shape::Circle(dot) if (dot.radius - 3.5).abs() < 1e-3 => Some(dot.center),
                _ => None,
            })
            .collect();
        assert_eq!(lone.len(), 1, "one day alone: {lone:?}");
        assert!(
            (lone[0].x - under.left()).abs() < 1.0,
            "{lone:?} over {under:?}"
        );
        assert!(lone[0].y < under.top());
    }
}

/// A sheet opened from a day's card comes back to the book (S158h): while it shows on the canvas
/// the book waits, the screen's switch back to the home keeps it open (its back), and the sheet
/// lands in its picture on the card; the next back closes it. A sheet that never showed (its
/// opening failed) waits for nothing.
#[test]
fn a_sheet_opened_from_a_day_comes_back_to_the_book() {
    let (word, picture) = (
        Rect::from_min_size(egui::pos2(40.0, 800.0), egui::vec2(120.0, 48.0)),
        Rect::from_min_size(egui::pos2(900.0, 300.0), egui::vec2(140.0, 152.0)),
    );
    let mut book = super::BookState::default();
    book.open(word);
    book.left = Some(picture);
    book.sheet_shows();
    assert_eq!(
        book.landing(),
        Some(picture),
        "it goes back into its picture"
    );
    assert!(
        book.close() && book.shows(),
        "the switch home keeps the book"
    );
    assert!(book.coming, "shown as it was left");
    assert_eq!(book.landing(), None, "landed once");
    assert!(book.close() && !book.shows(), "the next back closes it");
    book.open(word);
    book.left = Some(picture);
    assert!(
        book.close() && !book.shows(),
        "no sheet showed: the back closes it"
    );
}
