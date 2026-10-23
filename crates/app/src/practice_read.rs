//! A painted exercise on a practice sheet (S82): a colour combination, a value scale, a long
//! colour copy. Its copy is read from the artwork's pixels as the canvas shows them (strokes of
//! any brush, fills and smudges), so Compare asks the runtime for the copy's frame (`ReadJob`),
//! which draws it and reads it off the frame loop (S83, as the fits are), and the numbers come
//! when the reading does, the first Compare's kept in the record. How much of the original
//! shows over the copy is the user's (a slider); a combination's card compares one area at a
//! time, its outline marked on both frames; a value scale is painted a cell at a time, the cell
//! the pen lands in (Fill fills it, then the next).
//!
//! The eyedropper (`pick`): an original not compared yet reads nothing; a compared swatch its
//! colour; a value scale's given ends their colours; and while a colour combination or a colour
//! copy is compared, a point of either frame compares the original and the copy there (`Probe`):
//! the runtime reads the copy's pixel and hands it back (`probed`).

use std::sync::Arc;

use openpaint_core::Color;
use openpaint_practice::{Metrics, Original, Point, Rgb8, ScaleCheck};

use super::{Compared, PracticeSheet, unavailable};
use crate::error::AppError;
use crate::practice::{AreasRead, PracticeNotice, Probe, SHOWN};
use crate::practice_copy::PAPER;
use crate::practice_pictures::PictureImage;
use crate::projection::Availability;

/// The copy's frame for the runtime to draw and read, for the Compare numbered `sequence`:
/// canvas pixels from `origin`, `size` large, read as `original` asks (`run`).
#[derive(Debug, Clone, PartialEq)]
pub struct ReadJob {
    pub sequence: u64,
    pub origin: [u32; 2],
    pub size: [u32; 2],
    pub original: Original,
}

/// A painted copy as read: a colour combination's areas and the edges where it made a weak
/// contrast strong, or a value scale's cells checked.
#[derive(Debug, Clone, PartialEq)]
pub enum CopyRead {
    Areas(Arc<AreasRead>),
    Scale(Arc<ScaleCheck>),
}

impl ReadJob {
    /// The copy whose frame's pixels are `pixels` (8-bit RGBA on the sheet's paper) as read, with
    /// the numbers a first Compare keeps; none for an original that is not painted.
    #[must_use]
    pub fn run(&self, pixels: &PictureImage) -> Option<(CopyRead, Metrics)> {
        match &self.original {
            Original::Areas(areas) => {
                let copy = areas.read(pixels.size, &pixels.pixels, PAPER);
                let strong = (copy.contrasts.iter())
                    .filter(|c| c.made_strong())
                    .flat_map(|c| areas.edges(c.areas.0, Some(c.areas.1)))
                    .collect();
                let metrics = copy.metrics(areas);
                Some((
                    CopyRead::Areas(Arc::new(AreasRead { copy, strong })),
                    metrics,
                ))
            }
            Original::Scale(scale) => {
                let check = scale.read(pixels.size, &pixels.pixels, PAPER);
                let metrics = check.metrics();
                Some((CopyRead::Scale(Arc::new(check)), metrics))
            }
            _ => None,
        }
    }
}

/// What the eyedropper reads at a point of a practice sheet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PracticePick {
    /// The canvas, as anywhere.
    Canvas,
    /// Nothing: an original not compared yet.
    Refused,
    /// A compared swatch's colour, or a value scale's given end.
    Colour(Color),
    /// While a colour combination or a colour copy is compared: the copy at `copy` (canvas
    /// pixels) to be read and compared with the original's colour there; the eyedropper shows
    /// the original's when the point lies on it.
    Probe {
        copy: [f32; 2],
        original: Color,
        on_original: bool,
    },
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
    /// A value scale's cell the pen paints and Fill fills (an end never).
    cell: usize,
    /// The copy's pixels to read, how many reads the sheet asked and how many before this
    /// exercise came to show (a reading of an earlier exercise arriving late is dropped).
    job: Option<ReadJob>,
    reads: u64,
    since: u64,
    /// The point the eyedropper compares, waiting for the copy's pixel (frame units, the
    /// original's colour), and the last one compared.
    probing: Option<(Point, Rgb8)>,
    probe: Option<Probe>,
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
        }
    }
}

