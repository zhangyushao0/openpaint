//! Colour under light's marks (S154j). In its one frame, printed grey: the form's visible edges and
//! outline, the pin (its top a dot) and its shadow, and faintly dashed where the families meet (the
//! terminator and the cast shadow's outline: placing them is light on form's ability, this one is
//! colour); beside the frame the colours to mix from, each a swatch named under it: the local
//! colour, the light (the sun's, at 稳 a lamp's), the sky, at 稳 the ground.
//!
//! Compared, the model's render laid over the copy as strongly as the slider sets (one answer: a
//! model is not the light), the form and the pin printed over it, and each region breaking a rule
//! outlined red; in the Grey view the copy and the render in grey, only the regions off in
//! lightness outlined. The card: the disc with the given colours and each region's copy as a dot
//! (those breaking a rule ringed red), Grey, that the picture laid is one answer, the slider; until
//! the copy is read, that it is being read. The rules are a model's, so the learner's eye judges
//! once every region is painted (D123).

use std::sync::Arc;

use crate::kinds::lit_colours::check::Rule;
use crate::kinds::lit_colours::regions::Region;
use crate::kinds::lit_colours::scene::{ColourTask, Palette};
use crate::marks::asked::{Pixels, Read, Reads};
use crate::marks::card::{CardItem, Say};
use crate::marks::compared::{Compared, Comparison, Seen, Verdict};
use crate::marks::mark::{Clip, Figure, In, LinePen, Place, SheetMark, Source};
use crate::maths::colour::{Rgb8, in_grey};
use crate::maths::path::Point;

/// The pictures its reading brings, by their place: the copy in grey, the render in grey.
const GREY_COPY: u8 = 0;
const GREY_MODEL: u8 = 1;
/// Beside the copy's frame (frame units from its top right corner): the swatches' middles across,
/// the first's down and the step to the next (a swatch and its name under it).
const SWATCH_X: f64 = 0.115;
const FIRST_SWATCH: f64 = 0.06;
const SWATCH_STEP: f64 = 0.13;

