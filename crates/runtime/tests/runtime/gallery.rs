//! GPU test of the gallery through the whole frame loop on a real library: leaving the canvas
//! stores a thumbnail that matches the canvas, the title field asks for the keyboard and takes
//! typed text, and an artwork opened again equals a cold evaluation.

use openpaint_render::{thumbnail_reference, thumbnail_size};
use openpaint_runtime::{EditKey, TextInput, ToolKind};
use openpaint_store::Library;

use crate::support::{TestDir, assert_cold, core, gallery, start, until};

/// 800 x 500 points at density 2.
const SIZE: [u32; 2] = [1600, 1000];
const PIXELS_PER_POINT: f32 = 2.0;
/// The largest thumbnail the runtime makes.
const THUMBNAIL_MAX: [u32; 2] = [448, 336];

#[test]
fn the_gallery_shows_a_matching_thumbnail_and_renames_by_keyboard() {
    let core = core();
    let dir = TestDir::new("gallery-rename");
    let (mut runtime, mut hand) = start(PIXELS_PER_POINT, SIZE, Box::new(Library::at(&dir)));
    let wave: Vec<[f32; 2]> = (0..=30u8)
        .map(|i| {
            [
                // Right of the side bar.
                200.0 + 45.0 * f32::from(i),
                500.0 + 200.0 * (f32::from(i) / 4.0).sin(),
            ]
        })
        .collect();
    hand.gesture(&mut runtime, ToolKind::Stylus, &wave);
    let canvas = runtime.read_canvas().expect("readback").expect("canvas");
    assert!(runtime.back(), "back leaves the canvas");
    until(&mut runtime, &mut hand, |runtime| {
        gallery(runtime).is_some()
    });
    let listed = gallery(&runtime).expect("gallery");
    let thumbnail = listed.items()[0].thumbnail.clone().expect("a thumbnail");
    let size = thumbnail_size(canvas.size(), THUMBNAIL_MAX);
    assert_eq!(thumbnail.size(), size);
    assert!(
        thumbnail.pixels() == thumbnail_reference(&canvas, size).as_slice(),
        "the thumbnail does not match the canvas"
    );
    assert!(!runtime.back(), "back in the gallery is the system's");

    // The only card's title, below its picture.
    hand.tap(&mut runtime, "Rename artwork");
    assert!(
        runtime.keyboard_wanted(),
        "the title field asks for the keyboard"
    );
    runtime.text(TextInput::Commit("Hafen 墨".into()));
    runtime.text(TextInput::Key(EditKey::Enter));
    until(&mut runtime, &mut hand, |runtime| {
        gallery(runtime).is_some_and(|g| g.items()[0].title.as_str() == "Hafen 墨")
    });
    assert!(
        !runtime.keyboard_wanted(),
        "the keyboard goes when the field closes"
    );

    hand.tap(&mut runtime, "Open artwork");
    until(&mut runtime, &mut hand, |runtime| {
        runtime.artwork().is_some()
    });
    assert_cold(&core, &mut runtime, "reopened from the gallery");
    let reopened = runtime.read_canvas().expect("readback").expect("canvas");
    assert_eq!(reopened.content_hash(), canvas.content_hash());
}
