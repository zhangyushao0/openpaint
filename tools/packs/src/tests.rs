//! The builder on a cache of made-up answers and pictures (no network): the same list and cache
//! give the same pack byte for byte, every licence is checked against the API's own field, and
//! the official lists read with a reason per item.

use std::fs;
use std::path::Path;

use openpaint_app::ImageFormat;
use openpaint_practice::{Licence, Matte, Tier};
use openpaint_store::{encode_image, encode_pack, open_pack};

use super::build::{Outlined, attribution, build};
use super::commons::{plain, stated};
use super::fetch::{Fetcher, cache_name};
use super::list::SourceList;
use super::sources::{Found, Full, Stated};
use crate::error::PackError;
use crate::repo_root;
use crate::scratch::Scratch;

const LIST: &str = r#"
id = "test-shapes"
title = "Test shapes"
kind = "shape"

[[item]]
id = "teapot"
tier = "simple"
source = "cleveland"
object = "1916.1"
licence = "CC0"
reason = "one handle, one spout"

[[item]]
id = "helmet"
tier = "hard"
source = "met"
object = "4242"
licence = "CC0"
reason = "a visor's hinges"
no_outline = "a test of the list's mark"

[[item]]
id = "fox"
tier = "entry"
source = "commons"
object = "File:Test fox.png"
licence = "CC BY 4.0"
reason = "a bold closed outline"
"#;

fn cleveland(status: &str) -> String {
    format!(
        r#"{{"data": {{"share_license_status": "{status}", "title": "Teapot", "creators": [],
        "culture": ["America"], "url": "https://clevelandart.org/art/1916.1",
        "images": {{"print": {{"url": "https://cdn.example.org/1916.1_print.jpg"}}}}}}}}"#
    )
}

fn met(public: bool) -> String {
    format!(
        r#"{{"isPublicDomain": {public}, "title": "Close Helmet", "artistDisplayName": "",
        "culture": "Italian", "objectURL": "https://www.metmuseum.org/art/collection/search/4242",
        "primaryImage": "https://images.example.org/4242.jpg"}}"#
    )
}

fn commons(short: &str, code: &str) -> String {
    format!(
        r#"{{"query": {{"pages": [{{"title": "File:Test fox.png", "templates": [],
        "imageinfo": [{{"mime": "image/png", "url": "https://upload.example.org/fox.png",
        "descriptionurl": "https://commons.wikimedia.org/wiki/File:Test_fox.png",
        "extmetadata": {{"LicenseShortName": {{"value": "{short}"}},
        "License": {{"value": "{code}"}}, "ObjectName": {{"value": "Test fox"}},
        "Artist": {{"value": "<bdi><a href=\"https://example.org\">A. Painter</a></bdi>"}}}}}}]}}]}}}}"#
    )
}

/// A picture of `size` as a file of `format`, a gradient; transparent at the left when `clear`.
fn picture(format: ImageFormat, size: [u32; 2], clear: bool) -> Vec<u8> {
    let pixels: Vec<u8> = (0..size[1])
        .flat_map(|y| (0..size[0]).map(move |x| (x, y)))
        .flat_map(|(x, y)| {
            let alpha = if clear && x < size[0] / 4 { 0 } else { 255 };
            [(x % 256) as u8, (y % 256) as u8, 120, alpha]
        })
        .collect();
    encode_image(format, size, &pixels).expect("a picture")
}

/// A cache holding every answer and picture `LIST` needs, with the answers given.
fn cache(name: &str, answers: [String; 3]) -> Scratch {
    let dir = Scratch::new(name).expect("a directory");
    let put = |file: String, bytes: &[u8]| {
        let path = dir.join(file);
        fs::create_dir_all(path.parent().expect("a directory")).expect("the cache");
        fs::write(path, bytes).expect("a cached file");
    };
    let [first, second, third] = answers;
    put(cache_name("cleveland", "1916.1", "json"), first.as_bytes());
    put(cache_name("met", "4242", "json"), second.as_bytes());
    put(
        cache_name("commons", "File:Test fox.png", "json"),
        third.as_bytes(),
    );
    let print = picture(ImageFormat::Jpeg, [2600, 1300], false);
    put(cache_name("cleveland", "1916.1", "img"), &print);
    let helmet = picture(ImageFormat::Png, [300, 200], false);
    put(cache_name("met", "4242", "img"), &helmet);
    let fox = picture(ImageFormat::Png, [900, 2500], true);
    put(cache_name("commons", "File:Test fox.png", "img"), &fox);
    dir
}

