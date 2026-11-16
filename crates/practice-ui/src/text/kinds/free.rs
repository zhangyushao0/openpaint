//! The first lesson's drawing's words (S122, S151): its first exercise, something in front of the
//! learner drawn in one frame. Its name under its key; its How is the first lesson's card.

use super::{How, KindWords, words};

pub(super) const WORDS: [KindWords; 1] = [words("free", FREE_EXERCISE, FREE)];

/// No demonstration: nothing is copied.
const FREE: How = How {
    heading: FREE_EXERCISE,
    trains: None,
    steps: &[],
    says: &[],
};

texts! { PRACTICE_FREE;
    FREE_EXERCISE = "Something in front of you", "眼前的东西";
}
