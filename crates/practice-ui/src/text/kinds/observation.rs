//! A drawing from life's words (写生, S151): its name, its How card (no demonstration: how to
//! draw from life, the method only, never what the sheet does with the photo, D118), the path
//! page's card naming a subject by the unit shown, and its sheets' plural in the options.

use super::{How, KindWords, Text, words};

pub(super) const WORDS: [KindWords; 1] = [words("observation", LIFE_EXERCISE, LIFE_HOW)];

/// How to draw from life: the envelope first, the main directions and proportions by eye, key
/// points and negative shapes, then the parts (Proko's simplify, V大's order).
const LIFE_HOW: How = How {
    heading: LIFE_EXERCISE,
    trains: None,
    steps: &[],
    says: &[LIFE_SEAT, LIFE_ENVELOPE, LIFE_CHECK, LIFE_PARTS],
};

texts! { PRACTICE_LIFE;
    LIFE_EXERCISE = "Drawing from life", "写生";
    LIFE_DRAWINGS = "Drawings from life", "写生";
    LIFE_NEW = "New…", "新的…";
    LIFE_SEAT = "Draw the object in front of you as you see it from your seat; keep your head still.",
        "画你面前的物体，照你坐着看到的样子画，头别动。";
    LIFE_ENVELOPE = "With Sketch, first the whole object's envelope, then its main directions, \
        then how big its parts are against each other, by eye.",
        "先用草稿笔画整个物体的外框，再定主要方向，然后用眼睛比较各部分的大小。";
    LIFE_CHECK = "Check the key points against each other and the negative shapes between the \
        parts.",
        "用关键点互相对照，再看各部分之间的负形对不对。";
    LIFE_PARTS = "Then draw the parts with Draw, the big ones before the small.",
        "最后用勾线笔画出各个部分，先大后小。";
    LIFE_BOOK_AND_CUP = "A book and a cup", "一本书和一个杯子";
    LIFE_KETTLE_OR_LAMP = "A kettle or a desk lamp", "一个水壶或一盏台灯";
    LIFE_THREE_FACES = "An object turned to show three faces", "一个转到能看见三个面的物体";
}

/// What the path page's card suggests drawing from life at `unit` (from 2): an object's shape,
/// then solids, then an object turned in space.
pub(crate) const fn life_subject(unit: u8) -> Text {
    match unit {
        0..=2 => LIFE_BOOK_AND_CUP,
        3 | 4 => LIFE_KETTLE_OR_LAMP,
        _ => LIFE_THREE_FACES,
    }
}
