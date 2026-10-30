//! Practice packs in the store: a pack file round-trips, every damaged byte is refused, an
//! imported pack is kept beside the library and read back, its manifest and one picture at a
//! time, and the library beside it keeps every byte.

mod common;
mod sample;

use common::{TestDir, records, untitled, write_records};
use openpaint_app::{ImageFormat, PortError, ProjectStore};
use openpaint_practice::{
    Credit, ItemRef, Licence, Outline, PERSONAL, Pack, PackItem, PictureKind, Tier,
};
use openpaint_store::{
    Library, PACK_FORMAT, Packs, encode_image, encode_pack, open_pack, pack_picture, shown_picture,
};
use sample::{decode, encode, parts, sample_editor, sample_path, sample_thumbnail, sample_title};

/// A picture of `size`, its pixels from `seed`, as a PNG or (`jpeg`) a JPEG file.
fn picture(size: [u32; 2], seed: u8, jpeg: bool) -> Vec<u8> {
    let pixels: Vec<u8> = (0..size[0] * size[1])
        .flat_map(|i| [seed.wrapping_mul(i as u8), 90, 200 - seed, 255])
        .collect();
    let format = if jpeg {
        ImageFormat::Jpeg
    } else {
        ImageFormat::Png
    };
    encode_image(format, size, &pixels).expect("a picture")
}

fn item(id: &str, tier: Tier, size: [u32; 2], licence: Licence) -> PackItem {
    PackItem {
        id: id.to_owned(),
        kind: PictureKind::Shape,
        tier,
        size,
        title: format!("Object {id}"),
        credit: Some(Credit {
            source: format!("https://example.org/objects/{id}"),
            creator: "Unknown maker, Ohio".to_owned(),
            licence,
            attribution: format!("“Object {id}” by Unknown maker, {}", licence.name()),
            notes: "one clear outline".to_owned(),
        }),
        outline: None,
        masses: None,
    }
}

/// A pack of three items and their pictures (two PNGs, a JPEG), the first with an outline.
fn sample(id: &str) -> (Pack, Vec<Vec<u8>>) {
    let mut fox = item("fox", Tier::Entry, [6, 4], Licence::CcBy4);
    fox.outline = Some(Outline {
        contours: vec![
            vec![[0.5, 0.5], [5.5, 0.25], [5.75, 3.5], [0.25, 3.75]],
            vec![[2.0, 1.0], [3.0, 1.0], [2.5, 2.0]],
        ],
    });
    let pack = Pack {
        id: id.to_owned(),
        title: "Shapes for tests".to_owned(),
        items: vec![
            fox,
            item("teapot", Tier::Simple, [5, 7], Licence::Cc0),
            item("helmet", Tier::Hard, [16, 8], Licence::PdUsGov),
        ],
    };
    let pictures = vec![
        picture([6, 4], 3, false),
        picture([5, 7], 5, false),
        picture([16, 8], 7, true),
    ];
    (pack, pictures)
}

fn named(pack: &str, item: &str) -> ItemRef {
    ItemRef {
        pack: pack.to_owned(),
        item: item.to_owned(),
    }
}

#[test]
fn a_pack_file_round_trips_and_is_the_same_bytes_every_time() {
    let (pack, pictures) = sample("test-shapes");
    let bytes = encode_pack(&pack, &pictures).expect("encoded");
    assert_eq!(encode_pack(&pack, &pictures), Ok(bytes.clone()));
    assert_eq!(&bytes[..14], b"OpenPaintPack\0");
    assert_eq!(bytes[14..18], PACK_FORMAT.to_le_bytes());
    let (opened, read) = open_pack(&bytes).expect("opened");
    assert_eq!(opened, pack);
    assert_eq!(read, pictures.iter().map(Vec::as_slice).collect::<Vec<_>>());
}

