//! `bench export` (S59, `bench_app.rs`): the open artwork exported as Save PNG… exports it, at
//! 1x, 2x and 4x, on the bench's thread: each export's wall time, its bands drawn (planned and
//! submitted, then the GPU and the readback) and the encoding apart, and what drew it (the raster
//! core's regions, or the evaluator's dispatches).

use std::time::Instant;

use openpaint_app::ImageFormat;

use crate::error::RuntimeError;
use crate::frame::Runtime;
use crate::transfer::encoded;

impl Runtime {
    /// `export`: the artwork at 1x, 2x and 4x into PNG files kept in memory.
    pub(crate) fn bench_export(&mut self, lines: &mut Vec<String>) -> Result<(), RuntimeError> {
        let (_, artwork) = self.bench_artwork()?;
        for scale in [1, 2, 4] {
            let before = self.core.counts();
            let started = Instant::now();
            let png = encoded(&self.core, &artwork, ImageFormat::Png, scale)
                .map_err(|_| RuntimeError::NotNow("an export that renders"))?;
            let wall = started.elapsed().as_secs_f64() * 1e3;
            let counts = self.core.counts().since(before);
            lines.push(format!(
                "export at {scale}x in {wall:.0} ms: {}; raster regions {}, dispatches {}",
                png.line(),
                counts.rastered,
                counts.dispatches
            ));
        }
        Ok(())
    }
}
