//! What the progress fit reads of the records (S147), for one skill: every exercise of it judged in
//! a lesson (a day's, a test's or a re-test's; the first lesson scores nothing, a colour copy is no
//! lesson) on the local day of its first Compare, as its result against its level or, where its
//! kind's verdict cuts one kept number in three, as that number (`Described::measured`) held within
//! its result's band; and every blind pair of two of its copies on their days, halved where one was
//! drawn the day the pair was asked (a re-test's own copy). The kept counts and shares whose answer
//! is one of several (49 to 58) and the retired stroke measures (47, 48) are never read. Whether a
//! skill was practised at all, judged or not, names it on the book's page before it has a curve.

use crate::kinds::{Skill, SkillLevel};
use crate::marks::compared::Outcome;
use crate::progress::fit::{History, Observed};
use crate::records::lessons::lessons;
use crate::records::pairs::{BlindPair, CopyRef};
use crate::records::sheet::{Laid, Picked, Sheet};
use crate::records::time::local_day;

/// A number's scale: past the near band −1 and lower, within the clean band +1 and higher.
const FAR: f64 = 3.0;

/// The number `off` (an error, larger worse) on a result's scale: −1 at the near band's edge
/// `near`, +1 at the clean band's `clean`, the logarithm of the error between and beyond, held
/// within ±3; an error of nought is +3.
#[must_use]
pub(crate) fn scaled(off: f64, [clean, near]: [f64; 2]) -> f64 {
    if off <= 0.0 {
        return FAR;
    }
    let z = -1.0 + 2.0 * (near / off).ln() / (near / clean).ln();
    z.clamp(-FAR, FAR)
}

/// `z` held within `outcome`'s band of the scale: what the verdict knew that the number does not
/// (a contrast made strong, a line missing its dot) keeps it there.
fn within(z: f64, outcome: Outcome) -> f64 {
    let (low, high) = match outcome {
        Outcome::Redo => (-FAR, -1.0),
        Outcome::Near => (-1.0, 1.0),
        Outcome::Clean => (1.0, FAR),
    };
    z.clamp(low, high)
}

/// A skill's history as the fit takes it, and where each level past the first began.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Gathered {
    pub(crate) history: History,
    /// Each level's first day, by its index among the days: the second's and the third's.
    pub(crate) levels: Vec<(usize, SkillLevel)>,
}

/// What was observed before the days were numbered.
enum Raw {
    One {
        day: i64,
        level: SkillLevel,
        outcome: Outcome,
        z: Option<f64>,
    },
    Two {
        older: i64,
        newer: i64,
        pair: BlindPair,
        weight: f64,
    },
}

/// `skill`'s history in `sheets` and `pairs`, its days local on a clock `offset` minutes from UTC.
pub(crate) fn gathered(
    skill: Skill,
    sheets: &[Sheet],
    pairs: &[BlindPair],
    offset: i32,
) -> Gathered {
    let mut raw = Vec::new();
    for on in lessons(sheets) {
        if on.is_copy() || on.lesson.picked == Picked::First {
            continue;
        }
        for laid in &on.lesson.exercises {
            let Some((outcome, _)) = laid.result else {
                continue;
            };
            if Skill::of(laid.exercise.kind) != Some(skill) {
                continue;
            }
            let at = laid
                .first
                .as_ref()
                .map_or(on.lesson.started, |first| first.at);
            let z = (laid.first.as_ref())
                .and_then(|first| {
                    (laid.exercise.kind.described()).measured(&laid.exercise, &first.metrics)
                })
                .map(|z| within(z, outcome));
            raw.push(Raw::One {
                day: local_day(at, offset),
                level: SkillLevel::of_code(laid.level),
                outcome,
                z,
            });
        }
    }
    for pair in pairs {
        let day = |copy: CopyRef| {
            let (sheet, laid) = copy.find(sheets)?;
            (Skill::of(laid.exercise.kind) == Some(skill)).then(|| copy_day(sheet, laid, offset))
        };
        let (Some(older), Some(newer)) = (day(pair.older), day(pair.newer)) else {
            continue;
        };
        let asked = local_day(pair.asked, offset);
        let weight = if older == asked || newer == asked {
            0.5
        } else {
            1.0
        };
        raw.push(Raw::Two {
            older,
            newer,
            pair: *pair,
            weight,
        });
    }
    numbered(&raw)
}

/// Whether `skill` was laid in a lesson of `sheets` but the first one (a colour copy is no lesson),
/// judged or not.
pub(crate) fn practised(skill: Skill, sheets: &[Sheet]) -> bool {
    (sheets.iter().filter(|sheet| !sheet.is_copy()))
        .flat_map(|sheet| &sheet.lessons)
        .filter(|lesson| lesson.picked != Picked::First)
        .flat_map(|lesson| &lesson.exercises)
        .any(|laid| Skill::of(laid.exercise.kind) == Some(skill))
}

/// The local day a copy was compared on (its sheet's start where it never was).
pub(crate) fn copy_day(sheet: &Sheet, laid: &Laid, offset: i32) -> i64 {
    local_day(
        laid.first.as_ref().map_or(sheet.started, |first| first.at),
        offset,
    )
}

/// `raw` on its days numbered in order.
fn numbered(raw: &[Raw]) -> Gathered {
    let mut days: Vec<i64> = (raw.iter())
        .flat_map(|seen| match seen {
            Raw::One { day, .. } => vec![*day],
            Raw::Two { older, newer, .. } => vec![*older, *newer],
        })
        .collect();
    days.sort_unstable();
    days.dedup();
    let at = |day: i64| days.binary_search(&day).unwrap_or(0);
    let mut levels: Vec<(usize, SkillLevel)> = Vec::new();
    let observed = (raw.iter())
        .map(|seen| match *seen {
            Raw::One {
                day,
                level,
                outcome,
                z,
            } => {
                let day = at(day);
                if level != SkillLevel::First {
                    match levels.iter_mut().find(|(_, held)| *held == level) {
                        Some((first, _)) => *first = (*first).min(day),
                        None => levels.push((day, level)),
                    }
                }
                match z {
                    Some(z) => Observed::Measure { day, level, z },
                    None => Observed::Result {
                        day,
                        level,
                        outcome,
                    },
                }
            }
            Raw::Two {
                older,
                newer,
                pair,
                weight,
            } => Observed::Pair {
                older: at(older),
                newer: at(newer),
                better: pair.better(),
                weight,
            },
        })
        .collect();
    levels.sort_by_key(|(_, level)| *level);
    let history = History {
        days: days.clone(),
        observed,
    };
    Gathered { history, levels }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_number_at_its_bands_lands_on_the_thresholds() {
        let bands = [0.02, 0.05];
        assert!((scaled(0.05, bands) + 1.0).abs() < 1e-12);
        assert!((scaled(0.02, bands) - 1.0).abs() < 1e-12);
        assert!((scaled(0.0, bands) - FAR).abs() < 1e-12);
        assert!((scaled(10.0, bands) + FAR).abs() < 1e-12);
        assert!(scaled(0.03, bands) > -1.0 && scaled(0.03, bands) < 1.0);
        assert!((within(2.0, Outcome::Near) - 1.0).abs() < 1e-12);
        assert!((within(-2.0, Outcome::Clean) - 1.0).abs() < 1e-12);
    }
}
