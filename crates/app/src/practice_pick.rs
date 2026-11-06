//! The eyedropper on a practice sheet (`practice_read.rs`): an original not compared yet reads
//! nothing; a compared swatch its colour; a value scale's given ends their colours; and while a
//! colour combination, a colour copy or a whole copy's colour blocks (S96) are compared, a point
//! of either frame compares the original and the copy there (`Probe`): the runtime reads the
//! copy's pixel and hands it back (`probed`). Value masses compare greys, not colours: nothing.
//! A key shift's original is never picked, its copy as the canvas (S107).

use openpaint_core::Color;
use openpaint_practice::{Family, Original, Point, Rgb8};

use super::PracticeSheet;
use crate::practice::{PracticeNotice, Probe};
use crate::practice_copy::PAPER;

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

fn colour(rgb: Rgb8) -> Option<Color> {
    let [r, g, b] = rgb.map(|c| f32::from(c) / 255.0);
    Color::rgb(r, g, b).ok()
}

impl PracticeSheet {
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
            // A key shift's original is never picked (S107): its colours are judged by eye.
            if self.key_shift(index) {
                self.notice = Some(PracticeNotice::OriginalLocked);
                return PracticePick::Refused;
            }
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
            // A colour copy's picture, and a whole copy's where its colour blocks are compared
            // (S96); value masses compare greys, not colours; a key shift's copy is picked as the
            // canvas, its original never (S107).
            Original::Picture(_)
                if matches!(
                    self.family(index),
                    Some(Family::ValueMasses | Family::KeyShift)
                ) =>
            {
                None
            }
            Original::Picture(_) if !self.drawn(index) => self.picture_colour(index, at),
            Original::Picture(_) if self.family(index) == Some(Family::WholeCopy) => {
                self.picture_colour(index, at)
            }
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

    /// The runtime read the copy's pixel at the point the eyedropper compares: `copy`. On a whole
    /// copy a point left bare compares nothing (its colour blocks are compared where painted,
    /// S96).
    pub(crate) fn probed(&mut self, copy: Rgb8) {
        let bare = copy == PAPER
            && (self.current()).and_then(|index| self.family(index)) == Some(Family::WholeCopy);
        if let Some((at, original)) = self.painting_state.probing.take() {
            self.painting_state.probe = (!bare).then_some(Probe { at, original, copy });
        }
    }
}
