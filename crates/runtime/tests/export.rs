//! GPU tests of export and the reference image through the whole frame loop: the PNG the
//! export writes holds exactly the canvas the screen shows through the view; the reference
//! picture's colours are picked exactly.

mod hand;
mod screen_map;
mod support;

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use hand::{Hand, assert_cold};
use openpaint_core::Color;
use openpaint_runtime::{
    HostAnswer, HostRequest, ImageFormat, ImageSink, ImageSource, PortError, Runtime, ToolKind,
};
use openpaint_store::{decode_image, encode_image};
use screen_map::{mapping, pixel_at};
use support::Nowhere;

/// 800 x 500 points at density 2.
const SIZE: [u32; 2] = [1600, 1000];
const PIXELS_PER_POINT: f32 = 2.0;
/// The canvas area starts under the top bar.
const BAR: u32 = 96;
/// Right of Gallery at the top bar's left end.
const ACTIONS: [f32; 2] = [258.0, 48.0];
/// Rows of the Actions panel.
const SAVE_PNG: [f32; 2] = [330.0, 337.0];
const REFERENCE: [f32; 2] = [400.0, 785.0];
/// The side bar's eyedropper.
const EYEDROPPER: [f32; 2] = [80.0, 428.0];
/// In the reference window's picture, first at the lower right: the red band.
const RED_BAND: [f32; 2] = [950.0, 600.0];

#[derive(Clone, Default)]
struct Memory(Arc<Mutex<Option<Vec<u8>>>>);

impl ImageSink for Memory {
    fn write(self: Box<Self>, bytes: &[u8]) -> Result<(), PortError> {
        *self.0.lock().expect("lock") = Some(bytes.to_vec());
        Ok(())
    }
}

struct Picture(Vec<u8>);

impl ImageSource for Picture {
    fn read(self: Box<Self>, _limit: u64) -> Result<Vec<u8>, PortError> {
        Ok(self.0)
    }
}

/// Frames until the workers are done.
fn settle(runtime: &mut Runtime) {
    let start = Instant::now();
    while runtime.storing() || runtime.frame_offscreen(SIZE).expect("frame") {
        assert!(
            start.elapsed() < Duration::from_secs(30),
            "workers too slow"
        );
        runtime.frame_offscreen(SIZE).expect("frame");
    }
}

fn line(y: f32) -> Vec<[f32; 2]> {
    (0..=12u8)
        .map(|i| [300.0 + 90.0 * f32::from(i), y + 20.0 * f32::from(i % 3)])
        .collect()
}

#[test]
fn the_exported_png_holds_exactly_what_the_screen_shows() {
    let core = openpaint_render::Core::headless().expect("a GPU adapter is required");
    let mut runtime = support::start(PIXELS_PER_POINT, SIZE, Box::new(Nowhere));
    let mut hand = Hand::new(SIZE);
    hand.gesture(&mut runtime, ToolKind::Stylus, &line(400.0));
    hand.gesture(&mut runtime, ToolKind::Stylus, &line(700.0));
    hand.tap(&mut runtime, ACTIONS);
    hand.tap(&mut runtime, SAVE_PNG);
    assert_eq!(
        runtime.host_request(),
        Some(HostRequest::SaveFile {
            name: "Untitled Artwork.png".into(),
            format: ImageFormat::Png
        })
    );
    let file = Memory::default();
    runtime.answer(HostAnswer::File(Box::new(file.clone())));
    settle(&mut runtime);
    let bytes = file.0.lock().expect("lock").take().expect("written");
    let (size, pixels) = decode_image(&bytes).expect("a PNG");
    let canvas = runtime.read_canvas().expect("readback").expect("canvas");
    assert_eq!(size, canvas.size(), "native size");
    assert!(
        pixels == canvas.pixels(),
        "the file is the canvas, pixel for pixel"
    );
    assert_cold(&core, &runtime, "after exporting");
    // On screen, right of the side bar and under the top bar, nothing covers the canvas: at
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
    let mut runtime = support::start(PIXELS_PER_POINT, SIZE, Box::new(Nowhere));
    let mut hand = Hand::new(SIZE);
    hand.tap(&mut runtime, ACTIONS);
    hand.tap(&mut runtime, REFERENCE);
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
    runtime.answer(HostAnswer::Image(Box::new(Picture(png))));
    settle(&mut runtime);
    let transfers = runtime
        .projection()
        .transfers()
        .cloned()
        .expect("transfers");
    assert!(transfers.reference_shown);
    assert_eq!(transfers.reference.expect("image").size(), [480, 320]);
    hand.tap(&mut runtime, ACTIONS);
    hand.tap(&mut runtime, EYEDROPPER);
    hand.tap(&mut runtime, RED_BAND);
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
