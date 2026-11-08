//! Each kind's words, a file a kind (S109b), in V大's terms as `text/mod.rs` says: its name (its
//! tile, its drill, the bar), its line in a session's summary, its How card (S92: its title, one
//! line on what it trains and its week, its steps, one per step of its demonstration, Compare the
//! last, and once what Compare's marks mean, D108), for a value scale the steps made for the row
//! in hand, and the head of its rows in the summary. Each kind is registered here by its line in
//! `KINDS`, its families found by the key its description gives them
//! (`openpaint_practice::Facts::key`, S109). The words a kind's card, pens and notices say beside
//! these stay with it.

use openpaint_practice::Family;

use super::{Language, Text};

mod boxes;
mod colours;
mod combinations;
mod combined_lines;
mod divisions;
mod ellipses;
mod estimates;
mod free_boxes;
mod key_shifts;
mod lines;
mod objects;
mod points_in_space;
mod scales;
mod shapes;
mod solids;
mod study;
mod value_masses;
mod whole_copies;

pub(crate) use key_shifts::{
    GREY_VIEW, KEY_SHIFTS_STAGE, LIGHTNESS_LEFT, MERGED_PAIRS, NOT_TOWARD, ORIGINAL_LOCKED,
    TENDENCY, lightness_off, merged, not_toward,
};
pub(crate) use study::STUDY_EXERCISE;
pub(crate) use value_masses::{AT_ITS_VALUE, DARK_PEN, DRILL_NEEDS_MASSES, LIGHT_PEN, MIDDLE_PEN};

/// A kind's How card's words.
#[derive(Debug, Clone, Copy)]
pub(crate) struct How {
    pub(crate) heading: Text,
    /// What it trains and its week; none for the colour copy, which has no demonstration.
    pub(crate) trains: Option<Text>,
    /// One per step of its demonstration (a value scale's own Compare only: its others are made
    /// for the row in hand, `how_steps`).
    pub(crate) steps: &'static [Text],
    /// What Compare shows and how to correct: the card's paragraphs under the steps.
    pub(crate) says: &'static [Text],
}

/// A family's words.
pub(crate) struct KindWords {
    /// Its family's key (`Facts::key`).
    pub(crate) key: &'static str,
    /// Its name: "Line", "Centre".
    pub(crate) name: Text,
    /// What the summary counts: "Lines".
    pub(crate) counted: Text,
    pub(crate) how: How,
    /// Its How's steps made for the exercise in hand before its own (a value scale's row).
    pub(crate) made: Option<fn(Language, usize) -> Vec<String>>,
    /// The head of its rows in a session's summary (D108), where it has rows.
    pub(crate) rows: Option<Text>,
}

/// A family's words, nothing made, no rows.
const fn words(key: &'static str, (name, counted): (Text, Text), how: How) -> KindWords {
    KindWords {
        key,
        name,
        counted,
        how,
        made: None,
        rows: None,
    }
}

texts! { PRACTICE_HOW;
    AGAIN = "Again", "再看一遍";
    COMPARE_STEP = "Compare", "叠图对比";
}

/// Every kind's families' words: its line here registers them.
const KINDS: [&[KindWords]; 18] = [
    &lines::WORDS,
    &combined_lines::WORDS,
    &shapes::WORDS,
    &boxes::WORDS,
    &free_boxes::WORDS,
    &ellipses::WORDS,
    &estimates::WORDS,
    &points_in_space::WORDS,
    &solids::WORDS,
    &objects::WORDS,
    &divisions::WORDS,
    &whole_copies::WORDS,
    &colours::WORDS,
    &combinations::WORDS,
    &scales::WORDS,
    &study::WORDS,
    &value_masses::WORDS,
    &key_shifts::WORDS,
];

/// `family`'s words, by its key.
pub(crate) fn kind_words(family: Family) -> &'static KindWords {
    let key = family.facts().key;
    (KINDS.iter().flat_map(|words| words.iter()))
        .find(|words| words.key == key)
        .unwrap_or(&lines::WORDS[0])
}

/// `family`'s How card's numbered steps as `language` reads them, one per step of its
/// demonstration: those made for the `cells` of a value scale's row in hand (S97), then the
/// card's own, Compare last.
pub(crate) fn how_steps(family: Family, cells: usize, language: Language) -> Vec<String> {
    let words = kind_words(family);
    let mut steps = words
        .made
        .map(|made| made(language, cells))
        .unwrap_or_default();
    steps.extend(
        words
            .how
            .steps
            .iter()
            .map(|step| step.get(language).to_owned()),
    );
    steps
}

/// `family`'s How card.
pub(crate) fn how(family: Family) -> How {
    kind_words(family).how
}

/// Every kind's table, for the tables' tests.
#[cfg(test)]
pub(crate) fn tables() -> Vec<&'static [Text]> {
    vec![
        PRACTICE_HOW,
        boxes::PRACTICE_BOXES,
        colours::PRACTICE_COLOURS,
        combinations::PRACTICE_COMBINATIONS,
        combined_lines::PRACTICE_COMBINED_LINES,
        divisions::PRACTICE_DIVISIONS,
        ellipses::PRACTICE_ELLIPSES,
        estimates::PRACTICE_ESTIMATES,
        free_boxes::PRACTICE_FREE_BOXES,
        key_shifts::PRACTICE_KEY_SHIFTS,
        lines::PRACTICE_LINES,
        objects::PRACTICE_OBJECTS_HOW,
        points_in_space::PRACTICE_POINTS_IN_SPACE,
        scales::PRACTICE_SCALES,
        shapes::PRACTICE_SHAPES,
        solids::PRACTICE_SOLIDS_HOW,
        study::PRACTICE_STUDY,
        value_masses::PRACTICE_VALUE_MASSES,
        whole_copies::PRACTICE_WHOLE_COPIES,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_family_has_its_words_under_its_key() {
        for family in Family::ALL {
            assert_eq!(kind_words(family).key, family.facts().key, "{family:?}");
        }
        let count: usize = KINDS.iter().map(|words| words.len()).sum();
        assert_eq!(count, Family::ALL.len());
    }
}
