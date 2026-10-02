//! `shell-smoke`: one runtime frame drawn offscreen at tablet size and density, written as a PNG
//! so the top bar over the canvas can be looked at.

use crate::error::HarnessError;
use crate::launch::launch;
use crate::output::write_png;

/// A 10-inch landscape tablet: 2560 x 1600 pixels at density 2.
const SIZE: [u32; 2] = [2560, 1600];
const PIXELS_PER_POINT: f32 = 2.0;

pub fn run() -> Result<(), HarnessError> {
    let mut runtime = launch("shell", PIXELS_PER_POINT, SIZE)?;
    let image = runtime.screenshot(SIZE)?;
    let path = write_png("shell.png", &image)?;
    println!("wrote {}", path.display());
    println!("hash {:016x}", image.content_hash());
    Ok(())
}
