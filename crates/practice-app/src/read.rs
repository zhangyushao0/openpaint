//! A painted exercise on a practice sheet (S82): a colour combination, a value scale, value masses,
//! a key shift. Its copy is read from the artwork's pixels as the canvas shows them (strokes of any
//! brush, fills and smudges), so Compare asks the runtime for the copy's frame (`ReadJob`), which
//! draws it and reads it off the frame loop as the kind's reading says (`openpaint_practice::Reads`,
//! S83, as the fits are), and the numbers come when the reading does, the first Compare's kept in
//! the record. How much of the original shows over the copy is the user's (a slider); a picture's
//! colour areas are compared one at a time, the area's outline marked on both frames; a value
//! scale is filled a cell at a time, the cell in hand (Fill fills it, then the next; the pen
//! touching a cell takes it, S100); a reading that brings its frames in grey shows them on asking
//! (a key shift's Grey view, S107). The eyedropper is `pick.rs`.

use std::sync::Arc;

use openpaint_practice::{Comparison, Point, Reading, Rgb8, in_grey};

use super::{Compared, PracticeSheet, unavailable};
use crate::copy::PAPER;
use crate::pictures::PictureImage;
use crate::projection::{PracticeNotice, Probe, SHOWN};
use crate::refused::Refused;
use crate::shown::{CopyRead, Greys};
use openpaint_app::Availability;

/// The copy's frame for the runtime to draw and read, for the Compare numbered `sequence`:
/// canvas pixels from `origin`, `size` large, read as its kind's `reading` says (`run`), with the
/// original's picture as read for the screen (none: not read, or none) for a reading that brings
/// its frames in grey.
#[derive(Debug, Clone, PartialEq)]
pub struct ReadJob {
    pub sequence: u64,
    pub origin: [u32; 2],
    pub size: [u32; 2],
    pub reading: Reading,
    pub original: Option<Arc<PictureImage>>,
}

impl ReadJob {
    /// The copy whose frame's pixels are `pixels` (8-bit RGBA on the sheet's paper) as read, with
    /// the numbers a first Compare keeps and, where its reading brings them, both frames in grey.
    #[must_use]
    pub fn run(&self, pixels: &PictureImage) -> Option<CopyRead> {
        let read = self.reading.read(pixels.size, &pixels.pixels, PAPER);
        let grey = |image: &PictureImage| {
            Arc::new(PictureImage {
                size: image.size,
                pixels: in_grey(&image.pixels),
            })
        };
        let greys = (self.reading.greys()).then(|| {
            Arc::new(Greys {
                copy: grey(pixels),
                original: self.original.as_deref().map(grey),
            })
        });
        Some(CopyRead {
            comparison: read.comparison,
            metrics: read.metrics,
            greys,
        })
    }
}

