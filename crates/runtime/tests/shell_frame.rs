//! GPU test: one runtime frame, drawn offscreen, shows the top bar over the blank canvas and the
//! side bar over its left edge.

mod support;

use support::Nowhere;

const SIZE: [u32; 2] = [640, 400];
/// Density 2: the 48-point top bar covers the top 96 pixel rows.
const PIXELS_PER_POINT: f32 = 2.0;
const BAR_ROWS: usize = 96;
/// The side bar (8 + 64 points from the left edge) and some room.
const SIDE_BAR_COLUMNS: usize = 160;
const PAPER: [u8; 4] = [255; 4];

fn row(pixels: &[[u8; 4]], y: usize) -> &[[u8; 4]] {
    let width = SIZE[0] as usize;
    &pixels[y * width..(y + 1) * width]
}

#[test]
fn frame_shows_the_top_bar_over_the_canvas() {
    let mut runtime = support::start(PIXELS_PER_POINT, SIZE, Box::new(Nowhere));
    let image = runtime.screenshot(SIZE).expect("offscreen frame");
    assert_eq!(image.size(), SIZE);
    let (pixels, _) = image.pixels().as_chunks::<4>();
    let bar = row(pixels, BAR_ROWS / 2);
    assert!(
        bar.iter().all(|pixel| *pixel != PAPER),
        "the top bar covers its rows"
    );
    for y in [BAR_ROWS, SIZE[1] as usize - 1] {
        assert!(
            row(pixels, y)[SIDE_BAR_COLUMNS..]
                .iter()
                .all(|pixel| *pixel == PAPER),
            "row {y} shows the canvas right of the side bar"
        );
    }
    let side = &row(pixels, BAR_ROWS + 40)[..SIDE_BAR_COLUMNS];
    assert!(
        side.iter().any(|pixel| *pixel != PAPER),
        "the side bar floats over the canvas's left edge"
    );
    let label = (0..BAR_ROWS)
        .flat_map(|y| row(pixels, y))
        .any(|pixel| pixel != &bar[0]);
    assert!(label, "the top bar carries a label");
    let canvas = runtime.artwork().expect("the canvas is open").canvas();
    assert_eq!([canvas.width(), canvas.height()], [SIZE[0], SIZE[1] - 96]);
}

#[test]
fn second_frame_reuses_the_ui_textures() {
    let mut runtime = support::start(PIXELS_PER_POINT, SIZE, Box::new(Nowhere));
    let first = runtime.screenshot(SIZE).expect("first frame");
    let second = runtime.screenshot(SIZE).expect("second frame");
    assert_eq!(first.content_hash(), second.content_hash());
}
