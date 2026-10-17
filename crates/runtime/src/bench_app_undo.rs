//! `bench undo` (S62, `bench_app.rs`): a large change, undone and redone, as a hand does: the
//! layer holding the most strokes hidden, the hiding undone, redone and undone again (so the
//! artwork shows as before), each through the session's intents at fit, then frames until the
//! canvas is complete and still: how long each fill took and what its frames cost.

use std::time::Duration;

use openpaint_app::{HistoryIntent, Intent, LayerIntent, ViewIntent};

use super::Watch;
use super::cases::strokes;
use crate::error::RuntimeError;
use crate::frame::Runtime;
use crate::screen::Screen;

const LIMIT: Duration = Duration::from_secs(60);

impl Runtime {
    /// `undo`: the busiest layer hidden, then undo, redo and undo, each until complete.
    pub(crate) fn bench_undo(&mut self, lines: &mut Vec<String>) -> Result<(), RuntimeError> {
        let (_, artwork) = self.bench_artwork()?;
        let layer = (artwork.layers().iter())
            .max_by_key(|layer| layer.strokes().len())
            .ok_or(RuntimeError::NotNow("a layer"))?;
        lines.push(format!(
            "the layer of {} strokes (of {}) hidden, then undone, redone, undone",
            layer.strokes().len(),
            strokes(&artwork)
        ));
        let id = layer.id();
        let mut watch = Watch::paced(self);
        self.bench_change(vec![Intent::View(ViewIntent::Fit)], &mut watch)?;
        self.bench_until(&mut watch, LIMIT, Self::bench_settled)?;
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
            self.bench_change(vec![intent], &mut watch)?;
            let complete = self.bench_until(&mut watch, LIMIT, Self::bench_settled)?;
            let state = if complete {
                "complete after"
            } else {
                "NOT complete within the limit:"
            };
            lines.push(format!("{name}: {state} {}", watch.line(self)));
        }
        Ok(())
    }

    /// `intents` applied to the open canvas as a frame applies a hand's, then a frame drawn.
    fn bench_change(
        &mut self,
        intents: Vec<Intent>,
        watch: &mut Watch,
    ) -> Result<(), RuntimeError> {
        if let Screen::Canvas(canvas) = &mut self.screen {
            let app = (&mut self.toolbox, &mut self.clipboard, &mut self.seeds);
            canvas.dispatch(&self.core, intents, app);
        }
        self.bench_drawn(watch)
    }
}