/// The tests' segmentation: an ellipse filling the middle of any picture, its edge soft.
fn ellipse_matte(_: [u32; 2], _: &[u8]) -> Result<Matte, PackError> {
    let side = 64u32;
    let values = (0..side * side)
        .map(|i| {
            let [x, y] = [i % side, i / side].map(|v| (f64::from(v) + 0.5) / f64::from(side));
            let reach = ((x - 0.5) / 0.35).hypot((y - 0.5) / 0.3);
            ((1.2 - reach) * 2.5).clamp(0.0, 1.0) as f32
        })
        .collect();
    Ok(Matte {
        size: [side, side],
        values,
    })
}

fn answers() -> [String; 3] {
    [
        cleveland("CC0"),
        met(true),
        commons("CC BY 4.0", "cc-by-4.0"),
    ]
}

#[test]
fn a_rebuild_from_the_same_list_and_cache_gives_the_same_bytes() {
    let list = SourceList::parse(LIST).expect("the list");
    let dir = cache("pack-cache", answers());
    let fetcher = Fetcher::cache_only(&dir);
    let built = build(&list, &fetcher, &ellipse_matte).expect("built");
    let bytes = encode_pack(&built.pack, &built.pictures).expect("encoded");
    let again = build(&list, &fetcher, &ellipse_matte).expect("built again");
    assert!(encode_pack(&again.pack, &again.pictures).expect("encoded") == bytes);
    assert!(
        matches!(&built.outlines[1], Outlined::Listed { why, .. } if why.contains("mark")),
        "the list's mark kept"
    );
    assert!(built.pack.items[1].outline.is_none());
    assert_eq!(
        built.outlines[0],
        Outlined::Kept,
        "the matte's ellipse outlined"
    );
    assert!(built.pack.items[0].outline.is_some());

    let (opened, pictures) = open_pack(&bytes).expect("a whole pack");
    let sizes: Vec<[u32; 2]> = opened.items.iter().map(|item| item.size).collect();
    assert_eq!(sizes, [[2048, 1024], [300, 200], [737, 2048]]);
    let kinds: Vec<&[u8]> = pictures.iter().map(|p| &p[..2]).collect();
    assert_eq!(
        kinds,
        [b"\xff\xd8", b"\xff\xd8", b"\x89P"],
        "JPEG unless transparent"
    );
    let [teapot, helmet, fox] = &opened.items[..] else {
        panic!("three items");
    };
    let credit = |item: &openpaint_practice::PackItem| item.credit.clone().expect("credited");
    let (teapot_credit, fox_credit) = (credit(teapot), credit(fox));
    assert_eq!(
        (teapot.tier, teapot_credit.licence),
        (Tier::Simple, Licence::Cc0)
    );
    assert_eq!(teapot_credit.creator, "Unknown maker, America");
    assert_eq!(teapot_credit.notes, "one handle, one spout");
    assert_eq!(
        credit(helmet).source,
        "https://www.metmuseum.org/art/collection/search/4242"
    );
    assert_eq!(
        (fox_credit.creator.as_str(), fox_credit.licence),
        ("A. Painter", Licence::CcBy4)
    );
    assert_eq!(
        fox_credit.attribution,
        "“Test fox” by A. Painter, Wikimedia Commons \
         (https://commons.wikimedia.org/wiki/File:Test_fox.png), licensed CC BY 4.0 \
         (https://creativecommons.org/licenses/by/4.0/); resized."
    );
}

/// The item the build of `LIST` fails on with `answers`, and why.
fn failure(name: &str, answers: [String; 3]) -> (String, String) {
    let list = SourceList::parse(LIST).expect("the list");
    let dir = cache(name, answers);
    let failed = build(&list, &Fetcher::cache_only(&dir), &ellipse_matte);
    match failed {
        Err(PackError::Pack { item, problem }) => (item, problem),
        other => panic!("not a failed item: {:?}", other.map(|built| built.pack.id)),
    }
}

#[test]
fn a_licence_that_disagrees_fails_the_build_and_names_the_item() {
    let [teapot, helmet, _] = answers();
    let share_alike = commons("CC BY-SA 4.0", "cc-by-sa-4.0");
    let (item, problem) = failure("pack-sa", [teapot.clone(), helmet.clone(), share_alike]);
    assert_eq!(item, "fox");
    assert!(problem.contains("CC BY-SA 4.0") && problem.contains("expects CC BY 4.0"));
    let (item, problem) = failure("pack-met", [teapot, met(false), answers()[2].clone()]);
    assert_eq!(item, "helmet");
    assert!(problem.contains("not public domain"), "{problem}");
    let (item, _) = failure(
        "pack-cma",
        [cleveland("Copyrighted"), helmet, answers()[2].clone()],
    );
    assert_eq!(item, "teapot");
}

