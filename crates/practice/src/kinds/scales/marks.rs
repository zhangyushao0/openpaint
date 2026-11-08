//! A value scale's marks (S97), in the one frame drawn in: its cells, the ends filled as given, the
//! cell in hand outlined in ink; compared, the even scale as a row under the copy's, cell under cell
//! (a hued scale keeping its hue), a cell of the copy's that is uneven outlined red together with
//! its even one. The card says how many cells are uneven.

use crate::kinds::scales::scale::{ROW, ScaleCheck, ValueScale};
use crate::marks::card::{CardItem, Say};
use crate::marks::compared::{Compared, Seen};
use crate::marks::mark::{Figure, In, SheetMark};
use crate::maths::path::Point;

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
}
