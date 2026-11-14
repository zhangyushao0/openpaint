//! One headless pass of each Pomodoro state on the sheet, on fixtures (S124): the bar's ring
//! counting, held and at the time (End in place of Next), its menu, a lesson's end hidden while a
//! block runs, and the block's end with and without a dot gained, during the break and after it.

use std::sync::Arc;

use egui::Rect;
use openpaint_practice::{BlockTally, Length, Outcome, Skill};
use openpaint_practice_app::{BlockEnd, BlockShown, TimerIntent};

use super::bar::{CardShown, card_shown};
use super::block_end::{BlockTap, block_end_card};
use super::timer::{Command, next_or_end, ring_menu, ring_word};
use crate::palette::VERMILION;
use crate::passes::{assert_on_screen, context, drawn, outlined, pass, settled_drawn, tap, words};
use crate::text::{self as t, Language};

const COUNTING: BlockShown = BlockShown::Running {
    left: 18,
    share: 0.72,
    paused: false,
};
const HELD: BlockShown = BlockShown::Running {
    left: 18,
    share: 0.72,
    paused: true,
};

#[test]
fn the_bar_s_ring_is_named_by_its_minutes_and_a_tap_opens_its_menu() {
    let ctx = context();
    for (language, minutes, paused) in [
        (Language::English, "18 min", "Paused"),
        (Language::Chinese, "18 分", "暂停"),
    ] {
        let (map, shapes) = drawn(&ctx, (1.0, language, Vec::new()), |ui| {
            ring_word(ui, &COUNTING);
        });
        assert_on_screen(&map, "18 min");
        assert!(
            words(&shapes).iter().any(|word| word == minutes),
            "{minutes}"
        );
        let (map, shapes) = drawn(&ctx, (2.0, language, Vec::new()), |ui| {
            ring_word(ui, &HELD);
        });
        assert_on_screen(&map, "18 min");
        assert!(words(&shapes).iter().any(|word| word == paused), "held");
    }
    let map = pass(&ctx, (3.0, Language::English, Vec::new()), |ui| {
        ring_word(ui, &COUNTING);
    });
    let mut tapped = false;
    let [press, release] = tap(&map, "18 min");
    for (time, events) in [(3.1, press), (3.2, release)] {
        pass(&ctx, (time, Language::English, events), |ui| {
            tapped |= ring_word(ui, &COUNTING).is_some_and(|(_, tapped)| tapped);
        });
    }
    assert!(tapped, "the ring opens its menu");
}

#[test]
fn at_the_time_the_ring_is_red_with_time_and_next_reads_end() {
    let ctx = context();
    for (language, time) in [(Language::English, "Time"), (Language::Chinese, "到点")] {
        let (map, shapes) = drawn(&ctx, (1.0, language, Vec::new()), |ui| {
            ui.horizontal(|ui| {
                ring_word(ui, &BlockShown::Up);
                next_or_end(ui, (false, true), true);
            });
        });
        assert!(map.find("18 min").is_err(), "no menu at the time");
        assert!(words(&shapes).iter().any(|word| word == time), "{time}");
        assert_eq!(
            outlined(&shapes, VERMILION),
            1,
            "the ring in the seal's red"
        );
        assert_on_screen(&map, t::END.id());
        assert!(map.find(t::NEXT.id()).is_err() && map.find(t::FINISH.id()).is_err());
    }
    let commanded = |up: bool, name: &str, at: f64| {
        let mut asked = None;
        let map = pass(&ctx, (at, Language::English, Vec::new()), |ui| {
            next_or_end(ui, (false, up), true);
        });
        let [press, release] = tap(&map, name);
        for (time, events) in [(at + 0.1, press), (at + 0.2, release)] {
            pass(&ctx, (time, Language::English, events), |ui| {
                asked = asked.or(next_or_end(ui, (false, up), true).map(|(command, _)| command));
            });
        }
        asked
    };
    assert_eq!(commanded(true, t::END.id(), 10.0), Some(Command::End));
    assert_eq!(commanded(false, t::NEXT.id(), 20.0), Some(Command::Next));
}

