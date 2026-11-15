//! The words a compared exercise's card says, by the keys its comparison names them by (S109,
//! `openpaint_practice::Say`): each key read in the language shown from the tables beside this.

use openpaint_practice::{Arg, Missed, Reach, Roundness, Say, Side, Tilt};

use super::{Language, Text};

/// What `say` reads as in `language`; its key itself where no table knows it.
pub(crate) fn said(language: Language, say: &Say) -> String {
    let count = |at: usize| match say.args.get(at) {
        Some(Arg::Count(count)) => *count,
        _ => 0,
    };
    let word = |at: usize| match say.args.get(at) {
        Some(Arg::Word(word)) => *word,
        _ => "",
    };
    let plain = |text: Text| text.get(language).to_owned();
    match say.key {
        "off" => super::off(language, (count(0), count(1), count(2))),
        "ellipse-differs" => ellipse(language, say),
        "reason" => super::reason(language, say),
        "cells-uneven" => super::cells_uneven(language, count(0)),
        "lightness-off" => super::lightness_off(language, count(0), word(1) == "lighter"),
        "not-toward" => super::not_toward(language, count(0)),
        "merged" => super::merged(language, count(0), count(1)),
        "no-edges" => plain(super::NO_EDGES),
        "no-mark" => plain(super::NO_MARK),
        "no-solid" => plain(super::NO_SOLID),
        "no-object" => plain(super::NO_OBJECT),
        "no-ellipse" => plain(super::NO_ELLIPSE),
        "ellipse-fits" => plain(super::ELLIPSE_FITS),
        "not-painted" => plain(super::NOT_PAINTED),
        "no-outline" => plain(super::NO_OUTLINE),
        "analysis-strongest" => plain(super::ANALYSIS_STRONGEST),
        "analysis-strongest-how" => plain(super::ANALYSIS_STRONGEST_HOW),
        "analysis-by" => plain(super::ANALYSIS_BY),
        "analysis-second" => plain(super::ANALYSIS_SECOND),
        "analysis-second-how" => plain(super::ANALYSIS_SECOND_HOW),
        "analysis-spread" => plain(super::ANALYSIS_SPREAD),
        "analysis-look" => plain(super::ANALYSIS_LOOK),
        "analysis-chosen" => super::analysis_chosen(language, &words(say)),
        "analysis-spread-chosen" => super::analysis_spread(language, words(say).first().copied()),
        key => key.to_owned(),
    }
}

/// `say`'s words, a count an empty word.
fn words(say: &Say) -> Vec<&'static str> {
    (say.args.iter())
        .map(|arg| match arg {
            Arg::Word(word) => *word,
            Arg::Count(_) => "",
        })
        .collect()
}

/// An ellipse's difference from its words: its roundness, its tilt, then each side missed and
/// which way.
fn ellipse(language: Language, say: &Say) -> String {
    let words = words(say);
    let roundness = match words.first() {
        Some(&"rounder") => Some(Roundness::Rounder),
        Some(&"flatter") => Some(Roundness::Flatter),
        _ => None,
    };
    let tilt = match words.get(1) {
        Some(&"clockwise") => Some(Tilt::Clockwise),
        Some(&"anticlockwise") => Some(Tilt::Anticlockwise),
        _ => None,
    };
    let sides: Vec<Missed> = (words.get(2..).unwrap_or_default().chunks(2))
        .filter_map(|pair| {
            let side = match *pair.first()? {
                "top" => Side::Top,
                "right" => Side::Right,
                "bottom" => Side::Bottom,
                _ => Side::Left,
            };
            let reach = if pair.get(1) == Some(&"short") {
                Reach::Short
            } else {
                Reach::Past
            };
            Some(Missed {
                side,
                reach,
                middle: [0.0, 0.0],
            })
        })
        .collect();
    super::ellipse_differs(language, (roundness, tilt), &sides)
}
