//! One headless pass of a day's card in the practice book (S124): a small red seal for each
//! Pomodoro block completed that day, none on a day without.

use std::sync::Arc;

use egui::Rect;
use openpaint_practice::{Book, Day};
use openpaint_practice_app::BookProjection;

use super::days::day_card;
use crate::palette::VERMILION;
use crate::passes::{SCREEN, context, outlined, settled_drawn};
use crate::text::Language;

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
