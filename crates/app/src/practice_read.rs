//! A painted exercise on a practice sheet (S82): a colour combination, a value scale, a long
//! colour copy. Its copy is read from the artwork's pixels as the canvas shows them (strokes of
//! any brush, fills and smudges), so Compare asks the runtime for the copy's frame (`ReadJob`),
//! which draws it and reads it off the frame loop (S83, as the fits are), and the numbers come
//! when the reading does, the first Compare's kept in the record. How much of the original
//! shows over the copy is the user's (a slider); a combination's card compares one area at a
//! time, its outline marked on both frames; a value scale is filled a cell at a time, the cell in
//! hand (Fill fills it, then the next; the pen touching a cell takes it, S100); value masses (S96)
//! are read against the picture's answer. The eyedropper is `practice_pick.rs`.

use std::sync::Arc;

use openpaint_practice::{
    Areas, MassesCheck, Metrics, Original, Point, Rgb8, ScaleCheck, Tendency, ValueMasses,
    ValueScale,
};

use super::key::read_key;
use super::{Compared, PracticeSheet, unavailable};
use crate::error::AppError;
use crate::practice::{AreasRead, KeyRead, PracticeNotice, Probe, SHOWN};
use crate::practice_copy::PAPER;
use crate::practice_pictures::PictureImage;
use crate::projection::Availability;

/// The copy's frame for the runtime to draw and read, for the Compare numbered `sequence`:
/// canvas pixels from `origin`, `size` large, read as `read` asks (`run`).
#[derive(Debug, Clone, PartialEq)]
pub struct ReadJob {
    pub sequence: u64,
    pub origin: [u32; 2],
    pub size: [u32; 2],
    pub read: ReadAs,
}

/// What a painted copy is read against.
#[derive(Debug, Clone, PartialEq)]
pub enum ReadAs {
    /// A colour combination's areas and the size its picture fills in the frame (frame units).
    Areas(Arc<Areas>, Point),
    Scale(ValueScale),
    /// Value masses' answer (S96) and where the picture lies in the frame: its top left and its
    /// size, frame units.
    Masses(Arc<ValueMasses>, [Point; 2]),
    /// A key shift's (S107): its crop's areas, the size its picture fills in the frame, its
    /// tendency, and its picture as read for the screen (none while not read), greyed with it.
    KeyShift {
        areas: Arc<Areas>,
        extent: Point,
        tendency: Tendency,
        original: Option<Arc<PictureImage>>,
    },
}

/// A painted copy as read: a colour combination's areas and the edges where it made a weak
/// contrast strong, a value scale's cells checked, or value masses' hatching (S96).
#[derive(Debug, Clone, PartialEq)]
pub enum CopyRead {
    Areas(Arc<AreasRead>),
    Scale(Arc<ScaleCheck>),
    Masses(Arc<MassesCheck>),
    KeyShift(Arc<KeyRead>),
}

impl ReadJob {
    /// The copy whose frame's pixels are `pixels` (8-bit RGBA on the sheet's paper) as read, with
    /// the numbers a first Compare keeps.
    #[must_use]
    pub fn run(&self, pixels: &PictureImage) -> Option<(CopyRead, Metrics)> {
        match &self.read {
            ReadAs::KeyShift {
                areas,
                extent,
                tendency,
                original,
            } => Some(read_key(
                (areas, *extent, *tendency),
                original.as_deref(),
                pixels,
            )),
            ReadAs::Masses(answer, place) => {
                let check = answer.read((pixels.size, &pixels.pixels), *place);
                let metrics = check.metrics.clone();
                Some((CopyRead::Masses(Arc::new(check)), metrics))
            }
            ReadAs::Areas(areas, extent) => {
                let copy = areas.read(pixels.size, &pixels.pixels, PAPER);
                let strong = (copy.contrasts.iter())
                    .filter(|c| c.made_strong())
                    .flat_map(|c| in_frame(areas.edges(c.areas.0, Some(c.areas.1)), *extent))
                    .collect();
                let metrics = copy.metrics(areas);
                Some((
                    CopyRead::Areas(Arc::new(AreasRead { copy, strong })),
                    metrics,
                ))
            }
            ReadAs::Scale(scale) => {
                let check = scale.read(pixels.size, &pixels.pixels, PAPER);
                let metrics = check.metrics();
                Some((CopyRead::Scale(Arc::new(check)), metrics))
            }
        }
    }
}

