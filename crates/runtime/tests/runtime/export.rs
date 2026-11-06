//! GPU tests of export and the reference image through the whole frame loop: the PNG the
//! export writes holds the canvas the screen shows through the view (within one 8-bit level away
//! from edges since S59: the raster core draws the file); the reference picture's colours are
//! picked exactly.

use openpaint_core::Color;
use openpaint_render::{RgbaImage, image_difference};
use openpaint_runtime::{BytesSource, HostAnswer, HostRequest, ImageFormat, ToolKind};
use openpaint_store::{decode_image, encode_image};

use crate::support::{Nowhere, Written, assert_cold, core, mapping, pixel_at, settle, start};

/// 800 x 600 points at density 2: the Actions panel, with S29's scale row, fits.
const SIZE: [u32; 2] = [1600, 1200];
const PIXELS_PER_POINT: f32 = 2.0;
/// The canvas fits under the top band.
const BAR: u32 = 96;

/// A pen path of 13 points stepping up and down from `y` across the canvas.
fn zigzag(y: f32) -> Vec<[f32; 2]> {
    (0..=12u8)
        .map(|i| [300.0 + 90.0 * f32::from(i), y + 20.0 * f32::from(i % 3)])
        .collect()
}

#[test]
fn the_exported_png_holds_exactly_what_the_screen_shows() {
    let core = core();
    let (mut runtime, mut hand) = start(PIXELS_PER_POINT, SIZE, Box::new(Nowhere::default()));
    hand.gesture(&mut runtime, ToolKind::Stylus, &zigzag(400.0));
    hand.gesture(&mut runtime, ToolKind::Stylus, &zigzag(700.0));
    hand.tap(&mut runtime, "Actions");
    hand.tap(&mut runtime, "Save PNG…");
    assert_eq!(
        runtime.host_request(),
        Some(HostRequest::SaveFile {
            name: "Untitled Artwork.png".into(),
            format: ImageFormat::Png
        })
    );
    let file = Written::default();
    runtime.answer(HostAnswer::File(Box::new(file.clone())));
    settle(&mut runtime, &mut hand);
    let bytes = file.bytes();
    let (size, pixels) = decode_image(&bytes).expect("a PNG");
    let canvas = runtime.read_canvas().expect("readback").expect("canvas");
    assert_eq!(size, canvas.size(), "native size");
    // The raster core draws the file and the canvas: at 1x the same pixels (D93).
    let file = RgbaImage::from_rgba(size, pixels).expect("whole");
    let found = image_difference(&canvas, &file).expect("same size");
    assert_eq!(found.largest, [0; 4], "the file is the canvas: {found:?}");
    assert_cold(&core, &mut runtime, "after exporting");
    // On screen, right of the side bar and under the top band, nothing covers the canvas: at
    // fit, every screen pixel whose canvas neighbourhood is one colour shows that colour.
    let screen = runtime.screenshot(SIZE).expect("screen");
    let map = mapping(&runtime, SIZE, PIXELS_PER_POINT);
    let mut compared = 0;
    for y in (BAR..SIZE[1]).step_by(5) {
        for x in (200..SIZE[0]).step_by(5) {
            let at = [x as f32 + 0.5, y as f32 + 0.5];
            let [cx, cy] = map.to_canvas(at).map(|v| v - 0.5);
            let inside = cx >= 0.0 && cy >= 0.0 && cx + 1.0 < size[0] as f32;
            if !inside || cy + 1.0 >= size[1] as f32 {
                continue;
            }
            let texels = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]]
                .map(|[dx, dy]| pixel_at(&canvas, [cx + dx, cy + dy]));
            if texels.iter().all(|texel| *texel == texels[0]) {
                assert_eq!(pixel_at(&screen, at), texels[0], "screen pixel {at:?}");
                compared += 1;
            }
        }
    }
    assert!(compared > 30_000, "{compared} pixels compared");
}

#[test]
fn the_eyedropper_picks_the_reference_pictures_colour_exactly() {
    let (mut runtime, mut hand) = start(PIXELS_PER_POINT, SIZE, Box::new(Nowhere::default()));
    hand.tap(&mut runtime, "Actions");
    hand.tap(&mut runtime, "Reference");
    assert_eq!(runtime.host_request(), Some(HostRequest::OpenImage));
    // 480 x 320: red, green, blue and yellow bands.
    let bands = [[220, 40, 40], [40, 170, 70], [40, 80, 220], [240, 200, 40]];
    let pixels: Vec<u8> = (0..320 * 480)
        .flat_map(|i: u32| {
            let [r, g, b] = bands[(i % 480 / 120) as usize];
            [r, g, b, 255]
        })
        .collect();
    let png = encode_image(ImageFormat::Png, [480, 320], &pixels).expect("png");
    runtime.answer(HostAnswer::Image(Box::new(BytesSource(png))));
    settle(&mut runtime, &mut hand);
    let transfers = runtime
        .projection()
        .transfers()
        .cloned()
        .expect("transfers");
    assert!(transfers.reference_shown);
    assert_eq!(transfers.reference.expect("image").size(), [480, 320]);
    hand.tap(&mut runtime, "Actions");
    hand.tap(&mut runtime, "Eyedropper");
    // In the reference window's picture (first at the lower right), its left quarter: the red band.
    hand.tap(&mut runtime, ("Reference picture", [0.13, 0.22]));
    let toolbox = runtime.projection().brush().cloned().expect("toolbox");
    let red = Color::rgb(220.0 / 255.0, 40.0 / 255.0, 40.0 / 255.0).expect("colour");
    assert_eq!(toolbox.color, red);
    assert!(!toolbox.picking, "the pick settled");
    let artwork = runtime.artwork().expect("artwork");
    assert_eq!(
        artwork.layers()[0].strokes().len(),
        0,
        "picking draws nothing"
    );
}
