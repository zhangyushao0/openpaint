//! Free boxes (`Kind::FreeBoxes`, S84): boxes drawn freely on a horizon, checked by the rules
//! alone and judged by eye (S114). Their page is `geometry/perspective/page.rs`; its How
//! `demo.rs`.
//!
//! A lesson (a page of three or four boxes, about 3 minutes, at every level): the first level its
//! horizon in the middle, high, then low by the lesson, the boxes two-point; the second (S142) a
//! three-point page, the eye looking down from a high horizon or up from a low one in turn, two
//! faint guides leading to its third point; the third the same pages with the horizon alone, the
//! third point the learner's own, found from their uprights (`third.rs`). Each look has its own
//! How (a topic of its own).

pub(crate) mod demo;
pub(crate) mod marks;
pub(crate) mod perspective_free;
pub(crate) mod third;

use crate::course::picker::turned;
use crate::demos::demo::Demo;
use crate::geometry::perspective::{Direction, Horizon, Look, PerspectiveBox};
use crate::kinds::{
    Blank, Described, Facts, Kind, Original, SkillLevel, ToolSet, Turn, held, place,
};
use crate::marks::compared::{Compared, DrawnCopy};
use crate::marks::mark::SheetMark;
use crate::marks::tile::{TileMark, boxed};
use crate::packs::pack::ItemRef;
use crate::sheets::exercise::Exercise;
use std::sync::Arc;
use third::Uprights;

pub(crate) struct FreeBoxes;

/// Its topics, by the way the page's eye looks (`Look`'s order): each has its own How.
const FACTS: [Facts; 3] = [
    Facts::of("free-boxes", ToolSet::DRAWING, 180).constructed(),
    Facts::of("free-boxes-down", ToolSet::DRAWING, 180).constructed(),
    Facts::of("free-boxes-up", ToolSet::DRAWING, 180).constructed(),
];
/// The first level's horizons in turn: in the middle first; the second's and the third's pages
/// looking down or up in turn.
const HORIZONS: [Horizon; 3] = [Horizon::Middle, Horizon::High, Horizon::Low];
const GUIDED: [Horizon; 2] = [Horizon::HighDown, Horizon::LowUp];
const BARE: [Horizon; 2] = [Horizon::HighDownBare, Horizon::LowUpBare];

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
        match horizon.look() {
            Look::Level => &FACTS[0],
            Look::Down => &FACTS[1],
            Look::Up => &FACTS[2],
        }
    }

    fn generate(&self, exercise: &Exercise) -> Arc<dyn Original> {
        let Kind::FreeBoxes(horizon) = exercise.kind else {
            return held(Blank);
        };
        let example = PerspectiveBox::on_horizon(horizon, exercise.seed);
        held(FreePage(example, horizon))
    }

    fn how(&self, kind: Kind, _cells: usize) -> Option<Demo> {
        let Kind::FreeBoxes(horizon) = kind else {
            return None;
        };
        Some(demo::free_boxes(horizon.look()))
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
    /// Where the page's uprights head: vertical where its eye looks level; for its third point,
    /// given or the learner's own.
    #[must_use]
    pub fn uprights(&self) -> Uprights {
        let Self(example, horizon) = self;
        let Some(third) = example.vanishing[Direction::Vertical.index()] else {
            return Uprights::Plumb;
        };
        match horizon.look() {
            Look::Level => Uprights::Plumb,
            _ if horizon.guided() => Uprights::Given(third),
            look => Uprights::Found {
                below: look == Look::Down,
                page: third,
            },
        }
    }
}

impl Original for FreePage {
    fn givens(&self) -> Vec<SheetMark> {
        marks::givens(self)
    }

    fn check(&self, copy: &DrawnCopy) -> Option<Arc<dyn Compared>> {
        Some(marks::FreeCompared::of(self, copy))
    }
}
