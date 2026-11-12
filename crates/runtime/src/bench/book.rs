//! `bench book` (S89): in the gallery, the practice book opened by a finger's tap on the Today
//! card's sheet, its page growing out of the card, and closed by its Gallery button, going back
//! into the card; twice, the first opening laying out its words for the first time. Measured as
//! `ui` measures (`motion.rs`).

use crate::error::RuntimeError;
use crate::frame::Runtime;
use crate::input::ToolKind;

impl Runtime {
    /// `book`: see the module.
    pub(crate) fn bench_book(&mut self, lines: &mut Vec<String>) -> Result<(), RuntimeError> {
        if self.screen.gallery_mut().is_none() {
            return Err(RuntimeError::NotNow("the gallery"));
        }
        self.bench_rest()?;
        lines.push("frames drawn of the vsyncs wanting one (120 Hz); ms".to_owned());
        for when in ["first", "again"] {
            let opens = self.bench_tap("Practice book", ToolKind::Finger)?;
            lines.push(format!("book opens, {when}: {opens}"));
            let closes = self.bench_tap("Home", ToolKind::Finger)?;
            lines.push(format!("book closes, {when}: {closes}"));
        }
        Ok(())
    }
}
