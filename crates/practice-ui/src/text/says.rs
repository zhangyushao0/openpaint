//! The words a compared exercise's card says, by the keys its comparison names them by (S109,
//! `openpaint_practice::Say`): each key read in the language shown from the tables beside this.
//! Keys are never stored; a box's counts and an ellipse's findings went with their checks (S145).

use openpaint_practice::{Arg, Say};

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
        "reason" => super::reason(language, say),
        "cells-uneven" => super::cells_uneven(language, count(0)),
        "lightness-off" => super::lightness_off(language, count(0), word(1) == "lighter"),
        "not-toward" => super::not_toward(language, count(0)),
        "merged" => super::merged(language, count(0), count(1)),
        "no-edges" => plain(super::NO_EDGES),
        "no-mark" => plain(super::NO_MARK),
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
