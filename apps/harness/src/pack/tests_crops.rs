//! Crops (S101) on a cache of made-up answers (no network): a list of crops gives its picture's
//! best two crops, each a crop item with its areas, the same bytes every build; one the list
//! leaves out is cut and drawn but never packed; every crop 768 to 1,024 pixels long (S101b); the
//! list's marks are checked.

use std::fs;

use openpaint_app::ImageFormat;
use openpaint_practice::{LEAST_LONGER, Licence, PictureKind};
use openpaint_store::{encode_image, encode_pack, open_pack};

use super::build::build;
use super::fetch::{Fetcher, cache_name};
use super::list::SourceList;
use crate::error::HarnessError;
use crate::scratch::Scratch;

const LIST: &str = r#"
id = "test-textures"
title = "Test textures"
kind = "crop"

[[item]]
id = "rust"
source = "commons"
object = "File:Test rust.png"
licence = "CC0"
reason = "rust and paint, a weak contrast between the browns"
no_crop.2 = "a test of the list's mark"
"#;

const ANSWER: &str = r#"{"query": {"pages": [{"title": "File:Test rust.png", "templates": [],
    "imageinfo": [{"mime": "image/png", "url": "https://upload.example.org/rust.png",
    "descriptionurl": "https://commons.wikimedia.org/wiki/File:Test_rust.png",
    "extmetadata": {"LicenseShortName": {"value": "CC0"}, "License": {"value": "cc0"},
    "ObjectName": {"value": "Test rust"}, "Artist": {"value": "A. Welder"}}}]}]}}"#;

/// Rust: four patches meeting at a point left of the middle and four right of it, two browns a
/// weak contrast apart, a grey and a blue grey; large enough for crops of `LEAST_LONGER` (S101b).
pub(super) fn rust() -> Vec<u8> {
    let size = [2800, 1400];
    let pixels: Vec<u8> = (0..size[0] * size[1])
        .flat_map(|at| {
            let [x, y] = [at % size[0], at / size[0]];
            let centre = if x < size[0] / 2 { 385 } else { 2415 };
            let colour: [u8; 3] = match (x < centre, y < size[1] / 2) {
                (true, true) => [120, 80, 60],
                (false, true) => [128, 86, 64],
                (true, false) => [150, 150, 145],
                (false, false) => [80, 95, 115],
            };
            [colour[0], colour[1], colour[2], 255]
        })
        .collect();
    encode_image(ImageFormat::Png, size, &pixels).expect("a picture")
}

fn cache() -> Scratch {
    let dir = Scratch::new("pack-crops").expect("a directory");
    let put = |file: String, bytes: &[u8]| {
        let path = dir.join(file);
        fs::create_dir_all(path.parent().expect("a directory")).expect("the cache");
        fs::write(path, bytes).expect("a cached file");
    };
    put(
        cache_name("commons", "File:Test rust.png", "json"),
        ANSWER.as_bytes(),
    );
    put(cache_name("commons", "File:Test rust.png", "img"), &rust());
    dir
}

pub(super) fn no_segment(_: [u32; 2], _: &[u8]) -> Result<openpaint_practice::Matte, HarnessError> {
    Err(HarnessError::Missing("no outline is derived for a crop"))
}

#[test]
fn a_list_of_crops_packs_its_best_crops_with_their_areas_the_same_every_build() {
    let list = SourceList::parse(LIST).expect("the list");
    let dir = cache();
    let fetcher = Fetcher::cache_only(&dir);
    let built = build(&list, &fetcher, &no_segment).expect("built");
    let bytes = encode_pack(&built.pack, &built.pictures).expect("encoded");
    let again = build(&list, &fetcher, &no_segment).expect("built again");
    assert!(encode_pack(&again.pack, &again.pictures).expect("encoded") == bytes);
    assert_eq!(built.crops.len(), 2, "the picture's best two");
    assert_eq!(built.crops[0].left_out, None);
    let why = built.crops[1].left_out.as_deref();
    assert_eq!(
        why,
        Some("a test of the list's mark"),
        "cut and drawn, never packed"
    );
    let (pack, _) = open_pack(&bytes).expect("a whole pack");
    let ids: Vec<&str> = pack.items.iter().map(|item| item.id.as_str()).collect();
    assert_eq!(
        ids,
        ["rust-1"],
        "only the crop kept; the photograph itself never"
    );
    let crop = &pack.items[0];
    assert_eq!(crop.kind, PictureKind::Crop);
    let areas = crop.areas.as_ref().expect("its areas");
    assert_eq!((areas.count(), areas.weak_pairs().len()), (4, 1));
    let credit = crop.credit.as_ref().expect("credited");
    assert_eq!(
        (credit.licence, credit.creator.as_str()),
        (Licence::Cc0, "A. Welder")
    );
    assert!(crop.title.starts_with("Test rust (detail 1)"));
}

#[test]
fn every_crop_is_768_to_1024_pixels_long() {
    let list = SourceList::parse(LIST).expect("the list");
    let dir = cache();
    let built = build(&list, &Fetcher::cache_only(&dir), &no_segment).expect("built");
    assert!(!built.crops.is_empty());
    for crop in &built.crops {
        let longer = crop.size[0].max(crop.size[1]);
        assert!(
            (LEAST_LONGER..=1024).contains(&longer),
            "{}: {longer}",
            crop.id
        );
    }
    let (pack, _) = open_pack(&encode_pack(&built.pack, &built.pictures).expect("encoded"))
        .expect("a whole pack");
    for item in &pack.items {
        let longer = item.size[0].max(item.size[1]);
        assert!(
            (LEAST_LONGER..=1024).contains(&longer),
            "{}: {longer}",
            item.id
        );
    }
}

#[test]
fn a_crop_list_s_marks_are_checked() {
    let with = |line: &str| SourceList::parse(&LIST.replace("no_crop.2", line));
    assert!(matches!(
        with("tier = \"simple\"\nno_crop.2"),
        Err(HarnessError::List(_))
    ));
    assert!(
        matches!(with("no_crop.3"), Err(HarnessError::List(_))),
        "two crops a photograph"
    );
    let blank = LIST.replace("\"a test of the list's mark\"", "\" \"");
    assert!(
        SourceList::parse(&blank).is_err(),
        "a crop left out says why"
    );
    let shapes = LIST.replace("kind = \"crop\"", "kind = \"shape\"\n");
    assert!(
        SourceList::parse(&shapes).is_err(),
        "a shape is never cropped"
    );
}
