//! The reason beside the app's result (S138, `Say::reason`): one line under the rings saying what
//! of the copy is off, which way and how far in words ("The point is a little to the left.",
//! "点偏左了一点。"), never a number. In V大's terms: 色相, 明度 (偏亮/偏暗), 纯度 (偏纯/偏灰),
//! 色块, 带圈的一端, 顺时针/逆时针.

use openpaint_practice::{Arg, Say};

use super::Language;

/// What `say`, a reason, reads as in `language`: its subject (`what`, numbered where it names an
/// area or a cell), its way and its degree.
pub(crate) fn reason(language: Language, say: &Say) -> String {
    let word = |at: usize| match say.args.get(at) {
        Some(Arg::Word(word)) => *word,
        _ => "",
    };
    let number = match say.args.get(3) {
        Some(Arg::Count(number)) => Some(*number),
        _ => None,
    };
    let clearly = word(2) == "clearly";
    match language {
        Language::English => english((word(0), number), word(1), clearly),
        Language::Chinese => chinese((word(0), number), word(1), clearly),
    }
}

fn english((what, number): (&str, Option<usize>), way: &str, clearly: bool) -> String {
    let subject = match (what, number) {
        ("colour", Some(n)) => format!("Area {n}"),
        ("cell", Some(n)) => format!("Cell {n}"),
        ("colour", None) => "The colour".to_owned(),
        ("mark", _) => "The mark".to_owned(),
        ("line", _) => "The line".to_owned(),
        ("upright", _) => "The upright".to_owned(),
        _ => "The point".to_owned(),
    };
    let degree = if clearly { "clearly" } else { "a little" };
    let said = match way {
        "left" | "right" => format!("is {degree} to the {way}"),
        "up" => format!("is {degree} high"),
        "down" => format!("is {degree} low"),
        "far" => format!("is {degree} too far from the ringed end"),
        "near" => format!("is {degree} too near the ringed end"),
        "clockwise" | "anticlockwise" if clearly => format!("is clearly turned {way}"),
        "clockwise" | "anticlockwise" => format!("is turned a little {way}"),
        "light" | "dark" | "pure" | "grey" => format!("is {degree} too {way}"),
        hue => format!("leans {degree} toward {hue}"),
    };
    format!("{subject} {said}.")
}

fn chinese((what, number): (&str, Option<usize>), way: &str, clearly: bool) -> String {
    let subject = match (what, number) {
        ("colour", Some(n)) => format!("色块 {n} "),
        ("cell", Some(n)) => format!("第 {n} 格"),
        ("colour", None) => "颜色".to_owned(),
        ("mark", _) => "标记".to_owned(),
        ("line", _) => "线".to_owned(),
        ("upright", _) => "竖线".to_owned(),
        _ => "点".to_owned(),
    };
    let said = match (way, clearly) {
        ("far", false) => "离带圈的一端远了一点".to_owned(),
        ("far", true) => "离带圈的一端明显太远".to_owned(),
        ("near", false) => "离带圈的一端近了一点".to_owned(),
        ("near", true) => "离带圈的一端明显太近".to_owned(),
        ("clockwise", false) => "顺时针转偏了一点".to_owned(),
        ("clockwise", true) => "明显顺时针转偏了".to_owned(),
        ("anticlockwise", false) => "逆时针转偏了一点".to_owned(),
        ("anticlockwise", true) => "明显逆时针转偏了".to_owned(),
        (way, false) => format!("{}了一点", leaning(way)),
        (way, true) => format!("明显{}", leaning(way)),
    };
    format!("{subject}{said}。")
}

/// A way as it leans in Chinese: 偏左, 偏亮, 偏黄.
fn leaning(way: &str) -> &'static str {
    match way {
        "left" => "偏左",
        "right" => "偏右",
        "up" => "偏上",
        "down" => "偏下",
        "light" => "偏亮",
        "dark" => "偏暗",
        "pure" => "偏纯",
        "grey" => "偏灰",
        "red" => "偏红",
        "yellow" => "偏黄",
        "green" => "偏绿",
        "cyan" => "偏青",
        "blue" => "偏蓝",
        _ => "偏紫",
    }
}

