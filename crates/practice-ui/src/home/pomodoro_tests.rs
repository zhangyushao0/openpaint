//! One headless pass of the Pomodoro on the home, on fixtures (S124): the row idle and under way,
//! today's seals at the header's right, the practice options' Break chime.

use egui::Rect;
use openpaint_practice::Length;
use openpaint_practice_app::{BlockShown, PathProjection, TimerIntent, TimerShown};
use openpaint_ui::ControlMap;

use super::HomePart;
use super::tests::{COLUMN, paths};
use crate::palette::VERMILION;
use crate::passes::{assert_on_screen, context, drawn, outlined, pass, settled, tap, words};
use crate::text::{self as t, Language};

/// The timer with `block` in hand and `sealed` blocks today, 25 minutes chosen.
fn timer(block: Option<BlockShown>, sealed: u8) -> TimerShown {
    TimerShown {
        length: Length::TwentyFive,
        chime: true,
        sealed,
        block,
    }
}

/// The column's part between passes and the context they share, as a frame loop's.
struct Home {
    part: HomePart,
    ctx: egui::Context,
}

impl Home {
    fn new() -> Self {
        Self {
            part: HomePart::default(),
            ctx: context(),
        }
    }

    /// One pass of the column at `time` with `events`: the controls, the shapes drawn and the
    /// timer's taps.
    fn column(
        &mut self,
        (path, timer): (&PathProjection, &TimerShown),
        (time, language, events): (f64, Language, Vec<egui::Event>),
    ) -> (ControlMap, Vec<egui::Shape>, Vec<TimerIntent>) {
        let mut timed = Vec::new();
        let part = &mut self.part;
        let (map, shapes) = drawn(&self.ctx, (time, language, events), |ui| {
            let (mut taps, mut asked) = (Vec::new(), Vec::new());
            let taps = ((&mut taps, &mut asked), &mut timed);
            part.column(ui, COLUMN, (path, timer, true), taps);
        });
        (map, shapes, timed)
    }

    /// The timer's intents a tap on `name` sends, from a pass at `time`.
    fn tapped(
        &mut self,
        shown: (&PathProjection, &TimerShown),
        name: &str,
        time: f64,
    ) -> Vec<TimerIntent> {
        let (map, _, _) = self.column(shown, (time, Language::English, Vec::new()));
        let [press, release] = tap(&map, name);
        let mut timed = self.column(shown, (time + 0.1, Language::English, press)).2;
        timed.extend(
            self.column(shown, (time + 0.2, Language::English, release))
                .2,
        );
        timed
    }
}

#[test]
fn the_row_offers_the_lengths_and_start_timer() {
    let (_, going) = paths();
    let idle = timer(None, 0);
    let mut home = Home::new();
    for (at, language) in [(1.0, Language::English), (2.0, Language::Chinese)] {
        let (map, shapes, _) = home.column((&going, &idle), (at, language, Vec::new()));
        for name in ["15", "25", "40", t::START_TIMER.id(), t::START.id()] {
            assert_on_screen(&map, name);
        }
        assert!(map.find(t::CONTINUE.id()).is_err());
        let label = t::POMODORO.get(language);
        assert!(words(&shapes).iter().any(|word| word == label), "{label}");
    }
    let asked = home.tapped((&going, &idle), "40", 10.0);
    assert_eq!(asked, [TimerIntent::Length(Length::Forty)]);
    let asked = home.tapped((&going, &idle), t::START_TIMER.id(), 20.0);
    assert_eq!(asked, [TimerIntent::Start]);
    assert!(
        home.part.opening.is_some(),
        "the sheet grows out of Start timer"
    );
}

#[test]
fn under_way_the_row_shows_what_is_left_and_continue() {
    let (_, going) = paths();
    let mut home = Home::new();
    let running = |paused| BlockShown::Running {
        left: 18,
        share: 0.72,
        paused,
    };
    let states = [
        (running(false), ["18 min", "18 分"]),
        (running(true), ["Paused", "已暂停"]),
        (BlockShown::Up, ["Time", "到点"]),
    ];
    for (place, (block, shown)) in states.into_iter().enumerate() {
        let under_way = timer(Some(block), 0);
        for (language, word) in [Language::English, Language::Chinese]
            .into_iter()
            .zip(shown)
        {
            let at = 10.0 * place as f64 + f64::from(u8::from(language == Language::Chinese));
            let (map, shapes, _) = home.column((&going, &under_way), (at, language, Vec::new()));
            assert_on_screen(&map, t::CONTINUE.id());
            assert!(map.find(t::START_TIMER.id()).is_err() && map.find("25").is_err());
            assert!(words(&shapes).iter().any(|w| w == word), "{word}");
        }
    }
    let up = timer(Some(BlockShown::Up), 0);
    let (_, shapes, _) = home.column((&going, &up), (40.0, Language::English, Vec::new()));
    assert_eq!(
        outlined(&shapes, VERMILION),
        1,
        "the ring in the seal's red"
    );
    let asked = home.tapped((&going, &up), t::CONTINUE.id(), 50.0);
    assert_eq!(asked, [TimerIntent::Start]);
}