#[test]
fn every_damaged_byte_and_any_other_file_is_refused() {
    let (pack, pictures) = sample("test-shapes");
    let bytes = encode_pack(&pack, &pictures).expect("encoded");
    for at in 0..bytes.len() {
        let mut damaged = bytes.clone();
        damaged[at] ^= 0x20;
        assert!(open_pack(&damaged).is_err(), "byte {at} changed");
    }
    for cut in [0, 10, 40, bytes.len() - 1] {
        assert!(open_pack(&bytes[..cut]).is_err(), "cut at {cut}");
    }
    let longer = [bytes.as_slice(), &[0]].concat();
    assert!(open_pack(&longer).is_err(), "a byte after the last picture");
    // A later format, and S78's unreleased format 1 (no outlines): refused, saying so.
    for other in [PACK_FORMAT + 1, 1] {
        let mut changed = bytes.clone();
        changed[14..18].copy_from_slice(&other.to_le_bytes());
        assert_eq!(
            open_pack(&changed).map(|_| ()),
            Err(PortError::OtherFormat { found: Some(other) })
        );
    }
    assert!(open_pack(b"OpenPaint\0 an artwork file").is_err());
}

#[test]
fn a_pack_breaking_a_rule_or_its_pictures_is_never_written() {
    let (mut pack, pictures) = sample("test-shapes");
    assert!(
        encode_pack(&pack, &pictures[..2]).is_err(),
        "a picture short"
    );
    let mut swapped = pictures.clone();
    swapped.swap(0, 1);
    assert!(
        encode_pack(&pack, &swapped).is_err(),
        "sizes not the items'"
    );
    let mut other = pictures.clone();
    other[0] = b"GIF89a, no PNG or JPEG".to_vec();
    assert!(encode_pack(&pack, &other).is_err());
    if let Some(credit) = pack.items[0].credit.as_mut() {
        credit.attribution.clear();
    }
    assert!(
        encode_pack(&pack, &pictures).is_err(),
        "CC BY needs its credit"
    );
    let (mut personal, pictures) = sample(PERSONAL);
    assert!(
        encode_pack(&personal, &pictures).is_err(),
        "credited pictures of one's own"
    );
    for item in &mut personal.items {
        item.credit = None;
    }
    let bytes = encode_pack(&personal, &pictures).expect("the personal pack");
    assert_eq!(open_pack(&bytes).map(|(pack, _)| pack), Ok(personal));
    let dir = TestDir::new("packs-personal");
    assert!(Packs::at(&dir).import(&bytes).is_err(), "never imported");
}

#[test]
fn a_picture_of_one_s_own_joins_the_personal_pack_with_its_outline() {
    let dir = TestDir::new("packs-mine");
    let packs = Packs::at(&dir);
    let ground = |x: u32, y: u32| {
        if (x as i32 - 40).pow(2) + (y as i32 - 30).pow(2) < 400 {
            [40u8, 30, 30, 255]
        } else {
            [235, 235, 235, 255]
        }
    };
    let pixels: Vec<u8> = (0..60)
        .flat_map(|y| (0..80).flat_map(move |x| ground(x, y)))
        .collect();
    let file = encode_image(ImageFormat::Png, [80, 60], &pixels).expect("a picture");
    let picture = pack_picture(&file).expect("taken");
    assert_eq!(picture.size, [80, 60]);
    let first = packs
        .add_picture(&picture, (None, Tier::Entry))
        .expect("added");
    assert_eq!((first.id.as_str(), first.items.len()), (PERSONAL, 1));
    let outline = Outline {
        contours: vec![vec![[20.0, 10.0], [60.0, 10.0], [40.0, 50.0]]],
    };
    let second = packs
        .add_picture(&picture, (Some(outline.clone()), Tier::Hard))
        .expect("added");
    // Named by the picture's bytes, the same picture again apart.
    let id = format!("mine-{:08x}", crc32fast::hash(&picture.file));
    let ids: Vec<&str> = second.items.iter().map(|item| item.id.as_str()).collect();
    assert_eq!(ids, [id.clone(), format!("{id}-2")]);
    let titles: Vec<&str> = second
        .items
        .iter()
        .map(|item| item.title.as_str())
        .collect();
    assert_eq!(titles, ["Picture 1", "Picture 2"]);
    let tiers: Vec<Tier> = second.items.iter().map(|item| item.tier).collect();
    assert_eq!(tiers, [Tier::Entry, Tier::Hard], "the tiers chosen");
    assert_eq!(second.items[1].outline, Some(outline));
    assert!(second.items.iter().all(|item| item.credit.is_none()));
    assert_eq!(packs.list(), Ok(vec![second.clone()]));
    let named = named(PERSONAL, &format!("{id}-2"));
    let kept = packs.picture(&named).expect("read").expect("held");
    let shown = shown_picture(&kept).expect("decoded");
    assert_eq!(shown.size, [80, 60]);
}

