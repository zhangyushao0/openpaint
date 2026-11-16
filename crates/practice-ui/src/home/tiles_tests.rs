//! The unit in hand's tiles on the home's column in every state the home can be in (S156,
//! S158h): every skill shown inside the column and above its foot, four a row in both languages
//! where a unit holds no more, on the tests' screen and the tablet's.

use egui::Rect;
use openpaint_practice::Skill;
use openpaint_practice_app::{BlockShown, LessonRow, OpenLesson, PathProjection, TimerShown};

use super::HomePart;
use super::tests::{COLUMN, paths, retests_due};
use crate::passes::{SCREEN, TABLET, context, pass_on};
use crate::text::{self as t, Language};

/// The column on the tablet in landscape, as the home lays it (600 points wide, from under the
/// band to a card's gap above the screen's foot).
fn tablet_column() -> Rect {
    Rect::from_min_max(
        egui::pos2(32.0, 56.0),
        egui::pos2(632.0, TABLET.max.y - 32.0),
    )
}

/// A Pomodoro's block under way.
fn running() -> TimerShown {
    TimerShown {
        block: Some(BlockShown::Running {
            left: 18,
            share: 0.72,
            paused: false,
        }),
        ..TimerShown::default()
    }
}

/// The home's states, each named: a fresh learner; a next lesson with a review waiting; a unit's
/// start re-test in its place with the review and the end re-test; a lesson under way with the
/// review, the end re-test and a Pomodoro running.
fn states() -> Vec<(&'static str, PathProjection, TimerShown)> {
    let (fresh, going) = paths();
    let due = retests_due();
    let open = OpenLesson {
        sheet: openpaint_app::ProjectId::new(7),
        row: LessonRow {
            slot: 0,
            number: 3,
            skill: Skill::of_key("box"),
            first: false,
            retest: None,
        },
        place: (2, 5),
    };
    let under_way = PathProjection {
        open: Some(open),
        ..due.clone()
    };
    vec![
        ("fresh", fresh, TimerShown::default()),
        ("a review", going, TimerShown::default()),
        ("both re-tests", due.clone(), TimerShown::default()),
        ("both re-tests, a block", due, running()),
        ("under way, a block", under_way, running()),
    ]
}

/// `path` with every pack's pictures kept: nothing lacking.
fn packed(path: &PathProjection) -> PathProjection {
    let mut packed = path.clone();
    for unit in &mut packed.units {
        for skill in &mut unit.skills {
            skill.lacks = None;
        }
    }
    packed
}

/// Every skill of the unit in hand shows on the home: four a row where they fit, the sheet
/// longer while they need it, pressed if the column is short (a smaller glyph, then none, then
/// what the pictures lack unsaid), all inside it and above its foot; units 1 to 8 in every state,
/// without packs and with them, both languages, the tests' screen and the tablet's. A unit of
/// four or fewer lies in one row.
#[test]
fn the_home_shows_every_skill_of_the_unit_in_hand() {
    let mut home = (context(), HomePart::default(), 0.0);
    for (state, path, timer) in states() {
        for path in [path.clone(), packed(&path)] {
            for screen in [(SCREEN, COLUMN), (TABLET, tablet_column())] {
                for unit in 1..=8_u8 {
                    let path = PathProjection {
                        unit,
                        ..path.clone()
                    };
                    for language in [Language::English, Language::Chinese] {
                        let at = format!("{state}, unit {unit}, {language:?}, {:?}", screen.1);
                        every_tile(&mut home, (&path, &timer), (screen, language), &at);
                    }
                }
            }
        }
    }
}

/// One pass of the column for `path` and `timer` in `column` on `screen`: the foot inside it,
/// every tile of the unit in hand inside it and above the foot, a unit of four or fewer in one
/// row.
fn every_tile(
    (ctx, part, time): &mut (egui::Context, HomePart, f64),
    (path, timer): (&PathProjection, &TimerShown),
    ((screen, column), language): ((Rect, Rect), Language),
    at: &str,
) {
    *time += 1.0;
    let map = pass_on(ctx, screen, (*time, language, Vec::new()), |ui| {
        let (mut taps, mut asked, mut timed) = (Vec::new(), Vec::new(), Vec::new());
        let taps = ((&mut taps, &mut asked), &mut timed);
        part.column(ui, column, (path, timer, true), taps);
    });
    let rect = |name: &str| {
        let found = (map.find(name)).unwrap_or_else(|why| panic!("{at}: {name}: {why:?}"));
        let [left, top, right, bottom] = found.rect;
        Rect::from_min_max(egui::pos2(left, top), egui::pos2(right, bottom))
    };
    let foot = rect(t::PRACTICE_BOOK.id());
    assert!(column.contains_rect(foot), "{at}: the foot {foot:?}");
    let skills = &path.units[usize::from(path.unit - 1)].skills;
    assert!(!skills.is_empty(), "{at}: the unit holds skills");
    let tiles: Vec<Rect> = (skills.iter())
        .map(|shown| rect(t::skill_name(shown.skill).id()))
        .collect();
    for tile in &tiles {
        assert!(
            column.contains_rect(*tile) && tile.bottom() < foot.top(),
            "{at}: a tile at {tile:?}, the foot at {foot:?}"
        );
    }
    if tiles.len() <= 4 {
        let row = tiles[0].top();
        let one = tiles.iter().all(|tile| (tile.top() - row).abs() < 0.5);
        assert!(one, "{at}: not one row: {tiles:?}");
    }
}
