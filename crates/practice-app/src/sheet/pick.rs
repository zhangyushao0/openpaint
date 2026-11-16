//! The eyedropper on a practice sheet (`read.rs`), as the shown exercise's kind's
//! facts say (`Picking`, S109b, S123): an original not compared yet reads nothing; a compared
//! swatch its colour; a value scale's given ends their colours; and while a colour combination, a
//! colour copy or a whole copy's colour blocks (S96) are compared, a point of either frame
//! compares the original and the copy there (`Probe`): the runtime reads the copy's pixel and
//! hands it back (`probed`), a whole copy's bare paper comparing nothing. Value masses compare
//! greys, not colours: nothing. A key shift's original is never picked, its copy as the canvas
//! (S107), nor a picture laid under abstract analysis's marks (S112). A swatch given beside a frame
//! is picked whole, its colour (a gamut mask's corners, S153). Over an exercise before the one in
//! hand its original is made again by its generator
//! for the pick, the one place a sheet still remakes an exercise not in hand.

use std::sync::Arc;

use openpaint_core::Color;
use openpaint_practice::{BESIDE_SIDE, Figure, In, Original, Picking, Point, Rgb8, beside};

use super::PracticeSheet;
use crate::copy::PAPER;
use crate::projection::{PracticeNotice, Probe};

/// What the eyedropper reads at a point of a practice sheet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PracticePick {
    /// The canvas, as anywhere.
    Canvas,
    /// Nothing: an original not compared yet, or one never picked.
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

    /// What the eyedropper reads over exercise `index`'s frames.
    fn picks(&self, index: usize) -> Picking {
        self.facts(index)
            .map_or(Picking::default(), |facts| facts.picks)
    }

    /// What the eyedropper reads at `point` (canvas pixels).
    pub fn pick(&mut self, point: [f32; 2]) -> PracticePick {
        let point = point.map(f64::from);
        let revealed = self.revealed();
        let over = |copy: bool| {
            (self.sheet.pairs[..revealed].iter()).position(|pair| {
                let frame = if copy { pair.copy } else { pair.original };
                frame.contains(point, 0.0)
            })
        };
        if let Some(index) = over(false) {
            // A key shift's original is never picked (S107), and no notice says so (S146).
            if !self.picks(index).original {
                return PracticePick::Refused;
            }
            if self.record.first(index).is_none() {
                self.notice = Some(PracticeNotice::PickAfterCompare);
                return PracticePick::Refused;
            }
            return self.picked(index, point, true);
        }
        match over(true) {
            // A picture laid under the marks is an original too, never picked (S112).
            Some(index) if self.facts(index).is_some_and(|facts| facts.under) => {
                PracticePick::Refused
            }
            Some(index) => self.picked(index, point, false),
            None => (self.beside(point, revealed))
                .and_then(colour)
                .map_or(PracticePick::Canvas, PracticePick::Colour),
        }
    }

    /// The colour of a swatch given beside a frame of the first `revealed` exercises at the
    /// canvas `point` (`Figure::Beside`, S153: a gamut mask's corners), picked whole.
    fn beside(&self, point: Point, revealed: usize) -> Option<Rgb8> {
        let half = BESIDE_SIDE / 2.0;
        let pairs = self
            .sheet
            .pairs
            .iter()
            .zip(&self.sheet.givens)
            .take(revealed);
        pairs
            .flat_map(|(pair, marks)| marks.iter().map(move |mark| (pair, mark)))
            .find_map(|(pair, mark)| {
                let Figure::Beside { at, colour, .. } = mark.drawn else {
                    return None;
                };
                let frame = match mark.on {
                    In::Original => pair.original,
                    In::Copy => pair.copy,
                };
                let centre = beside(&frame, at);
                let reach = half * frame.unit();
                let within = (0..2).all(|axis| (point[axis] - centre[axis]).abs() <= reach);
                within.then_some(colour)
            })
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
        let given = |sheet: &Self| sheet.made(index).and_then(|made| made.colour_at(at));
        let picks = self.picks(index);
        if picks.ends && !on_original {
            let end = given(self).and_then(colour);
            return end.map_or(PracticePick::Canvas, PracticePick::Colour);
        }
        let original = if picks.swatch {
            (on_original.then(|| self.made(index)).flatten()).and_then(|made| made.swatch())
        } else if picks.probed {
            given(self).or_else(|| self.picture_colour(index, at))
        } else {
            None
        };
        let Some((rgb, shown)) = original.and_then(|rgb| Some((rgb, colour(rgb)?))) else {
            return PracticePick::Canvas;
        };
        let painted = !picks.swatch;
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

    /// Exercise `index`'s original: the one in hand's, or an earlier one's made again by its
    /// generator for the pick (a known limit: not read from what the sheet printed).
    fn made(&self, index: usize) -> Option<Arc<dyn Original>> {
        match self.original(index) {
            Some(original) => Some(Arc::clone(original)),
            None => Some(self.record.laid(index)?.exercise.original()),
        }
    }

    /// A pack picture's colour at the frame point `at`, as read for the screen.
    fn picture_colour(&self, index: usize, at: Point) -> Option<Rgb8> {
        let picture = self.sheet.pictures.get(index)?.as_ref()?;
        let (_, image) = self.images.iter().find(|(item, _)| *item == picture.item)?;
        let [x, y] = picture.fit.to_picture(at);
        let [w, h] = picture.size.map(f64::from);
        image.as_ref()?.colour_at([x / w, y / h])
    }

    /// The runtime read the copy's pixel at the point the eyedropper compares: `copy`. Where bare
    /// paper compares nothing (a whole copy's colour blocks are compared where painted, S96), a
    /// point left bare compares nothing.
    pub fn probed(&mut self, copy: Rgb8) {
        let blocks = (self.current()).is_some_and(|index| self.picks(index).bare);
        let bare = copy == PAPER && blocks;
        if let Some((at, original)) = self.painting_state.probing.take() {
            self.painting_state.probe = (!bare).then_some(Probe { at, original, copy });
        }
    }
}
