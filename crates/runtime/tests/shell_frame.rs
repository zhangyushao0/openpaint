//! GPU test: one runtime frame, drawn offscreen, shows the blank canvas at fit under the top
//! band on the desk, the top islands at the band's ends with the desk between them (nothing
//! spans the width), and the side bar over the canvas's left edge.

mod gpu;
mod support;

use support::Nowhere;

const SIZE: [u32; 2] = [1280, 800];
/// Density 2: the 48-point top band is the top 96 pixel rows.
const PIXELS_PER_POINT: f32 = 2.0;
const BAND_ROWS: usize = 96;
/// The row through the middle of the top islands (8 to 57 points down).
const ISLAND_ROW: usize = 66;
/// Under the islands, above the fitted canvas.
const UNDER_ISLANDS_ROW: usize = 118;
/// The side bar (8 + 66.5 points from the left edge) and some room.
const SIDE_BAR_COLUMNS: usize = 160;
const PAPER: [u8; 4] = [255; 4];
const SURROUND: [u8; 4] = openpaint_ui::design::SURROUND;

fn row(pixels: &[[u8; 4]], y: usize) -> &[[u8; 4]] {
    let width = SIZE[0] as usize;
    &pixels[y * width..(y + 1) * width]
}

#[test]
fn frame_shows_the_islands_over_the_desk_and_the_canvas_under_them() {
    let mut runtime = support::start(PIXELS_PER_POINT, SIZE, Box::new(Nowhere));
    let image = runtime.screenshot(SIZE).expect("offscreen frame");
    assert_eq!(image.size(), SIZE);
    let (pixels, _) = image.pixels().as_chunks::<4>();
    let islands = row(pixels, ISLAND_ROW);
    let width = SIZE[0] as usize;
    // The left island ends near x = 630 since S28 added the Adjustments wand.
    assert!(
        islands[640..720].iter().all(|pixel| *pixel == SURROUND),
        "the desk shows between the islands"
    );
    for (end, columns) in [("left", 20..120), ("right", width - 120..width - 20)] {
        assert!(
            islands[columns].iter().any(|pixel| *pixel != SURROUND),
            "the {end} island carries its controls"
        );
    }
    // At fit the canvas fills 92 % of the area under the band, centred: paper across the middle
    // row right of the side bar, the desk above, below and beside it.
    let middle = BAND_ROWS + (SIZE[1] as usize - BAND_ROWS) / 2;
    assert!(
        row(pixels, middle)[SIDE_BAR_COLUMNS..1200]
            .iter()
            .all(|pixel| *pixel == PAPER),
        "the middle row shows the canvas right of the side bar"
    );
    for y in [UNDER_ISLANDS_ROW, SIZE[1] as usize - 1] {
        assert!(
            row(pixels, y)[SIDE_BAR_COLUMNS..]
                .iter()
                .all(|pixel| *pixel != PAPER),
            "row {y} shows the desk around the fitted canvas"
        );
    }
    assert_eq!(row(pixels, middle)[width - 1], SURROUND);
    let side = &row(pixels, middle)[..SIDE_BAR_COLUMNS];
    assert!(
        side.iter().any(|pixel| *pixel != PAPER),
        "the side bar floats over the canvas's left edge"
    );
    let canvas = runtime.artwork().expect("the canvas is open").canvas();
    assert_eq!(
        [canvas.width(), canvas.height()],
        [SIZE[0], SIZE[1] - BAND_ROWS as u32]
    );
}

#[test]
fn second_frame_reuses_the_ui_textures() {
    let mut runtime = support::start(PIXELS_PER_POINT, SIZE, Box::new(Nowhere));
    let first = runtime.screenshot(SIZE).expect("first frame");
    let second = runtime.screenshot(SIZE).expect("second frame");
    assert_eq!(first.content_hash(), second.content_hash());
}