/// `lines` in shares of a picture as frame units, the picture filling `extent` of its frame.
pub(super) fn in_frame(lines: Vec<Vec<Point>>, extent: Point) -> Vec<Vec<Point>> {
    (lines.into_iter())
        .map(|line| {
            (line.into_iter())
                .map(|[u, v]| [u * extent[0], v * extent[1]])
                .collect()
        })
        .collect()
}

/// What a painted copy read as.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Reading {
    /// Its pixels asked of the runtime by the Compare numbered so.
    Asked(u64),
    Read(CopyRead),
    /// Nothing to read (a colour copy), or the pixels could not be read.
    Shown,
}

/// A painted exercise's state on the sheet.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Painting {
    /// How much of the original shows over the copy, 0 to 1.
    pub(super) shown: f32,
    /// A colour combination's area compared on the card, and its outline (frame units).
    area: usize,
    outline: Arc<Vec<Vec<Point>>>,
    /// A value scale's cell in hand, which Fill fills (an end never).
    cell: usize,
    /// The copy's pixels to read, how many reads the sheet asked and how many before this
    /// exercise came to show (a reading of an earlier exercise arriving late is dropped).
    job: Option<ReadJob>,
    reads: u64,
    since: u64,
    /// The point the eyedropper compares, waiting for the copy's pixel (frame units, the
    /// original's colour), and the last one compared (`practice_pick.rs`).
    pub(super) probing: Option<(Point, Rgb8)>,
    pub(super) probe: Option<Probe>,
    /// A compared key shift's frames shown in grey (S107).
    pub(super) grey: bool,
}

impl Default for Painting {
    fn default() -> Self {
        Self {
            shown: SHOWN,
            area: 0,
            outline: Arc::default(),
            cell: 1,
            job: None,
            reads: 0,
            since: 0,
            probing: None,
            probe: None,
            grey: false,
        }
    }
}

impl PracticeSheet {
    /// An exercise came to show: its painting begins anew (the sheet's count of reads goes on); a
    /// combination's first area outlined, a value scale's first cell between its ends the clip.
    pub(super) fn painted_shown(&mut self) {
        let reads = self.painting_state.reads;
        self.painting_state = Painting {
            reads,
            since: reads,
            ..Painting::default()
        };
        let Some(index) = self.current() else {
            return;
        };
        if let Some((areas, extent)) = self.sheet.areas(index) {
            self.painting_state.outline = Arc::new(in_frame(areas.edges(0, None), extent));
        }
        if let Original::Scale(_) = &self.sheet.originals[index] {
            self.take_cell_at(index, 1);
        }
    }

    /// Cell `cell` of exercise `index`'s value scale in hand: Fill fills it (with all tools the
    /// brush paints it).
    fn take_cell_at(&mut self, index: usize, cell: usize) {
        let Original::Scale(scale) = &self.sheet.originals[index] else {
            return;
        };
        let frame = self.sheet.pairs[index].copy;
        let [[left, top], [right, bottom]] = scale.cell(cell);
        let corners = [[left, top], [right, top], [right, bottom], [left, bottom]]
            .map(|p| frame.to_canvas(p).map(|v| v as f32));
        self.clip = openpaint_core::RegionMask::polygon(self.canvas, &corners);
        self.painting_state.cell = cell;
    }

    /// A pen landing at the frame point `p` of exercise `index`: on a value scale it takes the
    /// cell between the ends it lands in (true), or is refused; elsewhere nothing (false).
    pub(super) fn take_cell(&mut self, index: usize, p: Point) -> Result<bool, PracticeNotice> {
        let Original::Scale(scale) = &self.sheet.originals[index] else {
            return Ok(false);
        };
        match scale.cell_at(p) {
            Some(cell) if cell > 0 && cell + 1 < scale.cells => {
                self.take_cell_at(index, cell);
                Ok(true)
            }
            _ => Err(PracticeNotice::DrawInCell),
        }
    }

    /// After a value scale's cell was filled: the next one between the ends in hand.
    pub(super) fn filled_cell(&mut self) {
        let Some(index) = self.current() else {
            return;
        };
        if let Original::Scale(scale) = &self.sheet.originals[index] {
            let next = (self.painting_state.cell + 1).min(scale.cells - 2);
            self.take_cell_at(index, next);
        }
    }

