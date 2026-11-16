//! Headless passes of a day's card in the practice book: a small red seal for each Pomodoro block
//! completed that day, none on a day without (S124); a day of three sheets, one of six lessons,
//! lies within the card over progress, each sheet named within its picture's width (S143).

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
            day_card(ui.ctx(), &shown, (20_754, cell), SCREEN);
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
        unit: None,
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
            day_card(ui.ctx(), &shown, (20_754, cell), SCREEN);
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
