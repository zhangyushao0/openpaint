//! `bench undo`: a large change, undone and redone, as a hand does: the layer holding the most
//! strokes hidden, the hiding undone, redone and undone again (so the artwork shows as before),
//! each through the session's intents at fit, then frames until the canvas is complete and still:
//! how long each fill took and what its frames cost.

use openpaint_app::{HistoryIntent, Intent, LayerIntent, ViewIntent};

use super::hand::{busiest_layer, strokes};
use super::{Watch, reached};
use crate::error::RuntimeError;
use crate::frame::Runtime;

impl Runtime {
    /// `undo`: the busiest layer hidden, then undo, redo and undo, each until complete.
    pub(crate) fn bench_undo(&mut self, lines: &mut Vec<String>) -> Result<(), RuntimeError> {
        let (_, artwork) = self.bench_artwork()?;
        let layer = busiest_layer(&artwork)?;
        lines.push(format!(
            "the layer of {} strokes (of {}) hidden, then undone, redone, undone",
            layer.strokes().len(),
            strokes(&artwork)
        ));
        let id = layer.id();
        let mut watch = Watch::paced(self);
        self.bench_intents(vec![Intent::View(ViewIntent::Fit)], &mut watch)?;
        self.bench_until(&mut watch, Self::bench_settled)?;
        let hide = Intent::Layer(LayerIntent::SetVisible {
            layer: id,
            visible: false,
        });
        let undo = || Intent::History(HistoryIntent::Undo);
        let steps = [
            ("hidden", hide),
            ("undone", undo()),
            ("redone", Intent::History(HistoryIntent::Redo)),
            ("undone again", undo()),
        ];
        for (name, intent) in steps {
            let mut watch = Watch::paced(self);
            self.bench_intents(vec![intent], &mut watch)?;
            let complete = self.bench_until(&mut watch, Self::bench_settled)?;
            let state = reached(complete, "complete");
            lines.push(format!("{name}: {state} {}", watch.line(self)));
        }
        Ok(())
    }
}