/// A picture of one's own moved to another tier, then removed: the pack written whole again
/// without it, its file gone with the last; one it does not hold changes nothing.
#[test]
fn a_picture_of_one_s_own_changes_tier_and_goes() {
    let dir = TestDir::new("packs-mine-removed");
    let packs = Packs::at(&dir);
    let file = |grey: u8| {
        let pixels: Vec<u8> = (0..40 * 30).flat_map(|_| [grey, grey, grey, 255]).collect();
        pack_picture(&encode_image(ImageFormat::Png, [40, 30], &pixels).expect("png"))
            .expect("taken")
    };
    let (dark, light) = (file(40), file(200));
    packs
        .add_picture(&dark, (None, Tier::Simple))
        .expect("added");
    let both = packs
        .add_picture(&light, (None, Tier::Medium))
        .expect("added");
    let [first, second] = [0, 1].map(|at| both.items[at].id.clone());
    let moved = packs
        .set_tier(&first, Tier::Hard)
        .expect("moved")
        .expect("kept");
    assert_eq!(moved.items[0].tier, Tier::Hard);
    assert_eq!(packs.list(), Ok(vec![moved.clone()]));
    assert_eq!(
        packs.set_tier("mine-none", Tier::Entry),
        Ok(Some(moved.clone()))
    );
    let left = packs
        .remove_picture(&first)
        .expect("removed")
        .expect("one left");
    assert_eq!(left.items.len(), 1);
    assert_eq!(left.items[0].id, second);
    assert_eq!(packs.picture(&named(PERSONAL, &first)), Ok(None));
    let kept = packs.picture(&named(PERSONAL, &second)).expect("read");
    assert_eq!(kept, Some(light.file.clone()));
    assert_eq!(packs.remove_picture("mine-none"), Ok(Some(left)));
    assert_eq!(packs.remove_picture(&second), Ok(None), "none left");
    assert!(!packs.path(PERSONAL).exists(), "its file goes");
    assert_eq!(packs.list(), Ok(Vec::new()));
}

#[test]
fn an_imported_pack_is_kept_beside_the_library_whose_bytes_stay() {
    // The frozen format-1 library (every record kind, `tests/format.rs`) and packs beside it.
    let dir = TestDir::new("packs");
    let frozen = std::fs::read(sample_path()).expect("the format-1 sample");
    write_records(&dir.join("library"), &decode(&frozen));

    let packs = Packs::at(dir.join("packs"));
    assert_eq!(packs.list(), Ok(Vec::new()), "none before the first import");
    let (pack, pictures) = sample("test-shapes");
    let bytes = encode_pack(&pack, &pictures).expect("encoded");
    assert_eq!(packs.import(&bytes), Ok(pack.clone()));
    assert_eq!(std::fs::read(packs.path("test-shapes")).ok(), Some(bytes));
    assert_eq!(packs.list(), Ok(vec![pack.clone()]));
    for (item, picture) in pack.items.iter().zip(&pictures) {
        let read = packs.picture(&named("test-shapes", &item.id));
        assert_eq!(read, Ok(Some(picture.clone())), "{}", item.id);
    }
    assert_eq!(packs.picture(&named("test-shapes", "lamp")), Ok(None));
    assert_eq!(packs.picture(&named("other-pack", "fox")), Ok(None));
    assert_eq!(packs.picture(&named("../library", "fox")), Ok(None));

    let library = records(&dir.join("library"));
    assert!(encode(&library) == frozen, "format 1's bytes as they were");
    let mut library = Library::at(dir.join("library"));
    let project = library.open_last().expect("opens").expect("the sample");
    assert_eq!(project.artwork.to_parts(), parts());
    let listed = library.list().expect("listed");
    assert_eq!(listed[0].title, sample_title());
    assert_eq!(listed[0].thumbnail, Some(sample_thumbnail()));
    assert_eq!(library.editor(), Ok(Some(sample_editor())));
    library
        .create(&project.artwork, &untitled())
        .expect("the library takes artworks as before");
    assert_eq!(
        packs.list(),
        Ok(vec![pack]),
        "the pack beside it as imported"
    );
}

