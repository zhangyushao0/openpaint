//! Ghosted lines (`Kind::Ghosted`, S156, 鬼影线, unit 1's first skill): Drawabox's superimposed and
//! ghosted lines and ghosted planes, the confident stroke between two given points. Plan the two
//! points, ghost the motion from the shoulder a few times, then draw once. Its tasks and their
//! rules are `ghosted.rs` (a plane's `plane.rs`); the answer is the exact lines between the given
//! points, laid over the copy at Compare for the learner's eye (D123: the strokes are never read,
//! `marks.rs`).
//!
//! A lesson at 入门 holds three superimposed lines (about a minute each), at 熟练 two frames of five
//! pairs (two minutes each), at 稳 three planes (a minute and a half each). Drawn with Draw alone.

pub(crate) mod demo;
pub(crate) mod ghosted;
pub(crate) mod marks;
pub(crate) mod plane;

#[cfg(test)]
mod tests;

use std::sync::Arc;

use crate::course::picker::laid;
use crate::demos::demo::Demo;
use crate::kinds::{
    Blank, Described, Facts, Kind, Original, SkillLevel, ToolSet, Turn, held, place,
};
use crate::marks::asked::{Asked, Compare};
use crate::marks::compared::Comparison;
use crate::marks::mark::SheetMark;
use crate::marks::tile::TileMark;
use crate::maths::path::Point;
use crate::packs::pack::ItemRef;
use crate::sheets::exercise::Exercise;
use ghosted::{Ghosted, GhostedTask};
use marks::GhostedCompared;

pub(crate) struct GhostedLines;

/// Its topics, one a level: superimposed lines, pairs of points, planes.
const FACTS: [Facts; 3] = [
    Facts::of("superimposed-lines", ToolSet::INK, 60).constructed(),
    Facts::of("ghosted-pairs", ToolSet::INK, 120).constructed(),
    Facts::of("ghosted-planes", ToolSet::INK, 90).constructed(),
];
/// How many exercises each level lays.
const SUPERIMPOSED: usize = 3;
const PAIRED: usize = 2;
const PLANES: usize = 3;

impl Described for GhostedLines {
    fn code(&self) -> u8 {
        36
    }

    fn facts(&self) -> &'static [Facts] {
        &FACTS
    }

    fn key(&self) -> &'static str {
        "ghosted-lines"
    }

    fn variant(&self, kind: Kind) -> Option<u8> {
        match kind {
            Kind::Ghosted(ghosted) => Some(place(&Ghosted::ALL, &ghosted)),
            _ => None,
        }
    }

    fn of_variant(&self, variant: u8) -> Option<Kind> {
        (Ghosted::ALL.get(usize::from(variant))).map(|ghosted| Kind::Ghosted(*ghosted))
    }

    fn topic(&self, kind: Kind) -> &'static Facts {
        match kind {
            Kind::Ghosted(Ghosted::Pairs) => &FACTS[1],
            Kind::Ghosted(Ghosted::Plane) => &FACTS[2],
            _ => &FACTS[0],
        }
    }

    fn generate(&self, exercise: &Exercise) -> Arc<dyn Original> {
        let Kind::Ghosted(ghosted) = exercise.kind else {
            return held(Blank);
        };
        held(GhostedTask::generate(ghosted, exercise.seed))
    }

    fn how(&self, kind: Kind, _cells: usize) -> Option<Demo> {
        let Kind::Ghosted(ghosted) = kind else {
            return None;
        };
        Some(demo::ghosted(ghosted))
    }

    fn lesson(&self, level: SkillLevel, _turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        let (ghosted, count) = match level {
            SkillLevel::First => (Ghosted::Superimposed, SUPERIMPOSED),
            SkillLevel::Second => (Ghosted::Pairs, PAIRED),
            SkillLevel::Third => (Ghosted::Plane, PLANES),
        };
        laid(std::iter::repeat_n(Kind::Ghosted(ghosted), count))
    }

    /// A dot, three strokes from it fraying a little toward their far end.
    fn glyph(&self) -> Vec<TileMark> {
        let start = [11.0, 31.0];
        let mut marks: Vec<TileMark> = [[43.0, 10.0], [44.0, 11.5], [42.5, 8.8]]
            .into_iter()
            .map(|end| TileMark::Line(vec![start, end]))
            .collect();
        marks.push(TileMark::Dot(start));
        marks
    }
}

/// The lines between given points.
impl Original for GhostedTask {
    fn givens(&self) -> Vec<SheetMark> {
        marks::givens(self)
    }

    /// Once anything lies in the copy's frame, the exact lines laid over it for the eye; the copy's
    /// strokes are never read (D123).
    fn compare(&self, asked: &Asked<'_>) -> Option<Compare> {
        let compared = || GhostedCompared { task: self.clone() };
        (asked.held()).then(|| Compare::Checked(Comparison::of(compared())))
    }

    /// A stroke along each line: a superimposed line eight times over.
    fn answer(&self) -> Vec<Vec<Point>> {
        let times = if self.ghosted == Ghosted::Superimposed {
            8
        } else {
            1
        };
        (self.lines.iter())
            .flat_map(|line| std::iter::repeat_n(line.to_vec(), times))
            .collect()
    }
}
