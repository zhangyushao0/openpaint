//! Practice packs of format 4 (S101): a composition's value masses (S96) and a crop's colour
//! areas round-trip in its pack's head, a shape's masses or a damaged map is refused, and a pack of
//! another format is refused on import and left out of the list as it is (S116).

#[path = "../../store/tests/common/test_dir.rs"]
mod test_dir;

use std::sync::Arc;

use openpaint_app::{ImageFormat, PortError};
use openpaint_practice::{
    Areas, Credit, Licence, Masses, PERSONAL, Pack, PackItem, PictureKind, Tier, ValueMasses,
};
use openpaint_practice_store::{PACK_FORMAT, Packs, encode_pack, open_pack, pack_picture};
use openpaint_store::encode_image;
use test_dir::TestDir;

/// A composition of a landscape and its value masses, and its picture's file.
fn composition(id: &str) -> (PackItem, Vec<u8>) {
    let (size, pixels) = Masses::generate(4, 0.75).picture(160);
    let file = encode_image(ImageFormat::Png, size, &pixels).expect("a picture");
    let picture = pack_picture(&file).expect("as a pack holds it");
    let item = PackItem {
        id: id.to_owned(),
        kind: PictureKind::Composition,
        tier: Tier::Medium,
        size: picture.size,
        title: "A landscape".to_owned(),
        credit: None,
        outline: None,
        masses: Some(Arc::new(ValueMasses::of(picture.size, &picture.rgba))),
        areas: None,
    };
    (item, picture.file)
}

#[test]
fn value_masses_round_trip_and_only_a_compositions_whole_map_is_kept() {
    let (item, file) = composition("valley");
    let pack = Pack {
        id: PERSONAL.to_owned(),
        title: "Mine".to_owned(),
        items: vec![item.clone()],
    };
    let bytes = encode_pack(&pack, std::slice::from_ref(&file)).expect("encoded");
    assert_eq!(PACK_FORMAT, 4);
    assert_eq!(bytes[14..18], PACK_FORMAT.to_le_bytes());
    let (opened, _) = open_pack(&bytes).expect("opened");
    assert_eq!(opened, pack, "the masses as made");
    // A shape never holds them; a map not of its size is no map.
    let mut shape = pack.clone();
    shape.items[0].kind = PictureKind::Shape;
    assert!(encode_pack(&shape, std::slice::from_ref(&file)).is_err());
    let mut short = pack;
    let mut masses = (*item.masses.expect("masses")).clone();
    masses.values.pop();
    short.items[0].masses = Some(Arc::new(masses));
    assert!(encode_pack(&short, std::slice::from_ref(&file)).is_err());
}

#[test]
fn a_pack_of_another_format_is_refused_on_import_and_left_out_of_the_list_untouched() {
    let (item, file) = composition("valley");
    let pack = Pack {
        id: "old-compositions".to_owned(),
        title: "Old compositions".to_owned(),
        items: vec![PackItem {
            credit: credited(),
            ..item
        }],
    };
    let mut three = encode_pack(&pack, std::slice::from_ref(&file)).expect("encoded");
    three[14..18].copy_from_slice(&3_u32.to_le_bytes());
    let dir = TestDir::new("packs-other-format");
    let packs = Packs::at(&dir);
    assert_eq!(
        packs.import(&three),
        Err(PortError::OtherFormat { found: Some(3) })
    );
    assert!(!packs.path(&pack.id).exists(), "nothing kept");
    // One kept beside the library (format 3, 1.14.0 to 1.18.1): not listed, its bytes as found.
    std::fs::write(packs.path(&pack.id), &three).expect("kept as 1.18.1 left it");
    assert_eq!(packs.list(), Ok(Vec::new()));
    assert_eq!(std::fs::read(packs.path(&pack.id)).expect("kept"), three);
}

#[test]
fn a_crop_s_areas_round_trip_and_a_damaged_map_is_refused() {
    let (item, file) = composition("rust");
    let areas = Areas::of_map(
        [3, 2],
        vec![0, 0, 1, 2, 2, 1],
        vec![[90, 60, 40], [96, 66, 44], [200; 3]],
    );
    let crop = PackItem {
        kind: PictureKind::Crop,
        credit: credited(),
        masses: None,
        areas: areas.map(Arc::new),
        ..item
    };
    let pack = Pack {
        id: "test-crops".to_owned(),
        title: "Test crops".to_owned(),
        items: vec![crop.clone()],
    };
    let bytes = encode_pack(&pack, std::slice::from_ref(&file)).expect("encoded");
    let (opened, _) = open_pack(&bytes).expect("opened");
    assert_eq!(opened, pack, "the areas as found");
    let mut bare = pack.clone();
    bare.items[0].areas = None;
    assert!(
        encode_pack(&bare, std::slice::from_ref(&file)).is_err(),
        "a crop has its areas"
    );
    // A map naming an area it has no colour for is damage.
    let colours = bytes
        .windows(9)
        .position(|w| w == [90, 60, 40, 96, 66, 44, 200, 200, 200]);
    let mut damaged = bytes;
    damaged[colours.expect("the colours") - 4] = 2;
    assert!(open_pack(&damaged).is_err());
}

/// An official item's credit.
fn credited() -> Option<Credit> {
    Some(Credit {
        source: "https://example.org/test".to_owned(),
        creator: "The tests".to_owned(),
        licence: Licence::Cc0,
        attribution: "“A test”, the tests.".to_owned(),
        notes: "made for the tests".to_owned(),
    })
}
