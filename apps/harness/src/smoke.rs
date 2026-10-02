//! `render-smoke`: open a headless GPU device, clear an offscreen target, read it back, write
//! the PNG and print its hash.

use openpaint_render::Core;

use crate::error::HarnessError;
use crate::output::write_png;

/// The same clear as `crates/render/tests/clear_readback.rs`, so the printed hash is that
/// test's golden value.
const SIZE: [u32; 2] = [300, 200];
const COLOR: [u8; 4] = [38, 41, 46, 255];

pub fn run() -> Result<(), HarnessError> {
    let core = Core::headless()?;
    let image = core.render_clear(SIZE, COLOR)?;
    let path = write_png("smoke.png", &image)?;
    println!("adapter {}", core.adapter());
    println!("wrote {}", path.display());
    println!("hash {:016x}", image.content_hash());
    Ok(())
}
