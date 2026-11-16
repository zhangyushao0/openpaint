//! The texture libraries (S101c, S159) on a cache of made-up answers (no network): an ambientCG
//! material, a Poly Haven texture and a Texture Ninja photograph each give their crops, credited
//! to the library as CC0, the material's colour map read from its archive; a library's picture
//! never goes into a list of another kind; the official combinations list reads.

use std::fs;

use openpaint_practice::{Licence, PictureKind};
use openpaint_practice_store::{encode_pack, open_pack};

use super::build::build;
use super::fetch::{Fetcher, cache_name};
use super::list::SourceList;
use super::tests_crops::{no_segment, rust};
use crate::error::PackError;
use crate::repo_root;
use crate::scratch::Scratch;

const LIST: &str = r#"
id = "test-combinations"
title = "Test combinations"
kind = "crop"

[[item]]
id = "acg-rust"
source = "ambientcg"
object = "Rust900"
licence = "CC0"
reason = "rust and paint in four patches"

[[item]]
id = "ph-rust"
source = "polyhaven"
object = "rusty_test"
licence = "CC0"
reason = "rust and paint in four patches"

[[item]]
id = "tn-rust"
source = "textureninja"
object = "Rust_Test-0001.jpg"
licence = "CC0"
reason = "rust and paint in four patches"
"#;

const TEXTURE_NINJA: &str = r#"[{"id": 9001, "key": "Rust_Test-0001.jpg", "width": 450,
    "height": 300, "children": null, "tags": [{"tag": "rust"}], "categories": [37]}]"#;

const AMBIENTCG: &str = r#"{"foundAssets": [{"assetId": "Rust900", "displayName": "Rust 900",
    "shortLink": "https://ambientcg.com/a/Rust900", "downloadFolders": {"default":
    {"downloadFiletypeCategories": {"zip": {"downloads": [
    {"attribute": "1K-JPG", "fullDownloadPath": "https://ambientcg.com/get?file=Rust900_1K-JPG.zip"},
    {"attribute": "4K-JPG", "fullDownloadPath": "https://ambientcg.com/get?file=Rust900_4K-JPG.zip"}
    ]}}}}}]}"#;

const POLYHAVEN_INFO: &str = r#"{"name": "Rusty Test", "authors": {"A. Painter": "All"}}"#;
const POLYHAVEN_FILES: &str = r#"{"Diffuse": {"4k": {"jpg": {"url":
    "https://dl.polyhaven.org/file/ph-assets/Textures/jpg/4k/rusty_test/rusty_test_diff_4k.jpg"}}},
    "nor_gl": {"4k": {"jpg": {"url": "https://dl.polyhaven.org/rusty_test_nor_gl_4k.jpg"}}}}"#;

/// A cache holding both libraries' answers, the ambientCG material's colour map as the build
/// keeps it out of the archive, and the Poly Haven colour map.
fn cache() -> Scratch {
    let dir = Scratch::new("pack-libraries").expect("a directory");
    let put = |file: String, bytes: &[u8]| {
        let path = dir.join(file);
        fs::create_dir_all(path.parent().expect("a directory")).expect("the cache");
        fs::write(path, bytes).expect("a cached file");
    };
    put(
        cache_name("ambientcg", "Rust900", "json"),
        AMBIENTCG.as_bytes(),
    );
    put(cache_name("ambientcg", "Rust900 full", "img"), &rust());
    put(
        cache_name("polyhaven", "rusty_test", "json"),
        POLYHAVEN_INFO.as_bytes(),
    );
    put(
        cache_name("polyhaven", "rusty_test files", "json"),
        POLYHAVEN_FILES.as_bytes(),
    );
    put(cache_name("polyhaven", "rusty_test", "img"), &rust());
    let key = "Rust_Test-0001.jpg";
    put(
        cache_name("textureninja", key, "json"),
        TEXTURE_NINJA.as_bytes(),
    );
    put(cache_name("textureninja", key, "img"), &rust());
    dir
}

#[test]
fn every_library_gives_crops_of_its_pictures_credited_as_cc0() {
    let list = SourceList::parse(LIST).expect("the list");
    let dir = cache();
    let built = build(&list, &Fetcher::cache_only(&dir), &no_segment).expect("built");
    let (pack, _) =
        open_pack(&encode_pack(&built.pack, &built.pictures).expect("encoded")).expect("a pack");
    let credits: Vec<String> = (pack.items.iter())
        .map(|item| {
            let credit = item.credit.as_ref().expect("credited");
            format!(
                "{} | {} | {}",
                item.id,
                credit.creator,
                credit.licence.name()
            )
        })
        .collect();
    let expected: Vec<String> = [
        ("acg-rust", "ambientCG"),
        ("ph-rust", "A. Painter"),
        ("tn-rust", "Texture Ninja"),
    ]
    .into_iter()
    .flat_map(|(id, creator)| (1..=3).map(move |n| format!("{id}-{n} | {creator} | CC0")))
    .collect();
    assert_eq!(credits, expected);
    let first = pack.items[0].credit.as_ref().expect("credited");
    assert_eq!(first.source, "https://ambientcg.com/a/Rust900");
    assert!(pack.items[3].title.starts_with("Rusty Test (detail 1)"));
    let ninja = &pack.items[6];
    assert!(ninja.title.starts_with("Rust Test 0001 (detail 1)"));
    let credit = ninja.credit.as_ref().expect("credited");
    assert_eq!(
        credit.source,
        "https://texture.ninja/search?texture=Rust_Test-0001.jpg"
    );
    assert_eq!(credit.licence, Licence::Cc0);
    assert!(pack.items.iter().all(|item| item.kind == PictureKind::Crop));
}

#[test]
fn a_library_s_map_is_only_ever_cropped() {
    let text = LIST
        .replace("kind = \"crop\"", "kind = \"composition\"")
        .replace("licence = \"CC0\"", "tier = \"simple\"\nlicence = \"CC0\"");
    match SourceList::parse(&text) {
        Err(PackError::List(why)) => assert!(why.contains("only ever cropped"), "{why}"),
        other => panic!("read: {:?}", other.map(|list| list.id)),
    }
}

/// The official list in `tools/packs/lists/combinations.toml`: a list of crops, every item with
/// its reason, its photographs (Commons', Openverse's), texture libraries and paintings all there.
#[test]
fn the_official_combinations_list_reads() {
    let path = repo_root().join("tools/packs/lists/combinations.toml");
    let list = SourceList::parse(&fs::read_to_string(&path).expect("the list")).expect("reads");
    assert_eq!(
        (list.id.as_str(), list.kind),
        ("openpaint-combinations", PictureKind::Crop)
    );
    assert!(list.items.iter().all(|item| item.reason.len() >= 12));
    for source in [
        "commons",
        "ambientcg",
        "polyhaven",
        "textureninja",
        "openverse",
        "aic",
        "nga",
    ] {
        assert!(
            list.items.iter().any(|item| item.source.name() == source),
            "{source}"
        );
    }
}