/// The form's edges, the pin and its shadow, its top a dot, printed grey.
fn printed(task: &ColourTask) -> Vec<SheetMark> {
    let light = &task.task;
    let mut marks: Vec<SheetMark> = (light.edges.iter())
        .map(|edge| SheetMark::line(In::Copy, edge.clone(), LinePen::Printed))
        .collect();
    let [foot, top, tip] = light.pinned;
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

/// The colours given to mix from, each with its name's key: the local colour, the light, the sky
/// and at 稳 the ground.
pub(crate) fn given(task: &ColourTask) -> Vec<(Rgb8, &'static str)> {
    let colours = &task.colours;
    let bounce = task.variant.palette == Palette::Bounce;
    let light = if bounce { "lamplight" } else { "sunlight" };
    let mut given = vec![
        (colours.local, "local-colour"),
        (colours.light, light),
        (colours.sky, "skylight"),
    ];
    if bounce {
        given.push((colours.ground, "ground"));
    }
    given
}

/// What it gives: the form, the pin, where the families meet faintly dashed, and the colours
/// beside the copy's frame.
pub(crate) fn givens(task: &ColourTask) -> Vec<SheetMark> {
    let mut marks = printed(task);
    let parting = task.task.parting();
    let met = parting.terminator.into_iter().chain(parting.cast);
    marks.extend(met.map(|line| SheetMark::dashed(In::Copy, line, LinePen::PrintedHalf)));
    for (k, (colour, name)) in given(task).into_iter().enumerate() {
        let swatch = Figure::Beside {
            at: [SWATCH_X, FIRST_SWATCH + SWATCH_STEP * k as f64],
            colour,
            name: Some(name),
        };
        marks.push(SheetMark::free(In::Copy, swatch));
    }
    marks
}

/// Colour under light's copy, read from its frame's pixels on the compare worker.
#[derive(Debug, Clone, PartialEq)]
pub struct LitReading {
    pub task: Arc<ColourTask>,
}

impl Reads for LitReading {
    fn unread(&self) -> Comparison {
        Comparison::of(LitCompared {
            task: Arc::clone(&self.task),
            read: None,
        })
    }

    /// Each region's colour and the rules broken, their outlines, and the copy and the render in
    /// grey for the Grey view.
    fn read(&self, size: [u32; 2], pixels: &[u8], paper: Rgb8, _: Option<Pixels<'_>>) -> Read {
        let task = &self.task;
        let colours = task.read(size, pixels, paper);
        let broken = task.broken(&colours);
        let outlines = (task.asked().into_iter())
            .filter(|region| broken.iter().any(|(_, r)| r == region))
            .map(|region| (region, task.regions.outline(region)))
            .collect();
        let side = size[0].max(size[1]);
        let model = (([side; 2]), in_grey(&task.render(side)));
        let pictures = vec![(size, in_grey(pixels)), model];
        let compared = LitCompared {
            task: Arc::clone(task),
            read: Some(LitRead {
                colours,
                broken,
                outlines,
            }),
        };
        Read {
            comparison: Comparison::of(compared),
            metrics: Vec::new(),
            pictures,
        }
    }

    fn greys(&self) -> bool {
        true
    }
}

/// A copy as read: each region asked and its colour (none where not painted), the rules broken
/// with the region each marks, and the outlines of the regions marked (frame units).
#[derive(Debug, Clone, PartialEq)]
pub struct LitRead {
    pub colours: Vec<(Region, Option<Rgb8>)>,
    pub broken: Vec<(Rule, Region)>,
    pub outlines: Vec<(Region, Vec<Vec<Point>>)>,
}

impl LitRead {
    /// The outlines of the regions breaking a rule `which` keeps, red.
    fn outlined(&self, which: impl Fn(Rule) -> bool) -> Vec<SheetMark> {
        (self.outlines.iter())
            .filter(|(region, _)| (self.broken.iter()).any(|(rule, r)| r == region && which(*rule)))
            .map(|(_, lines)| {
                let lines = lines.clone();
                SheetMark::free(In::Copy, Figure::Haloed { lines, red: true })
            })
            .collect()
    }
}

/// Colour under light compared: the task and the copy as read (none until its pixels are).
#[derive(Debug, Clone, PartialEq)]
pub struct LitCompared {
    pub task: Arc<ColourTask>,
    pub read: Option<LitRead>,
}

impl Compared for LitCompared {
    fn marks(&self, seen: &Seen<'_>) -> Vec<SheetMark> {
        let picture = |source, shown| Figure::Picture {
            source,
            place: Place::Frame,
            shown,
        };
        let mut marks = if seen.grey {
            vec![
                SheetMark::free(In::Copy, picture(Source::Read(GREY_COPY), 1.0)),
                SheetMark::free(In::Copy, picture(Source::Read(GREY_MODEL), seen.shown))
                    .clipped(Clip::Laid),
            ]
        } else {
            let render = picture(Source::Painted, seen.shown);
            vec![SheetMark::free(In::Copy, render).clipped(Clip::Laid)]
        };
        marks.extend(printed(&self.task));
        if let Some(read) = &self.read {
            let grey = seen.grey;
            marks.extend(read.outlined(|rule| !grey || rule.lightness()));
        }
        marks
    }

    fn card(&self, _seen: &Seen<'_>) -> Vec<CardItem> {
        let Some(read) = &self.read else {
            return vec![CardItem::Reading];
        };
        let given = given(&self.task);
        let mixed_from = |region: Region| match region {
            Region::Light | Region::Shadow | Region::Reflected => Some(0),
            Region::Ground | Region::Cast => None,
        };
        let marked = (read.colours.iter().enumerate())
            .filter(|(_, (region, _))| read.broken.iter().any(|(_, r)| r == region))
            .map(|(at, _)| at)
            .collect();
        vec![
            CardItem::Mixed {
                given: given.iter().map(|(colour, _)| *colour).collect(),
                copy: (read.colours.iter())
                    .map(|(region, colour)| (*colour, mixed_from(*region)))
                    .collect(),
                marked,
            },
            CardItem::Grey,
            CardItem::Says(Say::plain("one-answer")),
            CardItem::Slider,
        ]
    }

    /// Incomplete while a region is not painted or the copy is unread; then the eye's, the marks
    /// its help (D123).
    fn verdict(&self) -> Verdict {
        let Some(read) = &self.read else {
            return Verdict::Incomplete;
        };
        if read.colours.iter().any(|(_, colour)| colour.is_none()) {
            return Verdict::Incomplete;
        }
        Verdict::Eye
    }
}
