//! Synthetic learners (S147), as the design's prototype drew them: 120 days, a lesson of a skill on
//! four days in five, four exercises a lesson at the learner's level, their results drawn from the
//! model at the true ability against the level's difficulty, and from day 35 one blind pair a
//! lesson (a copy 1 to 14 days old against one of its level 21 days older or more), its answer
//! drawn alike. Kept as the store keeps them: sheets and pairs.

use openpaint_practice::{
    Answer, BlindPair, CopyRef, Exercise, First, Judge, Kind, LineTier, Outcome, Picked, Planned,
    Purpose, Random, Sheet, SkillLevel, older_left,
};

/// 2026-10-05 12:00 UTC.
pub const NOON: u64 = 1_791_201_600;
pub const DAY: u64 = 86_400;
const DAYS: i64 = 120;
const PER_LESSON: usize = 4;
/// The true levels' steps, the rating scale's thresholds, a pair's tie.
const STEP: f64 = 1.5;
const TIE: f64 = 0.85;

/// How the true ability goes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Truth {
    /// From 0 to this over the 120 days.
    Rising(f64),
    /// Stays at 0, its levels rising on days 40 and 80 whatever.
    Forced,
    Flat,
}

/// Who judges.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Eye {
    /// As the model says.
    True,
    /// At random, every result and every pair.
    Random,
    /// As the model says half the time, at random the other half.
    Erratic,
}

/// A learner's records.
pub struct Learned {
    pub sheets: Vec<Sheet>,
    pub pairs: Vec<BlindPair>,
}

fn sigmoid(t: f64) -> f64 {
    1.0 / (1.0 + (-t).exp())
}

/// An outcome drawn at `u`.
fn drawn(random: &mut Random, u: f64) -> Outcome {
    let weights = [1.0, (u + 1.0).exp(), (2.0 * u).exp()];
    let total: f64 = weights.iter().sum();
    let pick = random.unit() * total;
    if pick < weights[0] {
        Outcome::Redo
    } else if pick < weights[0] + weights[1] {
        Outcome::Near
    } else {
        Outcome::Clean
    }
}

fn anything(random: &mut Random) -> Outcome {
    [Outcome::Redo, Outcome::Near, Outcome::Clean][(random.next_u64() % 3) as usize]
}

/// Which copy an answer of `v` calls better: -1 older, 0 alike, 1 newer.
fn answered(random: &mut Random, v: f64) -> i8 {
    let (newer, older) = (sigmoid(v - TIE), sigmoid(-v - TIE));
    let pick = random.unit();
    if pick < newer {
        1
    } else if pick < newer + older {
        -1
    } else {
        0
    }
}

/// The learner of `truth` judged by `eye`, seeded `seed`.
pub fn learner(truth: Truth, eye: Eye, seed: u64) -> Learned {
    let mut random = Random::new(seed);
    let ability = |day: i64| match truth {
        Truth::Rising(by) => by * day as f64 / DAYS as f64,
        Truth::Forced | Truth::Flat => 0.0,
    };
    let (mut sheets, mut pairs) = (Vec::new(), Vec::new());
    let mut copies: Vec<(CopyRef, i64, u8)> = Vec::new();
    let mut level: u8 = 0;
    for day in 0..DAYS {
        if random.unit() >= 0.8 {
            continue;
        }
        level = match truth {
            Truth::Forced => u8::from(day >= 40) + u8::from(day >= 80),
            _ if ability(day) - STEP * f64::from(level) > 2.0 && level < 2 => level + 1,
            _ => level,
        };
        let id = sheets.len() as u64 + 1;
        let at = NOON + day as u64 * DAY;
        let u = ability(day) - STEP * f64::from(level);
        let outcomes: Vec<Outcome> = (0..PER_LESSON)
            .map(|_| match eye {
                Eye::True => drawn(&mut random, u),
                Eye::Random => anything(&mut random),
                Eye::Erratic if random.unit() < 0.5 => anything(&mut random),
                Eye::Erratic => drawn(&mut random, u),
            })
            .collect();
        sheets.push(lesson(id, at, level, &outcomes));
        if day >= 35 {
            let pair = paired(&mut random, (&copies, day, level), (ability, eye), at);
            pairs.extend(pair.map(|pair| BlindPair {
                lesson: (id, 0),
                ..pair
            }));
        }
        for index in 0..PER_LESSON {
            let copy = CopyRef {
                sheet: id,
                exercise: index as u16,
            };
            copies.push((copy, day, level));
        }
    }
    Learned { sheets, pairs }
}

/// The day's pair, from the copies before it.
fn paired(
    random: &mut Random,
    (copies, today, level): (&[(CopyRef, i64, u8)], i64, u8),
    (ability, eye): (impl Fn(i64) -> f64, Eye),
    at: u64,
) -> Option<BlindPair> {
    let recent: Vec<_> = (copies.iter())
        .filter(|(_, day, held)| (today - 14..today).contains(day) && *held == level)
        .collect();
    let (newer, new_day, _) = **recent.get((random.next_u64() as usize) % recent.len().max(1))?;
    let older: Vec<_> = (copies.iter())
        .filter(|(_, day, held)| *day <= new_day - 21 && *held == level)
        .collect();
    let (older, old_day, _) = **older.get((random.next_u64() as usize) % older.len().max(1))?;
    let v = ability(new_day) - ability(old_day);
    let random_answer = matches!(eye, Eye::Random) || (eye == Eye::Erratic && random.unit() < 0.5);
    let said = if random_answer {
        (random.next_u64() % 3) as i8 - 1
    } else {
        answered(random, v)
    };
    let older_left = older_left(older, newer);
    let answer = match (said, older_left) {
        (0, _) => Answer::Alike,
        (1, true) | (-1, false) => Answer::Right,
        _ => Answer::Left,
    };
    Some(BlindPair {
        asked: at + 600,
        lesson: (0, 0),
        older,
        newer,
        older_left,
        answer,
    })
}

/// A day's sheet `id` at `at`: a lesson of lines at `level`, each exercise compared and judged by
/// eye as `outcomes` say.
pub fn lesson(id: u64, at: u64, level: u8, outcomes: &[Outcome]) -> Sheet {
    let level = SkillLevel::of_code(level);
    let exercises = (0..outcomes.len())
        .map(|i| {
            let kind = Kind::Line(LineTier::Straight);
            (Exercise::generated(kind, id * 16 + i as u64), level)
        })
        .collect();
    let planned = Planned {
        picked: Picked::Chosen,
        exercises,
    };
    let mut sheet = Sheet::begun(id, (Purpose::Day, at), planned);
    sheet.canvas = id;
    for (index, outcome) in outcomes.iter().enumerate() {
        let laid = sheet.laid_mut(index).expect("an exercise");
        laid.compared(First::of(at + 60 * index as u64, 1, Vec::new()));
        laid.result = Some((*outcome, Judge::Eye));
    }
    if let Some(lesson) = sheet.lesson_mut() {
        lesson.finished = Some(at + 900);
        lesson.at = u16::try_from(outcomes.len() - 1).unwrap_or(0);
    }
    sheet
}
