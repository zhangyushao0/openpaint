//! Free boxes (`Kind::FreeBoxes`, S84): boxes drawn freely on a horizon, judged by eye (S114);
//! nothing is read from the strokes (S145, D123): Compare runs the page's horizon on across the
//! sheet. Their page is `geometry/perspective/page.rs`; its How `demo.rs`.
//!
//! A lesson (a page of three or four boxes, about 3 minutes, at every level): the first level its
//! horizon in the middle, high, then low by the lesson, the boxes two-point; the second (S142) a
//! three-point page, the eye looking down from a high horizon or up from a low one in turn, two
//! faint guides leading to its third point (`marks::guides`); the third the same pages with the
//! horizon alone, the third point the learner's own, by turns with a page whose path winds back
//! to the horizon, the boxes sent back along it (S156l, `path.rs`). Each look has its own How (a
//! topic of its own), the path page too.

pub(crate) mod demo;
pub(crate) mod marks;
pub(crate) mod path;

use crate::course::picker::turned;
use crate::demos::demo::Demo;
use crate::geometry::perspective::{Direction, Horizon, Look, PerspectiveBox};
use crate::kinds::{
    Blank, Described, Facts, Kind, Original, SkillLevel, ToolSet, Turn, held, place,
};
use crate::marks::asked::{Asked, Compare};
use crate::marks::compared::Comparison;
use crate::marks::mark::SheetMark;
use crate::marks::tile::{TileMark, boxed};
use crate::maths::path::Point;
use crate::packs::pack::ItemRef;
use crate::sheets::exercise::Exercise;
use path::PathPage;
use std::sync::Arc;

pub(crate) struct FreeBoxes;

/// Its topics, by the way the page's eye looks (`Look`'s order), then the path page: each has its
/// own How.
const FACTS: [Facts; 4] = [
    Facts::of("free-boxes", ToolSet::DRAWING, 180).constructed(),
    Facts::of("free-boxes-down", ToolSet::DRAWING, 180).constructed(),
    Facts::of("free-boxes-up", ToolSet::DRAWING, 180).constructed(),
    Facts::of("free-boxes-path", ToolSet::DRAWING, 180).constructed(),
];
/// The first level's horizons in turn: in the middle first; the second's pages looking down or
/// up in turn; the third's bare pages by turns with the path page (S156l), a lesson's place two
/// round still the page looking down it laid before.
const HORIZONS: [Horizon; 3] = [Horizon::Middle, Horizon::High, Horizon::Low];
const GUIDED: [Horizon; 2] = [Horizon::HighDown, Horizon::LowUp];
const BARE: [Horizon; 4] = [
    Horizon::LowUpBare,
    Horizon::Path,
    Horizon::HighDownBare,
    Horizon::Path,
];

impl Described for FreeBoxes {
    fn code(&self) -> u8 {
        9
    }

    fn facts(&self) -> &'static [Facts] {
        &FACTS
    }

    fn variant(&self, kind: Kind) -> Option<u8> {
        match kind {
            Kind::FreeBoxes(horizon) => Some(place(&Horizon::ALL, &horizon)),
            _ => None,
        }
    }

    fn of_variant(&self, variant: u8) -> Option<Kind> {
        (Horizon::ALL.get(usize::from(variant))).map(|h| Kind::FreeBoxes(*h))
    }

    fn topic(&self, kind: Kind) -> &'static Facts {
        let Kind::FreeBoxes(horizon) = kind else {
            return &FACTS[0];
        };
        match (horizon, horizon.look()) {
            (Horizon::Path, _) => &FACTS[3],
            (_, Look::Level) => &FACTS[0],
            (_, Look::Down) => &FACTS[1],
            (_, Look::Up) => &FACTS[2],
        }
    }

    fn generate(&self, exercise: &Exercise) -> Arc<dyn Original> {
        match exercise.kind {
            Kind::FreeBoxes(Horizon::Path) => held(PathPage::generate(exercise.seed)),
            Kind::FreeBoxes(horizon) => {
                let example = PerspectiveBox::on_horizon(horizon, exercise.seed);
                held(FreePage(example, horizon))
            }
            _ => held(Blank),
        }
    }

    fn how(&self, kind: Kind, _cells: usize) -> Option<Demo> {
        match kind {
            Kind::FreeBoxes(Horizon::Path) => Some(demo::path_page()),
            Kind::FreeBoxes(horizon) => Some(demo::free_boxes(horizon.look())),
            _ => None,
        }
    }

    fn lesson(&self, level: SkillLevel, turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        let horizons: &[Horizon] = match level {
            SkillLevel::First => &HORIZONS,
            SkillLevel::Second => &GUIDED,
            SkillLevel::Third => &BARE,
        };
        turned((horizons, 1), turn, Kind::FreeBoxes)
    }

    fn window(&self) -> usize {
        3
    }

    /// Two small boxes on a horizon.
    fn glyph(&self) -> Vec<TileMark> {
        let mut marks = vec![TileMark::Line(vec![[4.0, 12.0], [50.0, 12.0]])];
        marks.extend(boxed([-12.0, 6.0], 0.45));
        marks.extend(boxed([12.0, 5.0], 0.4));
        marks
    }
}

/// A free-boxes page of `Horizon`: its horizon is its example box's, and on a three-point page
/// its third point too.
#[derive(Debug)]
pub struct FreePage(pub PerspectiveBox, pub Horizon);

impl FreePage {
    /// The third point a guided three-point page gives (its guides lead to it); none on a page
    /// whose eye looks level or whose third point is the learner's own.
    #[must_use]
    pub fn given_third(&self) -> Option<Point> {
        let Self(example, horizon) = self;
        let third = example.vanishing[Direction::Vertical.index()]?;
        (horizon.look() != Look::Level && horizon.guided()).then_some(third)
    }
}

impl Original for FreePage {
    fn givens(&self) -> Vec<SheetMark> {
        marks::givens(self)
    }

    /// Once anything lies in the copy's frame, the page's horizon run on across the sheet; the
    /// copy's strokes are never read (D123).
    fn compare(&self, asked: &Asked<'_>) -> Option<Compare> {
        let compared = || marks::FreeCompared {
            horizon: self.0.horizon,
        };
        (asked.held()).then(|| Compare::Checked(Comparison::of(compared())))
    }
}

/// A path page: its horizon and its path, the boxes free.
impl Original for PathPage {
    fn givens(&self) -> Vec<SheetMark> {
        marks::path_givens(self)
    }

    /// Once anything lies in the copy's frame, the page's horizon run on across the sheet; the
    /// copy's strokes are never read (D123).
    fn compare(&self, asked: &Asked<'_>) -> Option<Compare> {
        let compared = || marks::FreeCompared {
            horizon: self.horizon,
        };
        (asked.held()).then(|| Compare::Checked(Comparison::of(compared())))
    }
}
