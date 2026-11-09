//! A colour combination compared (S82, S101): its picture over the copy in place as strongly as the
//! user sets, the area the card compares outlined in ink on both frames, the edges where the copy
//! made a weak contrast strong in the marking red. The card: the slider, a chip for each area to
//! choose one, the chosen area's colours on the wheel and in the colour table, or the point the
//! eyedropper compared; a line where the area is not painted. A generated one's (a record of 1.18.1
//! and before) shows its picture drawn again in its original's frame.

use crate::colour_areas::areas_read::AreasCopy;
use crate::kinds::colours::judgement::DELTA_SPAN;
use crate::marks::card::{CardItem, Say, WheelSize};
use crate::marks::compared::{Compared, Outcome, Seen, Verdict};
use crate::marks::mark::{Figure, In, Place, SheetMark, Source};
use crate::maths::colour::{Difference, Rgb8};
use crate::maths::path::Point;

/// A combination is clean when its areas are off by at most this on the mean (Oklab: two and a
/// half just-noticeable steps, a colour's near band) and near within `DELTA_SPAN` (five).
pub const AREAS_CLEAN: f64 = 0.05;
/// A near combination made at most this many weak contrasts strong.
pub const NEAR_STRONG: usize = 1;

/// A generated combination's picture, drawn again, in its original's frame.
pub(crate) fn generated_givens() -> Vec<SheetMark> {
    let picture = Figure::Picture {
        source: Source::Painted,
        place: Place::Frame,
        shown: 1.0,
    };
    vec![SheetMark::free(In::Original, picture)]
}

/// A colour combination's copy as read at the last Compare, and the edges where it made a weak
/// contrast strong (frame units).
#[derive(Debug, Clone, PartialEq)]
pub struct AreasRead {
    pub copy: AreasCopy,
    pub strong: Vec<Vec<Point>>,
}

/// A colour combination compared: its areas' colours and the copy as read (none until its pixels
/// are).
#[derive(Debug, Clone, PartialEq)]
pub struct AreasCompared {
    pub colours: Vec<Rgb8>,
    pub read: Option<AreasRead>,
}

impl Compared for AreasCompared {
    fn marks(&self, seen: &Seen<'_>) -> Vec<SheetMark> {
        let picture = Figure::Picture {
            source: Source::PaintedOrPack,
            place: Place::Frame,
            shown: seen.shown,
        };
        let chosen = |on| {
            let lines = seen.outline.to_vec();
            SheetMark::free(on, Figure::Haloed { lines, red: false })
        };
        let mut marks = vec![
            SheetMark::free(In::Copy, picture),
            chosen(In::Original),
            chosen(In::Copy),
        ];
        if let Some(read) = &self.read {
            let lines = read.strong.clone();
            marks.push(SheetMark::free(
                In::Copy,
                Figure::Haloed { lines, red: true },
            ));
        }
        marks
    }

    fn card(&self, seen: &Seen<'_>) -> Vec<CardItem> {
        let copy = self.read.as_ref().map(|read| read.copy.colours.clone());
        let chips = CardItem::Chips {
            originals: self.colours.clone(),
            copy,
            marked: Vec::new(),
        };
        let wheel = |(original, copy)| CardItem::Wheel {
            original,
            copy,
            size: WheelSize::Small,
        };
        let last = match (seen.probe, &self.read) {
            (Some(probe), _) => wheel(probe),
            (None, None) => CardItem::Reading,
            (None, Some(read)) => match read.copy.colours.get(seen.chosen).copied().flatten() {
                Some(mine) => wheel((self.colours[seen.chosen], mine)),
                None => CardItem::Says(Say::plain("not-painted")),
            },
        };
        vec![CardItem::Slider, chips, last]
    }

    /// Once read: incomplete while an area is not painted; clean when the areas are off by
    /// `AREAS_CLEAN` at most on the mean and no weak contrast was made strong; near within
    /// `DELTA_SPAN` and `NEAR_STRONG` made strong; else redo. Unread, no verdict yet.
    fn verdict(&self) -> Verdict {
        let Some(read) = &self.read else {
            return Verdict::Incomplete;
        };
        let painted: Option<Vec<Rgb8>> = read.copy.colours.iter().copied().collect();
        let Some(painted) = painted.filter(|painted| !painted.is_empty()) else {
            return Verdict::Incomplete;
        };
        let off = (self.colours.iter().zip(&painted))
            .map(|(original, copy)| Difference::of(*original, *copy).delta)
            .sum::<f64>()
            / painted.len() as f64;
        let strong = (read.copy.contrasts.iter())
            .filter(|c| c.made_strong())
            .count();
        let outcome = match (Outcome::within(off, [AREAS_CLEAN, DELTA_SPAN]), strong) {
            (Outcome::Clean, 0) => Outcome::Clean,
            (Outcome::Clean | Outcome::Near, ..=NEAR_STRONG) => Outcome::Near,
            _ => Outcome::Redo,
        };
        Verdict::App(outcome)
    }
}