#[test]
fn nothing_missing_from_the_cache_is_taken_without_the_network() {
    let list = SourceList::parse(LIST).expect("the list");
    let dir = Scratch::new("pack-empty").expect("a directory");
    match build(&list, &Fetcher::cache_only(&dir), &ellipse_matte) {
        Err(PackError::Pack { item, problem }) => {
            assert_eq!(item, "teapot");
            assert!(problem.contains("not in the cache"), "{problem}");
        }
        other => panic!("not a failed item: {:?}", other.map(|built| built.pack.id)),
    }
}

#[test]
fn commons_states_a_licence_only_by_its_own_fields() {
    let licence = |code, templates: &[&str]| stated(code, "short", templates);
    assert_eq!(licence("cc0", &[]), Stated::Licence(Licence::Cc0));
    assert_eq!(licence("cc-by-3.0", &[]), Stated::Licence(Licence::CcBy3));
    let nasa = ["Template:PD-USGov-NASA", "Template:Information"];
    assert_eq!(licence("pd", &nasa), Stated::Licence(Licence::PdUsGov));
    let old = ["Template:PD-Art", "Template:PD-old-100"];
    assert_eq!(licence("pd", &old), Stated::Licence(Licence::PdOld));
    assert!(matches!(
        licence("pd", &["Template:PD-self"]),
        Stated::Other(_)
    ));
    assert!(matches!(licence("cc-by-sa-4.0", &[]), Stated::Other(_)));
    assert!(
        matches!(licence("", &nasa), Stated::Other(_)),
        "no code, no licence"
    );
    assert_eq!(
        plain("<bdi><a href=\"x\">Ren&amp;Stimpy</a>\n  studio</bdi>"),
        "Ren&Stimpy studio"
    );
}

#[test]
fn an_attribution_names_the_creator_and_the_credit_asked_for() {
    let found = Found {
        title: "Twemoji12 1f47b".to_owned(),
        creator: "Twitter".to_owned(),
        page: "https://commons.wikimedia.org/wiki/File:Twemoji12_1f47b.svg".to_owned(),
        stated: Stated::Licence(Licence::CcBy4),
        picture: String::new(),
        credit: "Copyright 2019 Twitter, Inc and other contributors".to_owned(),
        collection: "Wikimedia Commons",
        changes: "resized",
        headers: &[],
        full: Full::Picture,
    };
    assert_eq!(
        attribution(&found, Licence::CcBy4),
        "“Twemoji12 1f47b” by Twitter (Copyright 2019 Twitter, Inc and other contributors), \
         Wikimedia Commons (https://commons.wikimedia.org/wiki/File:Twemoji12_1f47b.svg), \
         licensed CC BY 4.0 (https://creativecommons.org/licenses/by/4.0/); resized."
    );
    let museum = Found {
        title: "Teapot".to_owned(),
        creator: "Nathaniel Hurd".to_owned(),
        page: "https://clevelandart.org/art/1940.228".to_owned(),
        credit: "Nathaniel Hurd".to_owned(),
        collection: "The Cleveland Museum of Art",
        ..found
    };
    assert_eq!(
        attribution(&museum, Licence::Cc0),
        "“Teapot”, Nathaniel Hurd. The Cleveland Museum of Art \
         (https://clevelandart.org/art/1940.228), CC0 \
         (https://creativecommons.org/publicdomain/zero/1.0/)."
    );
    assert!(attribution(&museum, Licence::PdUsGov).ends_with("US federal government."));
}

/// The official lists in `tools/packs/lists/`: each reads, every item with its reason.
#[test]
fn the_official_shapes_list_holds_30_to_40_items_a_tier() {
    let path = repo_root().join("tools/packs/lists/shapes.toml");
    let list = SourceList::parse(&fs::read_to_string(&path).expect("the list")).expect("reads");
    for tier in Tier::ALL {
        let count = list
            .items
            .iter()
            .filter(|item| item.tier == Some(tier))
            .count();
        assert!((30..=40).contains(&count), "{}: {count} items", tier.name());
    }
    assert!(list.items.iter().all(|item| item.reason.len() >= 12));
    assert!(Path::new(&path).is_file());
}
