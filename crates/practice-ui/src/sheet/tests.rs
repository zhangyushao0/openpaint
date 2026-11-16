//! One headless pass of each card the path added on the sheet, on fixtures: a result's row where
//! the app judges and where the eye does, a lesson's end and the first lesson's How, their
//! controls registered by name and on the screen.

use openpaint_practice::{
    Answer, CopyRef, Judge, Offer, Outcome, PairAsked, Seal, Skill, SkillLevel, Verdict,
};
use openpaint_practice_app::{End, Judged, OneMore, PairShown, PracticeIntent};

use super::end::{EndTap, end_card};
use super::first::first_how;
use super::result::result_row;
use crate::passes::{assert_on_screen, context, pass, settled, tap};
use crate::text::{self as t, Language};

const RESULTS: [&str; 3] = [t::ACCURATE.id(), t::ALMOST.id(), t::REDO_RESULT.id()];

#[test]
fn the_app_s_result_is_ringed_and_the_eye_s_three_are_tapped() {
    let ctx = context();
    let mut taps: Vec<PracticeIntent> = Vec::new();
    let app = Judged {
        verdict: Verdict::App(Outcome::Near),
        result: Some((Outcome::Near, Judge::App)),
    };
    for language in [Language::English, Language::Chinese] {
        let map = pass(&ctx, (1.0, language, Vec::new()), |ui| {
            result_row(ui, app, &mut taps);
        });
        RESULTS.iter().for_each(|name| assert_on_screen(&map, name));
    }
    let eye = Judged {
        verdict: Verdict::Eye,
        result: None,
    };
    let map = pass(&ctx, (2.0, Language::English, Vec::new()), |ui| {
        result_row(ui, eye, &mut taps);
    });
    RESULTS.iter().for_each(|name| assert_on_screen(&map, name));
    assert!(taps.is_empty(), "the app's result is no button");
    let [press, release] = tap(&map, t::REDO_RESULT.id());
    for (time, events) in [(2.1, press), (2.2, release)] {
        pass(&ctx, (time, Language::English, events), |ui| {
            result_row(ui, eye, &mut taps);
        });
    }
    assert_eq!(taps, [PracticeIntent::Judge(Outcome::Redo)]);
    let incomplete = Judged {
        verdict: Verdict::Incomplete,
        result: None,
    };
    let map = pass(&ctx, (3.0, Language::English, Vec::new()), |ui| {
        result_row(ui, incomplete, &mut taps);
    });
    assert!(
        map.find(t::ACCURATE.id()).is_err(),
        "nothing while no verdict is honest"
    );
}

#[test]
fn a_lesson_s_end_offers_one_more_and_that_s_all_for_today() {
    let ctx = context();
    let skill = Skill::of_key("line").expect("lines");
    let end = End {
        results: vec![
            (0, Some(Outcome::Clean)),
            (1, Some(Outcome::Near)),
            (2, None),
        ],
        exercises: 3,
        seconds: 192,
        skill: Some((skill, 1, 2)),
        sealed: vec![Seal::TenLessons],
        next: Some(OneMore::Skill(Offer {
            skill,
            level: SkillLevel::Third,
            exercises: 6,
            minutes: 4,
            redo: false,
        })),
        pairs: Vec::new(),
        drill: None,
    };
    let mut tapped = None;
    for (from, language) in [(10.0, Language::English), (30.0, Language::Chinese)] {
        let map = settled(&ctx, (from, language), |ui| {
            tapped = tapped.or(end_card(ui, &end, (t::skill_name(skill), None, &[])));
        });
        assert_on_screen(&map, t::ONE_MORE.id());
        assert_on_screen(&map, t::ENOUGH.id());
        let [press, release] = tap(&map, t::ENOUGH.id());
        for (time, events) in [(from + 3.0, press), (from + 3.1, release)] {
            pass(&ctx, (time, language, events), |ui| {
                tapped = tapped.or(end_card(ui, &end, (t::skill_name(skill), None, &[])));
            });
        }
        assert_eq!(tapped.take(), Some(EndTap::Enough));
    }
    let last = End { next: None, ..end };
    let map = pass(&ctx, (60.0, Language::English, Vec::new()), |ui| {
        end_card(ui, &last, (t::skill_name(skill), None, &[]));
    });
    assert!(
        map.find(t::ONE_MORE.id()).is_err(),
        "nothing left to practise"
    );
}

#[test]
fn a_lesson_s_end_asks_a_blind_pair_once_and_offers_a_drill_on_a_fall() {
    let ctx = context();
    let skill = Skill::of_key("shape").expect("shapes");
    let copy = |sheet, exercise| CopyRef { sheet, exercise };
    let pair = PairShown {
        asked: PairAsked {
            older: copy(3, 1),
            newer: copy(9, 0),
        },
        older_left: false,
        answer: None,
        skill: Some(skill),
    };
    let offer = Offer {
        skill,
        level: SkillLevel::Second,
        exercises: 4,
        minutes: 8,
        redo: false,
    };
    let end = End {
        results: vec![(0, Some(Outcome::Near))],
        exercises: 1,
        seconds: 300,
        skill: Some((skill, 1, 1)),
        sealed: Vec::new(),
        next: None,
        pairs: vec![pair],
        drill: Some(offer),
    };
    let name = t::skill_name(skill);
    let mut tapped = None;
    let map = settled(&ctx, (10.0, Language::English), |ui| {
        tapped = tapped.or(narrow(ui, |ui| end_card(ui, &end, (name, None, &[]))));
    });
    for asked in [t::LEFT_BETTER, t::ALIKE, t::RIGHT_BETTER, t::START] {
        assert_on_screen(&map, asked.id());
    }
    let [press, release] = tap(&map, t::LEFT_BETTER.id());
    for (time, events) in [(13.0, press), (13.1, release)] {
        pass(&ctx, (time, Language::English, events), |ui| {
            tapped = tapped.or(narrow(ui, |ui| end_card(ui, &end, (name, None, &[]))));
        });
    }
    assert_eq!(tapped.take(), Some(EndTap::Pair(0, Answer::Left)));
    let answered = End {
        pairs: vec![PairShown {
            answer: Some(Answer::Left),
            ..pair
        }],
        drill: None,
        ..end
    };
    let map = settled(&ctx, (20.0, Language::Chinese), |ui| {
        narrow(ui, |ui| end_card(ui, &answered, (name, None, &[])));
    });
    assert!(map.find(t::LEFT_BETTER.id()).is_err(), "answered once");
    assert!(map.find(t::START.id()).is_err(), "no drill without a fall");
}

#[test]
fn the_first_lesson_s_how_ends_with_got_it() {
    let ctx = context();
    for language in [Language::English, Language::Chinese] {
        let map = pass(&ctx, (1.0, language, Vec::new()), |ui| {
            first_how(ui);
        });
        assert_on_screen(&map, t::GOT_IT.id());
    }
}

/// `add` in a column as wide as a card's.
fn narrow<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    ui.allocate_ui(egui::Vec2::new(296.0, 860.0), add).inner
}