/// The home's column for a screen `width` points wide, as the home lays it (600 at most, 0.42 of
/// the screen).
fn column_for(width: f32) -> Rect {
    let wide = 600.0_f32.min(0.42 * width);
    Rect::from_min_size(COLUMN.min, egui::vec2(wide, COLUMN.height()))
}

#[test]
fn the_row_s_controls_never_overlap_nor_leave_the_column_at_any_width() {
    let (_, going) = paths();
    let idle = timer(None, 0);
    let held = timer(
        Some(BlockShown::Running {
            left: 18,
            share: 0.72,
            paused: true,
        }),
        0,
    );
    let ctx = context();
    let mut time = 0.0;
    // A third and a half of the tablet's landscape, the tablet upright, the tablet.
    for width in [549.0, 800.0, 823.0, 1029.0, 1646.0] {
        let column = column_for(width);
        for language in [Language::English, Language::Chinese] {
            for (shown, names) in [
                (&idle, &["15", "25", "40", t::START_TIMER.id()][..]),
                (&held, &[t::CONTINUE.id()][..]),
            ] {
                time += 1.0;
                let mut part = HomePart::default();
                let (map, _) = drawn(&ctx, (time, language, Vec::new()), |ui| {
                    let (mut taps, mut asked, mut timed) = (Vec::new(), Vec::new(), Vec::new());
                    let taps = ((&mut taps, &mut asked), &mut timed);
                    part.column(ui, column, (&going, shown, true), taps);
                });
                let ppp = ctx.pixels_per_point();
                let rects: Vec<[f32; 4]> = (names.iter())
                    .map(|name| map.find(name).expect("on the row").rect)
                    .collect();
                for (name, [left, _, right, _]) in names.iter().zip(&rects) {
                    let inside = *left >= column.left() * ppp && *right <= column.right() * ppp;
                    assert!(inside, "{name} within the column at {width} ({language:?})");
                }
                for (a, one) in rects.iter().enumerate() {
                    for (b, other) in rects.iter().enumerate().skip(a + 1) {
                        let apart = one[2] <= other[0]
                            || other[2] <= one[0]
                            || one[3] <= other[1]
                            || other[3] <= one[1];
                        let (a, b) = (names[a], names[b]);
                        assert!(apart, "{a} and {b} overlap at {width} ({language:?})");
                    }
                }
            }
        }
    }
}

#[test]
fn today_s_seals_show_at_the_header_s_right_only_when_there_is_one() {
    let (_, going) = paths();
    let mut home = Home::new();
    for (sealed, at) in [(0, 1.0), (2, 2.0)] {
        for language in [Language::English, Language::Chinese] {
            let shown = timer(None, sealed);
            let (_, shapes, _) = home.column((&going, &shown), (at, language, Vec::new()));
            let today = t::TODAY.get(language);
            let said = words(&shapes).iter().any(|word| word == today);
            assert_eq!(said, sealed > 0, "{today}");
            assert_eq!(outlined(&shapes, VERMILION), usize::from(sealed));
        }
    }
}

#[test]
fn the_practice_options_hold_the_break_chime() {
    let (_, going) = paths();
    let shown = timer(None, 0);
    let ctx = context();
    let mut part = HomePart::default();
    part.practice.open(Rect::from_min_size(
        egui::pos2(560.0, 56.0),
        egui::vec2(48.0, 48.0),
    ));
    let mut timed = Vec::new();
    let mut dialog = |ui: &mut egui::Ui, timed: &mut Vec<TimerIntent>| {
        let (mut taps, mut asked) = (Vec::new(), Vec::new());
        part.practice_dialog(ui.ctx(), (&going, &shown), (&mut taps, &mut asked, timed));
    };
    for (from, language) in [(10.0, Language::English), (30.0, Language::Chinese)] {
        let map = settled(&ctx, (from, language), |ui| dialog(ui, &mut timed));
        assert_on_screen(&map, t::BREAK_CHIME.id());
    }
    let map = settled(&ctx, (50.0, Language::English), |ui| dialog(ui, &mut timed));
    let [press, release] = tap(&map, t::BREAK_CHIME.id());
    for (time, events) in [(53.0, press), (53.1, release)] {
        pass(&ctx, (time, Language::English, events), |ui| {
            dialog(ui, &mut timed)
        });
    }
    assert_eq!(timed, [TimerIntent::Chime(false)]);
}
