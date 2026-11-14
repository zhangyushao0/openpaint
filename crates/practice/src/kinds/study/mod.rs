//! Long colour copies (`Kind::Study`, V大's 色彩临摹): a project beside the lessons, one pair of
//! frames of its picture's shape, painted in the full interface with the toolbox's brushes (S104);
//! compared by its picture laid over the copy as strongly as the user sets, the eyedropper
//! comparing a point. No How.

use std::sync::Arc;

use crate::kinds::{Described, Facts, Kind, Original, Picking, ToolSet, held};
use crate::marks::asked::{Asked, Compare};
use crate::marks::drawing::picture_laid;
use crate::marks::mark::SheetMark;
use crate::marks::tile::{TileMark, swatch};
use crate::packs::pack::ItemRef;
use crate::records::metric::Metrics;
use crate::sheets::exercise::Exercise;

pub(crate) struct Study;

/// A long colour copy runs 10 to 20 hours over many sittings; no lesson counts it.
const FACTS: [Facts; 1] = [Facts::of("study", ToolSet::FULL, 36_000).picks(Picking::PROBED)];

impl Described for Study {
    fn code(&self) -> u8 {
        7
    }

    fn facts(&self) -> &'static [Facts] {
        &FACTS
    }

    fn variant(&self, kind: Kind) -> Option<u8> {
        matches!(kind, Kind::Study).then_some(0)
    }

    fn of_variant(&self, variant: u8) -> Option<Kind> {
        (variant == 0).then_some(Kind::Study)
    }

    fn generate(&self, exercise: &Exercise) -> Arc<dyn Original> {
        held(Copied(exercise.item()))
    }

    fn glyph(&self) -> Vec<TileMark> {
        swatch()
    }
}

/// A long colour copy's original: the pack's picture it copies.
#[derive(Debug)]
pub(crate) struct Copied(ItemRef);

impl Original for Copied {
    fn givens(&self) -> Vec<SheetMark> {
        crate::marks::drawing::pictured()
    }

    fn compare(&self, asked: &Asked<'_>) -> Option<Compare> {
        if !asked.held() {
            return None;
        }
        picture_laid(asked, Metrics::new())
    }

    fn picture(&self) -> Option<&ItemRef> {
        Some(&self.0)
    }
}
