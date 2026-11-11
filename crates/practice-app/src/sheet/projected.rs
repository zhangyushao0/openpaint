//! What the screen draws of the open sheet for one frame (`PracticeProjection`).

use std::sync::Arc;

use openpaint_core::{Artwork, StrokeStyle};
use openpaint_practice::{Lesson, summary};

use super::PracticeSheet;
use crate::projection::PracticeProjection;

impl PracticeSheet {
    pub(crate) fn projection(&self, artwork: &Artwork, brush: &StrokeStyle) -> PracticeProjection {
        let current = self.current();
        let result = current.and_then(|index| self.record.first(index));
        let compare = self.compare_offered(current, artwork);
        let place = current.map_or((0, 0), |index| self.record.place(index));
        let before: Vec<&Lesson> = self.before.iter().collect();
        let pen = self.pen_style(brush).map(|style| (self.pen, style.size()));
        let painted = self.painted_overlay();
        let (overlay, card, greys) = current
            .and_then(|_| self.overlay())
            .map_or((None, Vec::new(), None), |(marks, card, greys)| {
                (Some(marks), card, greys)
            });
        PracticeProjection {
            sheet: Arc::clone(&self.sheet),
            revealed: self.revealed(),
            original: current.and_then(|index| self.original(index).cloned()),
            stage: self.record.stage(),
            copy: self.record.is_copy(),
            number: self.sheet.number,
            drill: self.record.drilled(),
            current,
            family: current.and_then(|index| self.family(index)),
            warming_up: current.is_some_and(|index| self.record.warming_up(index)),
            place,
            last: current.is_some_and(|index| self.ends(index)),
            compare,
            comparing: self.compared.is_some(),
            aligned: self.aligned,
            construction: self.construction && self.construction_offered(),
            all_tools: self.all_tools,
            pen,
            result: result.cloned(),
            overlay,
            card,
            greys,
            instructions: self.instructions,
            summary: (self.record.lesson().filter(|_| current.is_none()))
                .map(|this| summary(this, &before)),
            finished: self.finished(),
            notice: self.notice,
            aligned_offered: current.is_some_and(|index| self.aligned_offered(index)),
            fitting: self.fitting(),
            images: self.images.clone(),
            grounded: self.grounded.clone(),
            cell: self.cell(),
            probe: self.probe(),
            compared: (self.compared.as_ref()).map(|compared| compared.shown().clone()),
            shown: painted.0,
            area: painted.1,
            grey: painted.3,
        }
    }
}