fn colour(rgb: Rgb8) -> Option<Color> {
    let [r, g, b] = rgb.map(|c| f32::from(c) / 255.0);
    Color::rgb(r, g, b).ok()
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
        match &self.sheet.originals[index] {
            Original::Areas(areas) => {
                self.painting_state.outline = Arc::new(areas.edges(0, None));
            }
            Original::Scale(_) => self.take_cell_at(index, 1),
            _ => {}
        }
    }

    /// Cell `cell` of exercise `index`'s value scale in hand: the pen paints it, Fill fills it.
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
    /// cell between the ends it lands in, or is refused.
    pub(super) fn take_cell(&mut self, index: usize, p: Point) -> Option<PracticeNotice> {
        let Original::Scale(scale) = &self.sheet.originals[index] else {
            return None;
        };
        match scale.cell_at(p) {
            Some(cell) if cell > 0 && cell + 1 < scale.cells => {
                self.take_cell_at(index, cell);
                None
            }
            _ => Some(PracticeNotice::DrawInCell),
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
        let Original::Areas(areas) = &self.sheet.originals[index] else {
            return Err(unavailable(Availability::NothingLeft));
        };
        if area >= areas.count() {
            return Err(unavailable(Availability::NothingLeft));
        }
        self.painting_state.outline = Arc::new(areas.edges(area, None));
        self.painting_state.area = area;
        // The card shows the area chosen, not the point compared before.
        self.painting_state.probe = None;
        Ok(())
    }

    /// A Compare begins: no point compared yet.
    pub(super) fn forget_probe(&mut self) {
        self.painting_state.probe = None;
        self.painting_state.probing = None;
    }

    /// The copy of exercise `index` asked of the runtime for Compare.
    pub(super) fn ask_read(&mut self, index: usize) -> Reading {
        let frame = self.sheet.pairs[index].copy;
        self.painting_state.reads += 1;
        self.painting_state.job = Some(ReadJob {
            sequence: self.painting_state.reads,
            origin: frame.origin.map(|v| v.round().max(0.0) as u32),
            size: frame.size.map(|v| v.round().max(1.0) as u32),
            original: self.sheet.originals[index].clone(),
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

    /// The last point the eyedropper compared, while comparing.
    pub(super) fn probe(&self) -> Option<Probe> {
        self.compared.as_ref().and(self.painting_state.probe)
    }

    /// What the eyedropper reads at `point` (canvas pixels).
    pub(crate) fn pick(&mut self, point: [f32; 2]) -> PracticePick {
        let point = point.map(f64::from);
        let revealed = self.revealed();
        let over = |copy: bool| {
            (self.sheet.pairs[..revealed].iter()).position(|pair| {
                let frame = if copy { pair.copy } else { pair.original };
                frame.contains(point, 0.0)
            })
        };
        if let Some(index) = over(false) {
            if self.record.result(index).is_none() {
                self.notice = Some(PracticeNotice::PickAfterCompare);
                return PracticePick::Refused;
            }
            return self.picked(index, point, true);
        }
        match over(true) {
            Some(index) => self.picked(index, point, false),
            None => PracticePick::Canvas,
        }
    }

    /// What the eyedropper reads at the canvas `point` of exercise `index`'s original's frame
    /// (compared) or its copy's frame.
    fn picked(&mut self, index: usize, point: Point, on_original: bool) -> PracticePick {
        let pair = self.sheet.pairs[index];
        let frame = if on_original {
            pair.original
        } else {
            pair.copy
        };
        let at = frame.to_frame(point);
        let original = match &self.sheet.originals[index] {
            Original::Colour(rgb) if on_original => Some(*rgb),
            Original::Scale(scale) if !on_original => {
                let end = scale
                    .cell_at(at)
                    .filter(|cell| *cell == 0 || cell + 1 == scale.cells);
                let given = end.map(|cell| scale.ends()[usize::from(cell > 0)]);
                return given
                    .and_then(colour)
                    .map_or(PracticePick::Canvas, PracticePick::Colour);
            }
            Original::Areas(areas) => Some(areas.colour_at(at, 0.0)),
            Original::Picture(_) if !self.drawn(index) => self.picture_colour(index, at),
            _ => None,
        };
        let Some((rgb, shown)) = original.and_then(|rgb| Some((rgb, colour(rgb)?))) else {
            return PracticePick::Canvas;
        };
        let painted = !matches!(self.sheet.originals[index], Original::Colour(_));
        if painted && self.compared.is_some() && self.current() == Some(index) {
            self.painting_state.probing = Some((at, rgb));
            PracticePick::Probe {
                copy: pair.copy.to_canvas(at).map(|v| v as f32),
                original: shown,
                on_original,
            }
        } else if on_original {
            PracticePick::Colour(shown)
        } else {
            PracticePick::Canvas
        }
    }

    /// A colour copy's picture's colour at the frame point `at`, as read for the screen.
    fn picture_colour(&self, index: usize, at: Point) -> Option<Rgb8> {
        let picture = self.sheet.pictures.get(index)?.as_ref()?;
        let (_, image) = self.images.iter().find(|(item, _)| *item == picture.item)?;
        let [x, y] = picture.fit.to_picture(at);
        let [w, h] = picture.size.map(f64::from);
        image.as_ref()?.colour_at([x / w, y / h])
    }

    /// The runtime read the copy's pixel at the point the eyedropper compares: `copy`.
    pub(crate) fn probed(&mut self, copy: Rgb8) {
        if let Some((at, original)) = self.painting_state.probing.take() {
            self.painting_state.probe = Some(Probe { at, original, copy });
        }
    }

    /// How much of the original shows, the area compared and its outline, for the overlay.
    pub(super) fn painted_overlay(&self) -> (f32, usize, Arc<Vec<Vec<Point>>>) {
        let state = &self.painting_state;
        (state.shown, state.area, Arc::clone(&state.outline))
    }
}
