//! Practice packs of format 4 (S101): a composition's value masses (S96) and a crop's colour
//! areas round-trip in its pack's head, a shape's masses or a damaged map is refused, and a pack of
//! format 2 (1.4.0 to 1.13.0) or 3 (1.14.0 to 1.18.1) is upgraded as it is imported and where it
//! is kept (the personal pack among them), its items and pictures as they were.

#[path = "common/test_dir.rs"]
mod test_dir;

use std::sync::Arc;

use openpaint_app::ImageFormat;
use openpaint_practice::{
    Areas, Credit, Licence, Masses, PERSONAL, Pack, PackItem, PictureKind, Tier, ValueMasses,
};
use openpaint_store::{PACK_FORMAT, Packs, encode_image, encode_pack, open_pack, pack_picture};
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

/// `text` as a pack's head holds it: its length, then its bytes.
fn text(head: &mut Vec<u8>, text: &str) {
    head.extend_from_slice(&(text.len() as u32).to_le_bytes());
    head.extend_from_slice(text.as_bytes());
}

/// `pack` (no value masses, no outlines) and its pictures as 1.13.0 wrote them (`format` 2, an
/// item's head ending at its outline's count) or 1.18.1 (3, its value masses none after it).
fn older(pack: &Pack, pictures: &[Vec<u8>], format: u32) -> Vec<u8> {
    let mut head = Vec::new();
    text(&mut head, &pack.id);
    text(&mut head, &pack.title);
    head.extend_from_slice(&(pack.items.len() as u32).to_le_bytes());
    for (item, picture) in pack.items.iter().zip(pictures) {
        text(&mut head, &item.id);
        head.extend_from_slice(&[item.kind.code(), item.tier.code()]);
        head.extend_from_slice(&item.size[0].to_le_bytes());
        head.extend_from_slice(&item.size[1].to_le_bytes());
        text(&mut head, &item.title);
        match &item.credit {
            Some(credit) => {
                head.push(1);
                text(&mut head, &credit.source);
                text(&mut head, &credit.creator);
                head.push(credit.licence.code());
                text(&mut head, &credit.attribution);
                text(&mut head, &credit.notes);
            }
            None => head.push(0),
        }
        head.extend_from_slice(&0_u32.to_le_bytes());
        if format == 3 {
            head.extend_from_slice(&[0; 8]);
        }
        head.extend_from_slice(&(picture.len() as u32).to_le_bytes());
        head.extend_from_slice(&crc32fast::hash(picture).to_le_bytes());
    }
    let mut file = b"OpenPaintPack\0".to_vec();
    file.extend_from_slice(&format.to_le_bytes());
    file.extend_from_slice(&(head.len() as u32).to_le_bytes());
    file.extend_from_slice(&head);
    file.extend_from_slice(&crc32fast::hash(&head).to_le_bytes());
    for picture in pictures {
        file.extend_from_slice(picture);
    }
    file
}

#[test]
fn a_format_two_pack_is_upgraded_where_it_is_kept_and_as_it_is_imported() {
    let (mut item, file) = composition("valley");
    item.masses = None;
    let personal = Pack {
        id: PERSONAL.to_owned(),
        title: "My pictures".to_owned(),
        items: vec![item.clone()],
    };
    let pictures = vec![file.clone()];
    let kept = older(&personal, &pictures, 2);
    assert!(open_pack(&kept).is_err(), "no reader but the upgrade's");
    let dir = TestDir::new("packs-upgrade");
    let packs = Packs::at(&dir);
    std::fs::write(packs.path(PERSONAL), &kept).expect("kept as 1.13.0 left it");
    // Listed: upgraded in place, its items and pictures as they were.
    assert_eq!(packs.list(), Ok(vec![personal.clone()]));
    let upgraded = std::fs::read(packs.path(PERSONAL)).expect("the pack");
    assert_eq!(upgraded[14..18], PACK_FORMAT.to_le_bytes());
    let (opened, read) = open_pack(&upgraded).expect("opened");
    assert_eq!((opened, read), (personal, vec![file.as_slice()]));
    // Imported: an official pack of format 2 (one left in Download before) is upgraded first.
    item.credit = Some(Credit {
        source: "https://example.org/valley".to_owned(),
        creator: "The tests".to_owned(),
        licence: Licence::Cc0,
        attribution: "“A landscape”, the tests.".to_owned(),
        notes: "a landscape".to_owned(),
    });
    let official = Pack {
        id: "old-compositions".to_owned(),
        title: "Old compositions".to_owned(),
        items: vec![item],
    };
    let imported = packs.import(&older(&official, &pictures, 2));
    assert_eq!(imported, Ok(official.clone()));
    let kept = std::fs::read(packs.path(&official.id)).expect("kept");
    assert_eq!(kept[14..18], PACK_FORMAT.to_le_bytes());
}

#[test]
fn a_format_three_pack_is_upgraded_its_masses_kept_and_no_areas() {
    let (item, file) = composition("valley");
    let pack = Pack {
        id: "old-compositions".to_owned(),
        title: "Old compositions".to_owned(),
        items: vec![PackItem {
            masses: None,
            credit: credited(),
            ..item
        }],
    };
    let dir = TestDir::new("packs-upgrade-three");
    let packs = Packs::at(&dir);
    let three = older(&pack, std::slice::from_ref(&file), 3);
    assert!(open_pack(&three).is_err(), "no reader but the upgrade's");
    assert_eq!(packs.import(&three), Ok(pack.clone()));
    let kept = std::fs::read(packs.path(&pack.id)).expect("kept");
    assert_eq!(kept[14..18], PACK_FORMAT.to_le_bytes());
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
