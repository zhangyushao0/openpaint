//! A free scribble's words (S122): the first lesson's first exercise. The skeleton's least, its
//! name under its key; S122c: its How and the first lesson's card.

use super::{How, KindWords, words};

pub(super) const WORDS: [KindWords; 1] = [words("free", (FREE_EXERCISE, FREE_COUNTED), FREE)];

/// No demonstration: nothing is copied.
const FREE: How = How {
    heading: FREE_EXERCISE,
    trains: None,
    steps: &[],
    says: &[],
};

texts! { PRACTICE_FREE;
    FREE_EXERCISE = "Free scribble", "随手画";
    FREE_COUNTED = "Free scribbles", "随手画";
}
