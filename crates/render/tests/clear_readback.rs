//! GPU test: a headless device clears an offscreen texture and reads it back.

use openpaint_render::{Core, RenderError};

/// 300 px rows are 1,200 bytes, so the copy pads each row to 1,280 and readback must strip it.
const SIZE: [u32; 2] = [300, 200];
const COLOR: [u8; 4] = [38, 41, 46, 255];
/// `cargo run -p openpaint-harness -- render-smoke` renders the same clear and prints this hash.
const GOLDEN: u64 = 0x261e_692a_5ece_1106;

#[test]
fn clear_reads_back_every_pixel_with_the_golden_hash() {
    let core = Core::headless().expect("a GPU adapter is required for render tests");
    let image = core.render_clear(SIZE, COLOR).expect("clear and readback");
    assert_eq!(image.size(), SIZE);
    let (pixels, rest) = image.pixels().as_chunks::<4>();
    assert_eq!((pixels.len(), rest.len()), (300 * 200, 0));
    assert!(pixels.iter().all(|pixel| *pixel == COLOR));
    assert_eq!(
        image.content_hash(),
        GOLDEN,
        "hash {:016x} on {}",
        image.content_hash(),
        core.adapter()
    );
}

#[test]
fn empty_target_is_rejected() {
    let core = Core::headless().expect("a GPU adapter is required for render tests");
    let result = core.render_clear([0, 16], COLOR);
    assert!(matches!(result, Err(RenderError::Invalid(_))), "{result:?}");
}
