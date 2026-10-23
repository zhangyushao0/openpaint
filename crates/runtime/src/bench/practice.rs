//! `bench practice`: a practice sheet's motions as a hand makes them, each measured as the display
//! shows it (`motion.rs`), on the practice sheet open: How's card grown out of its word and gone
//! back, then Next three times, the view gliding to the next pair. Between motions the frames
//! still wanted, then a pause as the hand moves on. The session moves on three exercises.

use super::Watch;
use crate::error::RuntimeError;
use crate::frame::Runtime;
use crate::input::ToolKind;

impl Runtime {
    /// `practice`: see the module.
    pub(crate) fn bench_practice(&mut self, lines: &mut Vec<String>) -> Result<(), RuntimeError> {
        let practising = (self.screen.canvas()).is_some_and(|c| c.session().is_practice());
        if !practising {
            return Err(RuntimeError::NotNow("a practice sheet open"));
        }
        let mut watch = Watch::paced(self);
        self.bench_drawn(&mut watch)?;
        self.bench_settle()?;
        self.bench_rest()?;
        lines.push("frames drawn of the vsyncs wanting one (120 Hz); ms".to_owned());
        for when in ["opens", "closes"] {
            let line = self.bench_tap("How", ToolKind::Finger)?;
            lines.push(format!("instructions {when}: {line}"));
        }
        for next in 1..=3 {
            let line = self.bench_tap("Next", ToolKind::Finger)?;
            lines.push(format!("next {next}: {line}"));
        }
        self.bench_rest()
    }
}
