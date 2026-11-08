//! `bench practice` on stage 4's value masses (S96), in a session holding them after its
//! division: Middle chosen and Fill frame tapped (the picture's place filled), two dark masses and
//! a light one painted by the pen, then Compare opened (the copy drawn and read on the compare
//! worker, the answer's greys, edges and hatching shown when it arrives), the slider dragged and
//! Compare closed: a line each.

use openpaint_app::Pen;

use super::motion::drag;
use super::practice::of_key;
use crate::error::RuntimeError;
use crate::frame::Runtime;
use crate::input::ToolKind;

/// The slider dragged, ms.
const SLIDE_MS: i64 = 400;
/// The masses painted, by pen, frame units.
const MASSES: [(Pen, [[f64; 2]; 2]); 3] = [
    (Pen::Dark, [[0.15, 0.6], [0.4, 0.62]]),
    (Pen::Dark, [[0.6, 0.3], [0.62, 0.55]]),
    (Pen::Light, [[0.2, 0.2], [0.8, 0.22]]),
];

impl Runtime {
    /// See the module.
    pub(super) fn bench_masses(&mut self, lines: &mut Vec<String>) -> Result<(), RuntimeError> {
        if !self.bench_on_to(of_key("value-masses")?)? {
            lines.push("no whole copy or value masses after this point of the session".to_owned());
            return Ok(());
        }
        let line = self.bench_tap("Middle", ToolKind::Finger)?;
        lines.push(format!("value masses' Middle: {line}"));
        let line = self.bench_tap("Fill frame", ToolKind::Finger)?;
        lines.push(format!("value masses' Fill frame: {line}"));
        for (pen, line) in MASSES {
            self.bench_pen_line(pen, line)?;
        }
        let line = self.bench_tap("Compare", ToolKind::Finger)?;
        lines.push(format!(
            "value masses compare opens (read on the worker): {line}"
        ));
        let ([left, top], [width, height]) = self.bench_rect("Original shown")?;
        let middle = top + height / 2.0;
        let slide = [[left + width / 2.0, middle], [left + width - 2.0, middle]];
        let shown = self.bench_measured(drag(&slide, SLIDE_MS))?;
        lines.push(format!("value masses' original shown dragged: {shown}"));
        let line = self.bench_tap("Compare", ToolKind::Finger)?;
        lines.push(format!("value masses compare closes: {line}"));
        Ok(())
    }
}