    /// The value scale's cell in hand, while one shows.
    pub(super) fn cell(&self) -> Option<usize> {
        let index = self.current()?;
        matches!(self.sheet.originals[index], Original::Scale(_))
            .then_some(self.painting_state.cell)
    }

    /// The colour combination's area the card compares, outlined on both frames.
    pub(super) fn choose_area(&mut self, area: usize) -> Result<(), AppError> {
        let index = self
            .current()
            .ok_or(unavailable(Availability::NothingLeft))?;
        let Some((areas, extent)) = self.sheet.areas(index) else {
            return Err(unavailable(Availability::NothingLeft));
        };
        if area >= areas.count() {
            return Err(unavailable(Availability::NothingLeft));
        }
        self.painting_state.outline = Arc::new(in_frame(areas.edges(area, None), extent));
        self.painting_state.area = area;
        // The card shows the area chosen, not the point compared before.
        self.painting_state.probe = None;
        Ok(())
    }

    /// A Compare begins: no point compared yet, a key shift in colour.
    pub(super) fn forget_probe(&mut self) {
        self.painting_state.probe = None;
        self.painting_state.probing = None;
        self.painting_state.grey = false;
    }

    /// The copy of exercise `index` asked of the runtime for Compare; nothing to read where its
    /// original is no painted one (value masses without their answer: a picture removed).
    pub(super) fn ask_read(&mut self, index: usize) -> Reading {
        let combination = self.sheet.areas(index);
        let read = match &self.sheet.originals[index] {
            _ if let Some(asked) = self.key_asked(index) => asked,
            _ if let Some((areas, extent)) = combination => {
                ReadAs::Areas(Arc::new(areas.clone()), extent)
            }
            Original::Scale(scale) => ReadAs::Scale(*scale),
            Original::Picture(_) => {
                let picture = self.sheet.pictures.get(index).and_then(Option::as_ref);
                let Some((masses, fit)) = picture.and_then(|p| Some((p.masses.as_ref()?, p.fit)))
                else {
                    return Reading::Shown;
                };
                ReadAs::Masses(Arc::clone(&masses.answer), [[0.0, 0.0], fit.size])
            }
            _ => return Reading::Shown,
        };
        let frame = self.sheet.pairs[index].copy;
        self.painting_state.reads += 1;
        self.painting_state.job = Some(ReadJob {
            sequence: self.painting_state.reads,
            origin: frame.origin.map(|v| v.round().max(0.0) as u32),
            size: frame.size.map(|v| v.round().max(1.0) as u32),
            read,
        });
        Reading::Asked(self.painting_state.reads)
    }

    /// The copy's pixels to read, once: the runtime takes it.
    pub(crate) fn read_request(&mut self) -> Option<ReadJob> {
        self.painting_state.job.take()
    }

    /// The runtime read the copy for the Compare numbered `sequence` (`None`: it could not): the
    /// copy as read shows if that Compare's comparison still does, and its numbers are kept as
    /// the exercise's first.
    pub(crate) fn read(&mut self, sequence: u64, painted: Option<(CopyRead, Metrics)>) {
        if self.current().is_none() || sequence <= self.painting_state.since {
            return;
        }
        let reading = painted.map(|(read, metrics)| (Reading::Read(read), metrics));
        let index = self.record.current as usize;
        if let Some(result) = self
            .record
            .results
            .iter_mut()
            .find(|r| r.index as usize == index)
            && result.metrics.is_empty()
            && let Some((_, metrics)) = &reading
        {
            result.metrics.clone_from(metrics);
            self.changed = true;
        }
        if let Some(Compared::Painted(shown)) = &mut self.compared
            && *shown == Reading::Asked(sequence)
        {
            *shown = reading.map_or(Reading::Shown, |(reading, _)| reading);
        }
    }

    /// How much of the original shows, the area compared and its outline, for the overlay.
    pub(super) fn painted_overlay(&self) -> (f32, usize, Arc<Vec<Vec<Point>>>) {
        let state = &self.painting_state;
        (state.shown, state.area, Arc::clone(&state.outline))
    }
}