#[test]
fn a_new_edition_replaces_the_kept_one_and_a_refused_import_changes_nothing() {
    let dir = TestDir::new("packs-edition");
    let packs = Packs::at(&dir);
    let (pack, pictures) = sample("test-shapes");
    packs
        .import(&encode_pack(&pack, &pictures).expect("encoded"))
        .expect("imported");
    let (mut second, mut more) = sample("test-shapes");
    second.items.truncate(1);
    more.truncate(1);
    let (other, other_pictures) = sample("more-shapes");
    for (pack, pictures) in [(&second, &more), (&other, &other_pictures)] {
        let bytes = encode_pack(pack, pictures).expect("encoded");
        packs.import(&bytes).expect("imported");
    }
    assert_eq!(packs.list(), Ok(vec![other, second]));
    assert_eq!(packs.picture(&named("test-shapes", "helmet")), Ok(None));

    let kept = std::fs::read(packs.path("test-shapes")).expect("kept");
    let mut damaged = kept.clone();
    let last = damaged.len() - 1;
    damaged[last] ^= 1;
    assert!(packs.import(&damaged).is_err());
    assert_eq!(std::fs::read(packs.path("test-shapes")).ok(), Some(kept));
}

#[test]
fn a_damaged_kept_pack_is_left_out_and_its_pictures_refused() {
    let dir = TestDir::new("packs-damaged");
    let packs = Packs::at(&dir);
    let (pack, pictures) = sample("test-shapes");
    let bytes = encode_pack(&pack, &pictures).expect("encoded");
    packs.import(&bytes).expect("imported");
    let (other, other_pictures) = sample("more-shapes");
    let more = encode_pack(&other, &other_pictures).expect("encoded");
    packs.import(&more).expect("imported");

    // The last picture's last byte: the head still reads, the picture fails its sum.
    let mut cut_picture = bytes.clone();
    let last = cut_picture.len() - 1;
    cut_picture[last] ^= 1;
    std::fs::write(packs.path("test-shapes"), &cut_picture).expect("damaged");
    assert_eq!(packs.list().map(|list| list.len()), Ok(2));
    assert_eq!(
        packs.picture(&named("test-shapes", "helmet")),
        Err(PortError::Corrupt {
            what: "practice pack"
        })
    );
    assert_eq!(
        packs.picture(&named("test-shapes", "fox")),
        Ok(Some(pictures[0].clone()))
    );
    // A byte of the head: the pack is left out of the list.
    let mut cut_head = bytes;
    cut_head[30] ^= 1;
    std::fs::write(packs.path("test-shapes"), &cut_head).expect("damaged");
    assert_eq!(packs.list(), Ok(vec![other.clone()]));
    // A kept file named for another pack than it holds.
    std::fs::write(packs.path("renamed"), &more).expect("renamed");
    assert_eq!(packs.list(), Ok(vec![other]));
    assert!(packs.picture(&named("renamed", "fox")).is_err());
}
