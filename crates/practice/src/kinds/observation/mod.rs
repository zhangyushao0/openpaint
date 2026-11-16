//! Drawing from life (`Kind::Observation`, 写生, S151): an optional project offered from unit 2,
//! no skill and no lesson. The learner photographs the object on their table from the seat they
//! will draw from and adds the photo as a picture of their own (its silhouette derived as any is,
//! D103); a drawing from life is then a sheet of its own of one pair of frames of the photo's
//! shape (`Purpose::Copy`, kept until removed), drawn as a line drawing with the drawing pens
//! while the photo's frame is covered (`Facts::hidden`), the photo showing from its first Compare
//! on. Compare lays the photo over the drawing at the slider's strength, in place or fitted onto
//! the drawing's lines as a single shape's outline is (Aligned, `align_shape`); no result is asked
//! or kept (`Verdict::Nobody`), no lesson, dot or seal counts it. The practice book keeps the
//! drawing as first drawn (`Kept::Drawing`), the first lesson's drawing its "before".

use std::sync::Arc;

use crate::kinds::{Described, Facts, Kept, Kind, Original, Picking, ToolSet, held};
use crate::marks::asked::{Asked, Compare, ToFit};
use crate::marks::card::{CardItem, Say};
use crate::marks::compared::{Compared, Comparison, Seen, Verdict};
use crate::marks::drawing::pictured;
use crate::marks::mark::{Clip, Figure, In, Place, SheetMark, Source};
use crate::maths::path::Point;
use crate::packs::pack::ItemRef;
use crate::sheets::exercise::Exercise;

pub(crate) struct Observation;

/// A drawing from life takes a quarter of an hour or so; no lesson counts it.
const FACTS: [Facts; 1] = [Facts::of("observation", ToolSet::DRAWING, 900)
    .aligned()
    .hidden()
    .picks(Picking::LOCKED)
    .kept(Kept::Drawing)];

impl Described for Observation {
    fn code(&self) -> u8 {
        23
    }

    fn facts(&self) -> &'static [Facts] {
        &FACTS
    }

    fn variant(&self, kind: Kind) -> Option<u8> {
        matches!(kind, Kind::Observation).then_some(0)
    }

    fn of_variant(&self, variant: u8) -> Option<Kind> {
        (variant == 0).then_some(Kind::Observation)
    }

    fn generate(&self, exercise: &Exercise) -> Arc<dyn Original> {
        held(Observed(exercise.item()))
    }
}

/// A drawing from life's original: the photo of one's own it was drawn against.
#[derive(Debug)]
pub(crate) struct Observed(ItemRef);

impl Original for Observed {
    fn givens(&self) -> Vec<SheetMark> {
        pictured()
    }

    /// Once anything lies in its frame and its photo is held: the photo laid over the drawing, its
    /// silhouette fitted onto the drawing's lines for Aligned (none to fit without one: in place
    /// only).
    fn compare(&self, asked: &Asked<'_>) -> Option<Compare> {
        if !asked.held() {
            return None;
        }
        let picture = asked.picture?;
        let fit = (picture.outline.clone()).map(|original| ToFit {
            original,
            shape: true,
        });
        let shown = Comparison::of(ObservedCompared {
            corners: picture.fit.corners(),
            outlined: fit.is_some(),
        });
        Some(Compare::Fitted(shown, fit))
    }

    fn picture(&self) -> Option<&ItemRef> {
        Some(&self.0)
    }
}

/// A drawing from life compared: its photo's corners (frame units, its frame's shape) and whether
/// the photo has a silhouette to align.
#[derive(Debug, Clone, PartialEq)]
pub struct ObservedCompared {
    pub corners: [Point; 4],
    pub outlined: bool,
}

impl Compared for ObservedCompared {
    /// The photo over the drawing as strongly as the user sets, in place or aligned.
    fn marks(&self, seen: &Seen<'_>) -> Vec<SheetMark> {
        let Some(fit) = seen.fit else {
            return Vec::new();
        };
        let photo = Figure::Picture {
            source: Source::Pack,
            place: Place::Corners(self.corners.map(|p| fit.apply(p))),
            shown: seen.shown,
        };
        vec![SheetMark::free(In::Copy, photo).clipped(Clip::Laid)]
    }

    fn card(&self, _seen: &Seen<'_>) -> Vec<CardItem> {
        let none = (!self.outlined).then(|| CardItem::Says(Say::plain("no-outline")));
        [CardItem::Overlaid, CardItem::Slider]
            .into_iter()
            .chain(none)
            .collect()
    }

    /// The eye looks; nothing is asked or kept.
    fn verdict(&self) -> Verdict {
        Verdict::Nobody
    }
}
