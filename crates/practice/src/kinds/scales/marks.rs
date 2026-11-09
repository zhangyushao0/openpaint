//! A value scale's marks (S97), in the one frame drawn in: its cells, the ends filled as given, the
//! cell in hand outlined in ink; compared, the even scale as a row under the copy's, cell under cell
//! (a hued scale keeping its hue), a cell of the copy's that is uneven outlined red together with
//! its even one. The card says how many cells are uneven.

use crate::kinds::scales::scale::{ROW, ScaleCheck, ValueScale};
use crate::marks::asked::{Asked, Compare, Read, Reading, Reads};
use crate::marks::card::{CardItem, Say};
use crate::marks::compared::{Compared, Comparison, Seen, Verdict};
use crate::marks::mark::{Figure, In, SheetMark};
use crate::maths::colour::Rgb8;
use crate::maths::path::Point;

/// A value scale's copy, read from its frame's pixels on the compare worker: each cell's lightness
/// against its even step.
#[derive(Debug, Clone, PartialEq)]
pub struct ScaleReading {
    pub scale: ValueScale,
}

impl Reads for ScaleReading {
    fn unread(&self) -> Comparison {
        Comparison::of(ScaleCompared {
            scale: self.scale,
            check: None,
        })
    }

    fn read(&self, size: [u32; 2], pixels: &[u8], paper: Rgb8) -> Read {
        let check = self.scale.read(size, pixels, paper);
        let metrics = check.metrics();
        Read {
            comparison: Comparison::of(ScaleCompared {
                scale: self.scale,
                check: Some(check),
            }),
            metrics,
        }
    }
}

/// A value scale's copy compared, once anything lies in its frame: read from its pixels.
pub(crate) fn compare(asked: &Asked<'_>) -> Option<Compare> {
    if !asked.held() {
        return None;
    }
    let scale = *asked.original.scale()?;
    Some(Compare::Read(Reading::of(ScaleReading { scale })))
}

/// The ends filled as given and every cell outlined.
pub(crate) fn givens(scale: &ValueScale) -> Vec<SheetMark> {
    let targets = scale.targets();
    let ends = [0, scale.cells - 1].map(|end| filled(scale.cell(end), scale.colour(targets[end])));
    let cells = (0..scale.cells).map(|cell| Figure::Cell {
        cell,
        rect: scale.cell(cell),
    });
    (ends.into_iter().chain(cells))
        .map(|figure| SheetMark::free(In::Copy, figure))
        .collect()
}

/// `rect` (top left, bottom right) filled with `colour`.
fn filled([[left, top], [right, bottom]]: [Point; 2], colour: [u8; 3]) -> Figure {
    let corners = vec![[left, top], [right, top], [right, bottom], [left, bottom]];
    Figure::Fill { corners, colour }
}

/// A value scale compared: the copy's cells as checked, none until its pixels are read.
#[derive(Debug, Clone, PartialEq)]
pub struct ScaleCompared {
    pub scale: ValueScale,
    pub check: Option<ScaleCheck>,
}

impl Compared for ScaleCompared {
    fn marks(&self, _seen: &Seen<'_>) -> Vec<SheetMark> {
        let scale = &self.scale;
        let mut figures = Vec::new();
        for (cell, target) in scale.targets().iter().enumerate() {
            let even = scale.even_cell(cell);
            figures.push(filled(even, scale.colour(*target)));
            figures.push(Figure::Rule(even));
        }
        for cell in self.check.iter().flat_map(|check| &check.uneven) {
            let [[left, _], [right, bottom]] = scale.even_cell(*cell);
            figures.push(Figure::Uneven([[left, ROW[1]], [right, bottom]]));
        }
        (figures.into_iter())
            .map(|figure| SheetMark::free(In::Copy, figure))
            .collect()
    }

    fn card(&self, _seen: &Seen<'_>) -> Vec<CardItem> {
        match &self.check {
            None => vec![CardItem::Reading],
            Some(check) if !check.uneven.is_empty() => {
                vec![CardItem::Says(Say::counting(
                    "cells-uneven",
                    &[check.uneven.len()],
                ))]
            }
            Some(_) => Vec::new(),
        }
    }

    /// Once read, its check's (`ScaleCheck::verdict`); unread, no verdict yet.
    fn verdict(&self) -> Verdict {
        self.check
            .as_ref()
            .map_or(Verdict::Incomplete, ScaleCheck::verdict)
    }
}
