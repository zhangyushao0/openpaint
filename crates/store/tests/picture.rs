//! An inserted picture in the library (S26): the fixture PNG, read as the transfer worker reads
//! it, on a layer of its own as a placed patch with nothing lifted; saved, it reopens with every
//! pixel and its placement, and needs no format change (an S20 transform record).

use std::path::PathBuf;

use openpaint_app::{ProjectStore, Title};
use openpaint_core::{Artwork, ArtworkCommand, CanvasSize, LayerCommand, Placement};
use openpaint_store::{Library, open_picture};

const CUTOUT: &[u8] = include_bytes!("../../../tests/fixtures/s26-cutout.png");
const PHOTO: &[u8] = include_bytes!("../../../tests/fixtures/s26-photo.jpg");

fn inserted(artwork: &Artwork, bytes: &[u8], placement: Placement) -> Artwork {
    let picture = open_picture(bytes, artwork.canvas()).expect("a picture");
    let layered = artwork
        .apply(ArtworkCommand::Layer(LayerCommand::Create))
        .expect("layer")
        .artwork;
    layered
        .transform_region(None, picture, placement)
        .expect("placed")
        .artwork
}

#[test]
fn the_fixtures_open_fitted_upright_and_with_their_transparency() {
    let canvas = CanvasSize::new(1600, 1000).expect("canvas");
    let cutout = open_picture(CUTOUT, canvas).expect("cutout");
    assert_eq!((cutout.origin(), cutout.size()), ([480, 290], [640, 420]));
    let alphas: Vec<u8> = cutout.pixels().iter().skip(3).step_by(4).copied().collect();
    assert!(alphas.contains(&0) && alphas.contains(&255));
    assert!(alphas.iter().any(|a| (1..255).contains(a)), "a soft glow");
    let photo = open_picture(PHOTO, canvas).expect("photo");
    // 1800 x 1200 turned upright is 1200 x 1800, fitted into 1000 rows.
    assert_eq!((photo.origin(), photo.size()), ([466, 0], [667, 1000]));
    // The sun lies in the upper part, right of the middle.
    let at = |x: u32, y: u32| (((y * 667) + x) * 4) as usize;
    let sun = &photo.pixels()[at(455, 183)..at(455, 183) + 4];
    assert!(sun[0] > 200 && sun[2] < 120, "the sun on top: {sun:?}");
}

#[test]
fn an_inserted_picture_reopens_with_every_pixel_and_its_place() {
    // A directory of its own under cargo's temporary directory for tests.
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("store-picture-{}", std::process::id()));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).expect("clear an old test directory");
    }
    let blank = Artwork::new(CanvasSize::new(900, 600).expect("canvas"));
    let placed = Placement::around([450.0, 300.0], 0.8, 0.3, [true, false]).expect("placed");
    let one = inserted(&blank, CUTOUT, Placement::IDENTITY);
    let two = inserted(&one, PHOTO, placed);
    let mut library = Library::at(&dir);
    let id = library.create(&blank, &Title::untitled()).expect("create");
    for version in [&one, &two] {
        library.save(id, version).expect("save");
        drop(library);
        library = Library::at(&dir);
        let reopened = library.open_last().expect("opens").expect("a project");
        assert_eq!(reopened.artwork.to_parts(), version.to_parts());
    }
}
