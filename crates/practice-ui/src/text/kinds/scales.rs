//! A value scale's words: its name and its How card, its steps
//! before Compare made for the row in hand (S97's review): of five the 3rd, then the 2nd and 4th;
//! of nine the 5th, the 3rd and 7th, then the rest. In V大's terms: 明度推移, 明度, 叠图对比.

use openpaint_practice::halving;

use super::{COMPARE_STEP, How, KindWords, words};
use crate::text::Language;

pub(super) const WORDS: [KindWords; 1] = [KindWords {
    made: Some(scale_steps),
    ..words("scale", SCALE_EXERCISE, SCALE)
}];

/// A value scale's How steps before Compare for a row of `cells`, a step a round of halving: the
/// middle cell, the middle of each half, then the cells left.
fn scale_steps(language: Language, cells: usize) -> Vec<String> {
    let rounds = halving(cells);
    (rounds.iter().enumerate())
        .map(|(at, round)| {
            let named = cells_named(language, round);
            match (language, at) {
                (Language::English, 0) => {
                    format!("The middle cell first ({named}): halfway in lightness")
                }
                (Language::English, 1) => format!("Then the middle of each half ({named})"),
                (Language::English, _) => format!("Then the cells left ({named})"),
                (Language::Chinese, 0) => format!("先填正中的{named}：明度取两端的正中"),
                (Language::Chinese, 1) => format!("再填两半各自的正中：{named}"),
                (Language::Chinese, _) => format!("最后填剩下的{named}"),
            }
        })
        .collect()
}

/// Cells (indices from 0) named in order: "the 2nd and 4th", "第 2、4 格".
fn cells_named(language: Language, cells: &[usize]) -> String {
    let places: Vec<usize> = cells.iter().map(|cell| cell + 1).collect();
    match language {
        Language::Chinese => {
            let numbers: Vec<String> = places.iter().map(usize::to_string).collect();
            format!("第 {} 格", numbers.join("、"))
        }
        Language::English => {
            let ordinal = |n: &usize| match n % 10 {
                1 if n % 100 != 11 => format!("{n}st"),
                2 if n % 100 != 12 => format!("{n}nd"),
                3 if n % 100 != 13 => format!("{n}rd"),
                _ => format!("{n}th"),
            };
            let named: Vec<String> = places.iter().map(ordinal).collect();
            match named.split_last() {
                Some((last, rest)) if !rest.is_empty() => {
                    format!("the {} and {last}", rest.join(", "))
                }
                _ => format!("the {}", named.join("")),
            }
        }
    }
}

/// A value scale's How: its steps before Compare name the cells of the row in hand
/// (`scale_steps`).
const SCALE: How = How {
    heading: SCALE_EXERCISE,
    trains: Some(SCALE_TRAINS),
    steps: &[COMPARE_STEP],
    says: &[SCALE_SAYS],
};

texts! { PRACTICE_SCALES;
    SCALE_EXERCISE = "Value scale", "明度推移";
    SCALE_TRAINS = "Lightness as the eye sees it, in even steps, beside the colour work.",
        "按眼睛看到的明度均匀推移，配合色彩练习。";
    SCALE_SAYS = "The numbers under the example's cells are the order. Keep the hue and purity: \
        move only up and down in the wheel's square, from an end's colour picked; Fill cell fills \
        the cell in hand. To correct a cell, touch it with the pen to take it in hand and Fill \
        cell again.",
        "示例格子下的数字就是填的顺序。色相和纯度保持不变：先吸一端的颜色，只在色环中间的方块里上下移动；“填满格子”填满选中的那一格。要修正一格，先用笔点一下那一格选中它，再点“填满格子”。";
}
