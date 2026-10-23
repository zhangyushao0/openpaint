//! The builder on a cache of made-up answers and pictures (no network): the same list and cache
//! give the same pack byte for byte, every licence is checked against the API's own field, and
//! the official lists read with a reason per item.

use std::fs;
use std::path::Path;

use openpaint_app::ImageFormat;
use openpaint_practice::{Licence, Tier};
use openpaint_store::{encode_image, encode_pack, open_pack};

use super::build::{attribution, build};
use super::commons::{plain, stated};
use super::fetch::{Fetcher, cache_name};
use super::list::SourceList;
use super::sources::{Found, Stated};
use crate::error::HarnessError;
use crate::output::workspace_root;
use crate::test_art::scratch;

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
fn cache(name: &str, answers: [String; 3]) -> std::path::PathBuf {
    let dir = scratch(name);
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
    let (pack, pictures) = build(&list, &fetcher).expect("built");
    let bytes = encode_pack(&pack, &pictures).expect("encoded");
    let (again, again_pictures) = build(&list, &fetcher).expect("built again");
    assert!(encode_pack(&again, &again_pictures).expect("encoded") == bytes);

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
    assert_eq!((teapot.tier, teapot.licence), (Tier::Simple, Licence::Cc0));
    assert_eq!(teapot.creator, "Unknown maker, America");
    assert_eq!(teapot.notes, "one handle, one spout");
    assert_eq!(
        helmet.source,
        "https://www.metmuseum.org/art/collection/search/4242"
    );
    assert_eq!(
        (fox.creator.as_str(), fox.licence),
        ("A. Painter", Licence::CcBy4)
    );
    assert_eq!(
        fox.attribution,
        "“Test fox” by A. Painter, Wikimedia Commons \
         (https://commons.wikimedia.org/wiki/File:Test_fox.png), licensed CC BY 4.0 \
         (https://creativecommons.org/licenses/by/4.0/); resized."
    );
    fs::remove_dir_all(&dir).expect("removed");
}

/// The item the build of `LIST` fails on with `answers`, and why.
fn failure(name: &str, answers: [String; 3]) -> (String, String) {
    let list = SourceList::parse(LIST).expect("the list");
    let dir = cache(name, answers);
    let failed = build(&list, &Fetcher::cache_only(&dir));
    fs::remove_dir_all(&dir).expect("removed");
    match failed {
        Err(HarnessError::Pack { item, problem }) => (item, problem),
        other => panic!("not a failed item: {other:?}"),
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
    let dir = scratch("pack-empty");
    match build(&list, &Fetcher::cache_only(&dir)) {
        Err(HarnessError::Pack { item, problem }) => {
            assert_eq!(item, "teapot");
            assert!(problem.contains("not in the cache"), "{problem}");
        }
        other => panic!("not a failed item: {other:?}"),
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

/// The official lists in `apps/harness/packs/`: each reads, every item with its reason.
#[test]
fn the_official_shapes_list_holds_30_to_40_items_a_tier() {
    let path = workspace_root().join("apps/harness/packs/shapes.toml");
    let list = SourceList::parse(&fs::read_to_string(&path).expect("the list")).expect("reads");
    for tier in Tier::ALL {
        let count = list.items.iter().filter(|item| item.tier == tier).count();
        assert!((30..=40).contains(&count), "{}: {count} items", tier.name());
    }
    assert!(list.items.iter().all(|item| item.reason.len() >= 12));
    assert!(Path::new(&path).is_file());
}
