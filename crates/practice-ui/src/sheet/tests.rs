//! One headless pass of each card the path added on the sheet, on fixtures: a result's row where
//! the app judges and where the eye does, a lesson's end and the first lesson's How, their
//! controls registered by name and on the screen.

use openpaint_practice::{Judge, Offer, Outcome, Seal, Skill, SkillLevel, Verdict};
use openpaint_practice_app::{End, Judged, PracticeIntent};

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
        sealed: vec![Seal::TenSessions],
        next: Some(Offer {
            skill,
            level: SkillLevel::Third,
            exercises: 6,
            minutes: 4,
            redo: false,
        }),
    };
    let mut tapped = None;
    for (from, language) in [(10.0, Language::English), (30.0, Language::Chinese)] {
        let map = settled(&ctx, (from, language), |ui| {
            tapped = tapped.or(end_card(ui, &end, t::skill_name(skill)));
        });
        assert_on_screen(&map, t::ONE_MORE.id());
        assert_on_screen(&map, t::ENOUGH.id());
        let [press, release] = tap(&map, t::ENOUGH.id());
        for (time, events) in [(from + 3.0, press), (from + 3.1, release)] {
            pass(&ctx, (time, language, events), |ui| {
                tapped = tapped.or(end_card(ui, &end, t::skill_name(skill)));
            });
        }
        assert_eq!(tapped.take(), Some(EndTap::Enough));
    }
    let last = End { next: None, ..end };
    let map = pass(&ctx, (60.0, Language::English, Vec::new()), |ui| {
        end_card(ui, &last, t::skill_name(skill));
    });
    assert!(
        map.find(t::ONE_MORE.id()).is_err(),
        "nothing left to practise"
    );
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