/// Every reason's words in `language`, for the tables' tests.
#[cfg(test)]
pub(crate) fn made(language: Language) -> Vec<String> {
    let ways: [(&'static str, &[&'static str]); 6] = [
        ("point", &["left", "right", "up", "down"]),
        ("mark", &["far", "near"]),
        ("line", &["clockwise", "anticlockwise", "up", "down"]),
        ("upright", &["left", "right"]),
        (
            "colour",
            &[
                "red", "yellow", "green", "cyan", "blue", "purple", "light", "dark", "pure", "grey",
            ],
        ),
        ("cell", &["light", "dark"]),
    ];
    let mut made = Vec::new();
    for (what, ways) in ways {
        for way in ways {
            for off in [0.5, 2.0] {
                let say = Say::reason(what, way, (off, 1.0));
                let say = if what == "cell" { say.numbered(4) } else { say };
                made.push(reason(language, &say));
                if what == "colour" {
                    made.push(reason(language, &say.numbered(3)));
                }
            }
        }
    }
    made
}

#[cfg(test)]
mod tests {
    use super::*;

    fn said(language: Language, say: &Say) -> String {
        super::super::said(language, say)
    }

    #[test]
    fn a_reason_says_which_way_and_how_far_in_both_languages() {
        let (en, zh) = (Language::English, Language::Chinese);
        let left = Say::reason("point", "left", (0.5, 1.0));
        assert_eq!(said(en, &left), "The point is a little to the left.");
        assert_eq!(said(zh, &left), "点偏左了一点。");
        let low = Say::reason("point", "down", (-2.0, 1.0));
        assert_eq!(said(en, &low), "The point is clearly low.");
        assert_eq!(said(zh, &low), "点明显偏下。");
        let far = Say::reason("mark", "far", (2.0, 1.0));
        assert_eq!(
            said(en, &far),
            "The mark is clearly too far from the ringed end."
        );
        assert_eq!(said(zh, &far), "标记离带圈的一端明显太远。");
        let turned = Say::reason("line", "clockwise", (0.5, 1.0));
        assert_eq!(said(en, &turned), "The line is turned a little clockwise.");
        assert_eq!(said(zh, &turned), "线顺时针转偏了一点。");
        let turned = Say::reason("line", "anticlockwise", (2.0, 1.0));
        assert_eq!(
            said(en, &turned),
            "The line is clearly turned anticlockwise."
        );
        assert_eq!(said(zh, &turned), "线明显逆时针转偏了。");
        let hue = Say::reason("colour", "yellow", (0.5, 1.0));
        assert_eq!(said(en, &hue), "The colour leans a little toward yellow.");
        assert_eq!(said(zh, &hue), "颜色偏黄了一点。");
        let area = Say::reason("colour", "grey", (2.0, 1.0)).numbered(3);
        assert_eq!(said(en, &area), "Area 3 is clearly too grey.");
        assert_eq!(said(zh, &area), "色块 3 明显偏灰。");
        let cell = Say::reason("cell", "dark", (0.5, 1.0)).numbered(4);
        assert_eq!(said(en, &cell), "Cell 4 is a little too dark.");
        assert_eq!(said(zh, &cell), "第 4 格偏暗了一点。");
    }

    #[test]
    fn every_reason_reads_in_its_language_and_never_as_a_number() {
        let han = |c: char| ('\u{4e00}'..='\u{9fff}').contains(&c);
        let (en, zh) = (made(Language::English), made(Language::Chinese));
        assert_eq!(en.len(), zh.len());
        for (english, chinese) in en.iter().zip(&zh) {
            assert!(
                english.ends_with('.') && !english.chars().any(han),
                "{english}"
            );
            assert!(
                chinese.ends_with('。') && chinese.chars().any(han),
                "{chinese}"
            );
            // Only an area's or a cell's number, never a measure.
            for line in [english, chinese] {
                let digits: String = line.chars().filter(char::is_ascii_digit).collect();
                assert!(
                    digits.is_empty() || digits == "3" || digits == "4",
                    "{line}"
                );
            }
        }
        let unique = |all: &[String]| {
            let mut sorted = all.to_vec();
            sorted.sort();
            sorted.dedup();
            sorted.len()
        };
        assert_eq!(unique(&en), en.len(), "each reason its own words");
        assert_eq!(unique(&zh), zh.len());
    }
}
