//! Long colour copies (`Kind::Study`, V大's 色彩临摹): a project beside the sessions, one pair of
//! frames of its picture's shape, painted in the full interface with the toolbox's brushes (S104);
//! compared by its picture laid over the copy as strongly as the user sets, the eyedropper
//! comparing a point. No How.

use crate::kinds::{Described, Facts, Kind, Original, Picks, Tools, Work};
use crate::marks::asked::{Asked, Compare};
use crate::marks::drawing::picture_laid;
use crate::marks::mark::SheetMark;
use crate::marks::tile::{TileMark, swatch};
use crate::records::metric::Metrics;
use crate::sessions::family::Family;
use crate::sessions::plan::Exercise;

pub(crate) struct Study;

const FACTS: [Facts; 1] = [Facts::of(Family::Study, "study", Work::Painted)
    .tools(Tools::Full)
    .picks(Picks::Probed)];

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

    fn givens(&self, _exercise: &Exercise, _original: &Original) -> Vec<SheetMark> {
        crate::marks::drawing::pictured()
    }

    fn original(&self, exercise: &Exercise) -> Original {
        exercise.picture()
    }

    fn compare(&self, asked: &Asked<'_>) -> Option<Compare> {
        if !asked.held() {
            return None;
        }
        picture_laid(asked, Metrics::new())
    }

    fn tile(&self, _family: Family) -> Vec<TileMark> {
        swatch()
    }
}
