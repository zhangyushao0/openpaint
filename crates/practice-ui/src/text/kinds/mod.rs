//! Each kind's words, a file a kind (S109b), in V大's terms as `text/mod.rs` says: its name (the
//! bar, a compared exercise's heading where no skill names it), its How card (S92: its title, one
//! line on what it trains, its steps, one per step of its demonstration, Compare the last, then
//! how to practise, never what Compare draws, D118), and for a value scale the steps made for the
//! row in hand. Each kind is registered here by its line in `KINDS`, its topics found by the key
//! its description gives them (`openpaint_practice::Facts::key`, S109): the screens ask a kind's
//! words by the kind in hand (S122). The words a kind's card, pens and notices say beside these
//! stay with it.

use openpaint_practice::Kind;

use super::{Language, Text};

mod abstract_analysis;
mod arrows;
mod boxes;
mod cast_shadows;
mod cold_warm;
mod colours;
mod combinations;
mod combined_lines;
mod contours;
mod contrast_thumbnails;
mod curved_rotations;
mod divisions;
mod ellipses;
mod estimates;
mod free;
mod free_boxes;
mod gamut_masks;
mod ghosted_lines;
mod intersections;
mod key_shifts;
mod light_on_form;
mod lines;
mod lit_colours;
mod objects;
mod observation;
mod placements;
mod points_in_space;
mod rotations;
mod scales;
mod shapes;
mod solids;
mod study;
mod textures;
mod value_masses;
mod whole_copies;

pub(crate) use abstract_analysis::{
    ANALYSIS_BY, ANALYSIS_LOOK, ANALYSIS_SECOND, ANALYSIS_SECOND_HOW, ANALYSIS_SPREAD,
    ANALYSIS_STRONGEST, ANALYSIS_STRONGEST_HOW, SECOND_MARK_PEN, STRONG_PEN, WEAK_PEN,
    analysis_chosen, analysis_spread, choice_word,
};
pub(crate) use cold_warm::{BIG_HOMEWORK, COLD_WARM_EXERCISE, LIGHTS_TENDENCY, SHADOWS_TENDENCY};
pub(crate) use contrast_thumbnails::{
    FOCUS_OUTSIDE, FOCUS_RIVAL, SECOND_STRONG, SECOND_WEAK, THUMBNAIL_BARE,
};
#[cfg(test)]
pub(crate) use gamut_masks::made as outside_mask_made;
pub(crate) use gamut_masks::outside_mask;
pub(crate) use key_shifts::{GREY_VIEW, TENDENCY, lightness_off, merged, not_toward};
pub(crate) use light_on_form::{DARKEST_PEN, light_off};
pub(crate) use lit_colours::{
    GROUND_NAME, LAMPLIGHT_NAME, LOCAL_COLOUR_NAME, ONE_ANSWER, SKYLIGHT_NAME, SUNLIGHT_NAME,
};
pub(crate) use observation::{LIFE_DRAWINGS, LIFE_EXERCISE, LIFE_NEW, life_subject};
pub(crate) use study::STUDY_EXERCISE;
pub(crate) use value_masses::{DARK_PEN, LIGHT_PEN, MIDDLE_PEN};

/// A kind's How card's words.
#[derive(Debug, Clone, Copy)]
pub(crate) struct How {
    pub(crate) heading: Text,
    /// What it trains; none for a kind without a demonstration (a colour copy, a drawing from
    /// life).
    pub(crate) trains: Option<Text>,
    /// One per step of its demonstration (a value scale's own Compare only: its others are made
    /// for the row in hand, `how_steps`).
    pub(crate) steps: &'static [Text],
    /// How to practise and correct: the card's paragraphs under the steps (D118: never what Compare
    /// draws, who judges or what the app does).
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
const KINDS: [&[KindWords]; 35] = [
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
    &abstract_analysis::WORDS,
    &rotations::WORDS,
    &observation::WORDS,
    &gamut_masks::WORDS,
    &placements::WORDS,
    &light_on_form::WORDS,
    &cast_shadows::WORDS,
    &curved_rotations::WORDS,
    &lit_colours::WORDS,
    &textures::WORDS,
    &contrast_thumbnails::WORDS,
    &cold_warm::WORDS,
    &arrows::WORDS,
    &contours::WORDS,
    &intersections::WORDS,
    &ghosted_lines::WORDS,
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
        abstract_analysis::PRACTICE_ANALYSIS,
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
        observation::PRACTICE_LIFE,
        points_in_space::PRACTICE_POINTS_IN_SPACE,
        rotations::PRACTICE_ROTATIONS,
        scales::PRACTICE_SCALES,
        shapes::PRACTICE_SHAPES,
        solids::PRACTICE_SOLIDS_HOW,
        study::PRACTICE_STUDY,
        value_masses::PRACTICE_VALUE_MASSES,
        whole_copies::PRACTICE_WHOLE_COPIES,
        gamut_masks::PRACTICE_GAMUT_MASKS,
        placements::PRACTICE_PLACEMENTS,
        light_on_form::PRACTICE_LIGHT_ON_FORM,
        cast_shadows::PRACTICE_CAST_SHADOWS,
        curved_rotations::PRACTICE_CURVED_ROTATIONS,
        lit_colours::PRACTICE_LIT_COLOURS,
        textures::PRACTICE_TEXTURES,
        contrast_thumbnails::PRACTICE_THUMBNAILS,
        cold_warm::PRACTICE_COLD_WARM,
        arrows::PRACTICE_ARROWS,
        contours::PRACTICE_CONTOURS,
        intersections::PRACTICE_INTERSECTIONS,
        ghosted_lines::PRACTICE_GHOSTED_LINES,
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
