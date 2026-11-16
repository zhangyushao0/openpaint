//! Light on form's marks (S153). In its one frame, printed grey: the form's visible edges and
//! outline (its hidden ones not), the pin (its top a dot) and its shadow; at 入门 the ray from the
//! pin's top to its shadow's tip dashed, and at 入门 and 熟练 the cast shadow's outline faintly
//! dashed (placing it is cast shadows' ability; at 稳 the learner places it alone). Compared: the
//! answer in the level's greys laid over the copy as strongly as the slider sets (value masses'
//! way), where the families differ hatched in the grey they should be and the inner tones out of
//! order too, the terminator and the cast shadow's outline red. Construction (看画法) lays the
//! light's way through the form in pencil (`construction.rs`). The card: the reason, the slider,
//! Construction; until the copy is read, that it is being read.

use std::sync::Arc;

use crate::kinds::light_on_form::read::LightCheck;
use crate::kinds::light_on_form::scene::{LightTask, Tones};
use crate::kinds::light_on_form::trace::Parting;
use crate::marks::asked::{Pixels, Read, Reads};
use crate::marks::card::CardItem;
use crate::marks::compared::{Compared, Comparison, Seen, Verdict};
use crate::marks::mark::{Clip, Figure, In, LinePen, Place, SheetMark, Source};
use crate::maths::colour::Rgb8;

/// The form's edges, the pin and its shadow, its top a dot, printed grey.
fn printed(task: &LightTask) -> Vec<SheetMark> {
    let mut marks: Vec<SheetMark> = (task.edges.iter())
        .map(|edge| SheetMark::line(In::Copy, edge.clone(), LinePen::Printed))
        .collect();
    let [foot, top, tip] = task.pinned;
    marks.extend([
        SheetMark::line(In::Copy, vec![foot, top], LinePen::Printed),
        SheetMark::line(In::Copy, vec![foot, tip], LinePen::Printed),
        SheetMark::free(
            In::Copy,
            Figure::Given {
                at: top,
                pen: LinePen::Printed,
            },
        ),
    ]);
    marks
}

/// What `printed` prints, and the helps the level gives.
pub(crate) fn givens(task: &LightTask) -> Vec<SheetMark> {
    let mut marks = printed(task);
    let [_, top, tip] = task.pinned;
    if task.lit.tones == Tones::Two {
        marks.push(SheetMark::dashed(
            In::Copy,
            vec![top, tip],
            LinePen::PrintedHalf,
        ));
    }
    if task.lit.tones != Tones::Four {
        let cast = task.parting().cast.into_iter();
        marks.extend(cast.map(|line| SheetMark::dashed(In::Copy, line, LinePen::PrintedHalf)));
    }
    marks
}

/// Light on form's copy, read from its frame's pixels on the compare worker against the model.
#[derive(Debug, Clone, PartialEq)]
pub struct LightReading {
    pub task: Arc<LightTask>,
}

impl Reads for LightReading {
    fn unread(&self) -> Comparison {
        Comparison::of(LightCompared {
            task: Arc::clone(&self.task),
            parting: Parting::default(),
            check: None,
        })
    }

    fn read(&self, size: [u32; 2], pixels: &[u8], _: Rgb8, _: Option<Pixels<'_>>) -> Read {
        let check = self.task.read((size, pixels));
        let metrics = check.metrics.clone();
        let compared = LightCompared {
            task: Arc::clone(&self.task),
            parting: self.task.parting(),
            check: Some(check),
        };
        Read {
            comparison: Comparison::of(compared),
            metrics,
            pictures: Vec::new(),
        }
    }
}

/// Light on form compared: the task, where its families meet, and the copy as read (none until
/// its pixels are).
#[derive(Debug, Clone, PartialEq)]
pub struct LightCompared {
    pub task: Arc<LightTask>,
    pub parting: Parting,
    pub check: Option<LightCheck>,
}

impl Compared for LightCompared {
    fn marks(&self, seen: &Seen<'_>) -> Vec<SheetMark> {
        let picture = Figure::Picture {
            source: Source::Painted,
            place: Place::Frame,
            shown: seen.shown,
        };
        let mut marks = vec![SheetMark::free(In::Copy, picture).clipped(Clip::Laid)];
        // The form and the pin printed over the answer, as over the copy.
        marks.extend(printed(&self.task));
        if seen.construction {
            let lines = self.task.construction().into_iter();
            marks.extend(lines.map(|(points, hidden)| {
                let mark = if hidden {
                    SheetMark::dashed(In::Copy, points, LinePen::Pencil)
                } else {
                    SheetMark::line(In::Copy, points, LinePen::Pencil)
                };
                mark.clipped(Clip::Inside)
            }));
        }
        let hatching = self.check.iter().flat_map(|check| &check.hatching);
        marks.extend(
            hatching
                .map(|(tone, ends)| SheetMark::free_line(In::Copy, ends.to_vec(), tone.hatch())),
        );
        let parted = self.parting.terminator.iter().chain(&self.parting.cast);
        marks.extend(parted.map(|line| {
            SheetMark::line(In::Copy, line.clone(), LinePen::Answer).clipped(Clip::Inside)
        }));
        marks
    }

    fn card(&self, _seen: &Seen<'_>) -> Vec<CardItem> {
        let Some(check) = &self.check else {
            return vec![CardItem::Slider, CardItem::Reading];
        };
        let reason = check.reason.clone().map(CardItem::Says);
        (reason.into_iter())
            .chain([CardItem::Slider, CardItem::Construction])
            .collect()
    }

    fn metrics(&self) -> crate::records::metric::Metrics {
        (self.check.as_ref()).map_or_else(Vec::new, |check| check.metrics.clone())
    }

    /// Once read, its check's (`LightCheck::verdict`); unread, no verdict yet.
    fn verdict(&self) -> Verdict {
        (self.check.as_ref()).map_or(Verdict::Incomplete, LightCheck::verdict)
    }
}
