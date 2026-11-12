//! Each kind's words, a file a kind (S109b), in V大's terms as `text/mod.rs` says: its name (the
//! bar, a compared exercise's heading where no skill names it), its How card (S92: its title, one
//! line on what it trains, its steps, one per step of its demonstration, Compare the
//! last, and once what Compare's marks mean, D108), and for a value scale the steps made for the
//! row in hand. Each kind is registered here by its line in `KINDS`, its topics found by the key
//! its description gives them (`openpaint_practice::Facts::key`, S109): the screens ask a kind's
//! words by the kind in hand (S122). The words a kind's card, pens and notices say beside these
//! stay with it.

use openpaint_practice::Kind;

use super::{Language, Text};

mod boxes;
mod colours;
mod combinations;
mod combined_lines;
mod divisions;
mod ellipses;
mod estimates;
mod free;
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
    GREY_VIEW, ORIGINAL_LOCKED, TENDENCY, lightness_off, merged, not_toward,
};
pub(crate) use study::STUDY_EXERCISE;
pub(crate) use value_masses::{DARK_PEN, LIGHT_PEN, MIDDLE_PEN};

/// A kind's How card's words.
#[derive(Debug, Clone, Copy)]
pub(crate) struct How {
    pub(crate) heading: Text,
    /// What it trains; none for the colour copy, which has no demonstration.
    pub(crate) trains: Option<Text>,
    /// One per step of its demonstration (a value scale's own Compare only: its others are made
    /// for the row in hand, `how_steps`).
    pub(crate) steps: &'static [Text],
    /// What Compare shows and how to correct: the card's paragraphs under the steps.
    pub(crate) says: &'static [Text],
}

/// A topic's words.
pub(crate) struct KindWords {
    /// Its key (`Facts::key`).
    pub(crate) key: &'static str,
    /// Its name: "Line", "Centre".
    pub(crate) name: Text,
    pub(crate) how: How,
    /// Its How's steps made for the exercise in hand before its own (a value scale's row).
    pub(crate) made: Option<fn(Language, usize) -> Vec<String>>,
}

/// A topic's words, nothing made.
const fn words(key: &'static str, name: Text, how: How) -> KindWords {
    KindWords {
        key,
        name,
        how,
        made: None,
    }
}

texts! { PRACTICE_HOW;
    WATCH_AGAIN = "Watch again", "再看一遍";
    COMPARE_STEP = "Compare", "叠图对比";
}

/// Every kind's topics' words: its line here registers them.
const KINDS: [&[KindWords]; 19] = [
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
    &free::WORDS,
];

/// `kind`'s words, by its topic's key.
pub(crate) fn kind_words(kind: Kind) -> &'static KindWords {
    let key = kind.facts().key;
    (KINDS.iter().flat_map(|words| words.iter()))
        .find(|words| words.key == key)
        .unwrap_or(&lines::WORDS[0])
}

/// `kind`'s How card's numbered steps as `language` reads them, one per step of its
/// demonstration: those made for the `cells` of a value scale's row in hand (S97), then the
/// card's own, Compare last.
pub(crate) fn how_steps(kind: Kind, cells: usize, language: Language) -> Vec<String> {
    let words = kind_words(kind);
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

/// `kind`'s How card.
pub(crate) fn how(kind: Kind) -> How {
    kind_words(kind).how
}

/// Every kind there is, each variant once, by its stored code: what the tables' tests walk.
#[cfg(test)]
pub(crate) fn every_kind() -> Vec<Kind> {
    (1..=u8::MAX)
        .flat_map(|code| (0..=u8::MAX).filter_map(move |variant| Kind::of_code(code, variant)))
        .collect()
}

/// A kind of the topic `key`, for the tests.
#[cfg(test)]
pub(crate) fn kind_of(key: &str) -> Kind {
    (every_kind().into_iter())
        .find(|kind| kind.facts().key == key)
        .unwrap_or(Kind::Free)
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
        free::PRACTICE_FREE,
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
    fn every_kind_has_its_words_under_its_key_and_every_words_a_kind() {
        let kinds = every_kind();
        for kind in &kinds {
            assert_eq!(kind_words(*kind).key, kind.facts().key, "{kind:?}");
        }
        for words in KINDS.iter().flat_map(|words| words.iter()) {
            assert!(
                kinds.iter().any(|kind| kind.facts().key == words.key),
                "{}",
                words.key
            );
        }
    }
}