/// What a painted copy read as.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum ReadState {
    /// Its pixels asked of the runtime by the Compare numbered so.
    Asked(u64),
    Read(CopyRead),
    /// The pixels could not be read.
    Unread,
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
    /// original's colour), and the last one compared (`pick.rs`).
    pub(super) probing: Option<(Point, Rgb8)>,
    pub(super) probe: Option<Probe>,
    /// A compared copy's frames shown in grey (S107).
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
    /// An exercise came to show: its painting begins anew (the sheet's count of reads goes on);
    /// its colour areas' first outlined, a value scale's first cell between its ends the clip.
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
            self.painting_state.outline = Arc::new(areas.edges_in_frame(0, None, extent));
        }
        if self.sheet.originals[index].scale().is_some() {
            self.take_cell_at(index, 1);
        }
    }

    /// Cell `cell` of exercise `index`'s value scale in hand: Fill fills it (with all tools the
    /// brush paints it).
    fn take_cell_at(&mut self, index: usize, cell: usize) {
        let Some(scale) = self.sheet.originals[index].scale() else {
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
        let Some(scale) = self.sheet.originals[index].scale() else {
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
        if let Some(scale) = self.sheet.originals[index].scale() {
            let next = (self.painting_state.cell + 1).min(scale.cells - 2);
            self.take_cell_at(index, next);
        }
    }

    /// The value scale's cell in hand, while one shows.
    pub(super) fn cell(&self) -> Option<usize> {
        let index = self.current()?;
        (self.sheet.originals[index].scale().is_some()).then_some(self.painting_state.cell)
    }

    /// The colour area the card compares, outlined on both frames.
    pub(super) fn choose_area(&mut self, area: usize) -> Result<(), Refused> {
        let index = self
            .current()
            .ok_or(unavailable(Availability::NothingLeft))?;
        let Some((areas, extent)) = self.sheet.areas(index) else {
            return Err(unavailable(Availability::NothingLeft));
        };
        if area >= areas.count() {
            return Err(unavailable(Availability::NothingLeft));
        }
        self.painting_state.outline = Arc::new(areas.edges_in_frame(area, None, extent));
        self.painting_state.area = area;
        // The card shows the area chosen, not the point compared before.
        self.painting_state.probe = None;
        Ok(())
    }

    /// A Compare begins: no point compared yet, the frames in colour.
    pub(super) fn forget_probe(&mut self) {
        self.painting_state.probe = None;
        self.painting_state.probing = None;
        self.painting_state.grey = false;
    }

    /// The copy of exercise `index` asked of the runtime for Compare, read as `reading` says, and
    /// what shows until it is read.
    pub(super) fn ask_read(&mut self, index: usize, reading: &Reading) -> (ReadState, Comparison) {
        let frame = self.sheet.pairs[index].copy;
        let picture = self.sheet.pictures.get(index).and_then(Option::as_ref);
        let original = (self.images.iter())
            .find(|(item, _)| picture.is_some_and(|picture| *item == picture.item))
            .and_then(|(_, image)| image.clone());
        self.painting_state.reads += 1;
        self.painting_state.job = Some(ReadJob {
            sequence: self.painting_state.reads,
            origin: frame.origin.map(|v| v.round().max(0.0) as u32),
            size: frame.size.map(|v| v.round().max(1.0) as u32),
            reading: reading.clone(),
            original,
        });
        (
            ReadState::Asked(self.painting_state.reads),
            reading.unread(),
        )
    }

    /// The copy's pixels to read, once: the runtime takes it.
    pub fn read_request(&mut self) -> Option<ReadJob> {
        self.painting_state.job.take()
    }

    /// The runtime read the copy for the Compare numbered `sequence` (`None`: it could not): the
    /// copy as read shows if that Compare's comparison still does, and its numbers are kept as
    /// the exercise's first.
    pub fn read(&mut self, sequence: u64, painted: Option<CopyRead>) {
        if self.current().is_none() || sequence <= self.painting_state.since {
            return;
        }
        let index = self.record.current as usize;
        if let Some(result) = self
            .record
            .results
            .iter_mut()
            .find(|r| r.index as usize == index)
            && result.metrics.is_empty()
            && let Some(read) = &painted
        {
            result.metrics.clone_from(&read.metrics);
            self.changed = true;
        }
        if let Some(Compared::Painted(_, shown, _)) = &mut self.compared
            && *shown == ReadState::Asked(sequence)
        {
            *shown = painted.map_or(ReadState::Unread, ReadState::Read);
        }
    }

    /// The compared copy's frames in grey (`on`) or in colour; refused unless its reading brings
    /// them (a key shift's, S107).
    pub(super) fn grey(&mut self, on: bool) -> Result<(), Refused> {
        let greys =
            matches!(&self.compared, Some(Compared::Painted(reading, ..)) if reading.greys());
        if !greys {
            return Err(unavailable(Availability::NothingLeft));
        }
        self.painting_state.grey = on;
        Ok(())
    }

    /// How much of the original shows, the area compared and its outline, the frames in grey,
    /// and the point the eyedropper compared, for the comparison's marks and card.
    pub(super) fn painted_overlay(&self) -> (f32, usize, Arc<Vec<Vec<Point>>>, bool) {
        let state = &self.painting_state;
        (
            state.shown,
            state.area,
            Arc::clone(&state.outline),
            state.grey,
        )
    }

    /// The point the eyedropper compared: the original's colour there and the copy's.
    pub(super) fn probe_colours(&self) -> Option<(Rgb8, Rgb8)> {
        (self.painting_state.probe).map(|probe| (probe.original, probe.copy))
    }
}

/// A read copy's frames in grey, as its reading brought them (none until read, or for a reading
/// without).
pub(super) fn greys(state: &ReadState) -> Option<Arc<Greys>> {
    match state {
        ReadState::Read(read) => read.greys.clone(),
        _ => None,
    }
}