#[test]
fn the_ring_s_menu_pauses_continues_and_stops() {
    let ring = Rect::from_min_size(egui::pos2(500.0, 8.0), egui::vec2(120.0, 48.0));
    for (paused, held, intent) in [
        (false, t::PAUSE.id(), TimerIntent::Pause),
        (true, t::CONTINUE.id(), TimerIntent::Resume),
    ] {
        for (from, language) in [(10.0, Language::English), (30.0, Language::Chinese)] {
            let ctx = context();
            let (map, _) = settled_drawn(&ctx, (from, language), |ui| {
                ring_menu(ui.ctx(), ring, paused);
            });
            assert_on_screen(&map, held);
            assert_on_screen(&map, t::STOP_TIMER.id());
            let mut asked = Vec::new();
            for name in [held, t::STOP_TIMER.id()] {
                let [press, release] = tap(&map, name);
                for (step, events) in [press, release].into_iter().enumerate() {
                    let at = from + 3.0 + 0.1 * step as f64;
                    pass(&ctx, (at, language, events), |ui| {
                        asked.extend(ring_menu(ui.ctx(), ring, paused).0);
                    });
                }
            }
            assert_eq!(asked, [intent, TimerIntent::Stop]);
        }
    }
}

#[test]
fn no_lesson_s_end_shows_while_a_block_runs_and_a_block_s_end_takes_its_place() {
    let ended = BlockShown::Ended {
        end: Arc::new(BlockEnd::default()),
        rest: Some((4, 0.8)),
    };
    let lesson_done = (true, false, false);
    assert_eq!(card_shown(lesson_done, None), Some(CardShown::End));
    for block in [COUNTING, HELD, BlockShown::Up] {
        assert_eq!(card_shown(lesson_done, Some(&block)), None, "{block:?}");
        assert_eq!(
            card_shown((false, true, false), Some(&block)),
            Some(CardShown::How)
        );
    }
    assert_eq!(
        card_shown(lesson_done, Some(&ended)),
        Some(CardShown::BlockEnd)
    );
    assert_eq!(
        card_shown((false, false, true), None),
        Some(CardShown::Compared)
    );
}

/// A block of five lessons and 19 exercises, Lines gaining its first dot when `gained`.
fn block(gained: bool) -> BlockEnd {
    let line = Skill::of_key("line").expect("lines");
    let mut results = vec![Some(Outcome::Clean); 15];
    results.extend([Some(Outcome::Near); 3]);
    results.push(Some(Outcome::Redo));
    BlockEnd {
        length: Length::TwentyFive,
        tally: BlockTally {
            lessons: 5,
            exercises: 19,
            results,
            skills: vec![(line, if gained { 0 } else { 1 }, 1)],
        },
    }
}

#[test]
fn a_block_s_end_shows_its_length_counts_dots_seal_and_break() {
    let ctx = context();
    let english = [
        "25 minutes",
        "5 lessons · 19 exercises",
        "Lines up to “Beginning”",
        "FOCUS",
        "Break",
        "4 min",
    ];
    let chinese = [
        "25 分钟",
        "5 课 · 19 题",
        "线条到“入门”了",
        "番",
        "茄",
        "休息",
        "4 分",
    ];
    for (language, shown) in [
        (Language::English, &english[..]),
        (Language::Chinese, &chinese),
    ] {
        let end = block(true);
        let (map, shapes) = drawn(&ctx, (1.0, language, Vec::new()), |ui| {
            block_end_card(ui, &end, Some((4, 0.8)));
        });
        let seen = words(&shapes);
        for word in shown {
            assert!(seen.iter().any(|w| w == word), "{word} among {seen:?}");
        }
        assert_eq!(outlined(&shapes, VERMILION), 1, "its seal");
        assert_on_screen(&map, t::ANOTHER_POMODORO.id());
        assert_on_screen(&map, t::ENOUGH.id());
    }
    let (_, shapes) = drawn(&ctx, (2.0, Language::English, Vec::new()), |ui| {
        block_end_card(ui, &block(false), None);
    });
    let seen = words(&shapes);
    assert!(!seen.iter().any(|w| w.contains("up to")), "no dot gained");
    assert!(!seen.iter().any(|w| w.ends_with(" min")), "the break over");
    assert!(seen.iter().any(|w| w == "Break"));
    let mut tapped = None;
    let end = block(true);
    let map = pass(&ctx, (3.0, Language::English, Vec::new()), |ui| {
        block_end_card(ui, &end, Some((4, 0.8)));
    });
    let [press, release] = tap(&map, t::ANOTHER_POMODORO.id());
    for (time, events) in [(3.1, press), (3.2, release)] {
        pass(&ctx, (time, Language::English, events), |ui| {
            tapped = tapped.or(block_end_card(ui, &end, Some((4, 0.8))));
        });
    }
    assert_eq!(tapped, Some(BlockTap::Another));
}
