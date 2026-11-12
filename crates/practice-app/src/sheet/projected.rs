//! What the screen draws of the open sheet for one frame (`PracticeProjection`).

use std::sync::Arc;

use openpaint_core::{Artwork, StrokeStyle};

use super::PracticeSheet;
use crate::projection::PracticeProjection;

impl PracticeSheet {
    pub(crate) fn projection(&self, artwork: &Artwork, brush: &StrokeStyle) -> PracticeProjection {
        let current = self.current();
        let result = current.and_then(|index| self.record.first(index));
        let compare = self.compare_offered(current, artwork);
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
            copy: self.record.is_copy(),
            current,
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
            lesson: self.lesson_shown(),
            kind: current.and_then(|index| self.kind(index)),
            how: self.instructions,
            judged: self.judged(),
            end: self.end(),
            marked: self.marked(),
        }
    }
}
