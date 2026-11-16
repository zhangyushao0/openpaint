//! Openverse (S159) on a cache of made-up answers (no network): an image gives its crops with the
//! licence its answer's own fields state, credited to its creator at its provider's page; a
//! licence the list does not expect, or one no pack takes, fails the build naming the item.

use std::fs;

use openpaint_practice::Licence;
use openpaint_practice_store::{encode_pack, open_pack};

use super::build::build;
use super::fetch::{Fetcher, cache_name};
use super::list::SourceList;
use super::tests_crops::{no_segment, rust};
use crate::error::PackError;
use crate::scratch::Scratch;

const ID: &str = "8396a3b2-002e-41cc-843a-f04af34bc89c";

const LIST: &str = r#"
id = "test-openverse"
title = "Test Openverse"
kind = "crop"

[[item]]
id = "wp-wall"
source = "openverse"
object = "8396a3b2-002e-41cc-843a-f04af34bc89c"
licence = "CC BY-SA 2.0"
reason = "peeling paint over rust in four patches"
"#;

/// Openverse's answer about `ID`, its licence `licence` of `version`.
fn answer(licence: &str, version: &str) -> String {
    format!(
        r#"{{"id": "{ID}", "title": "A weathered wall with peeling paint. Blue and rust.",
        "foreign_landing_url": "https://wordpress.org/photos/photo/794677ae17/",
        "url": "https://pd.w.org/2025/01/794677ae173eebd84.34072220-2048x1365.jpg",
        "creator": "A. Photographer", "license": "{licence}", "license_version": "{version}",
        "provider": "wordpress", "width": 2048, "height": 1365}}"#
    )
}

fn cache(name: &str, answer: &str) -> Scratch {
    let dir = Scratch::new(name).expect("a directory");
    let put = |file: String, bytes: &[u8]| {
        let path = dir.join(file);
        fs::create_dir_all(path.parent().expect("a directory")).expect("the cache");
        fs::write(path, bytes).expect("a cached file");
    };
    put(cache_name("openverse", ID, "json"), answer.as_bytes());
    put(cache_name("openverse", ID, "img"), &rust());
    dir
}

#[test]
fn an_openverse_image_gives_crops_under_the_licence_its_answer_states() {
    let list = SourceList::parse(LIST).expect("the list");
    let dir = cache("openverse", &answer("by-sa", "2.0"));
    let built = build(&list, &Fetcher::cache_only(&dir), &no_segment).expect("built");
    let (pack, _) =
        open_pack(&encode_pack(&built.pack, &built.pictures).expect("encoded")).expect("a pack");
    let crop = &pack.items[0];
    assert_eq!(crop.id, "wp-wall-1");
    assert!(
        crop.title
            .starts_with("A weathered wall with peeling paint (detail 1)")
    );
    let credit = crop.credit.as_ref().expect("credited");
    assert_eq!(
        (
            credit.licence,
            credit.creator.as_str(),
            credit.source.as_str()
        ),
        (
            Licence::CcBySa2,
            "A. Photographer",
            "https://wordpress.org/photos/photo/794677ae17/"
        )
    );
    let shown = crop.shown_credit().expect("an attribution");
    assert!(
        shown.contains("WordPress Photo Directory") && shown.ends_with("under the same licence."),
        "{shown}"
    );
}

#[test]
fn a_licence_not_the_list_s_or_none_a_pack_takes_fails_naming_the_item() {
    let list = SourceList::parse(LIST).expect("the list");
    for (licence, version, said) in [
        (
            "cc0",
            "1.0",
            "states the licence CC0, the list expects CC BY-SA 2.0",
        ),
        ("by-nc-sa", "2.0", "states the licence by-nc-sa 2.0"),
    ] {
        let dir = cache(&format!("openverse-{licence}"), &answer(licence, version));
        match build(&list, &Fetcher::cache_only(&dir), &no_segment) {
            Err(PackError::Pack { item, problem }) => {
                assert_eq!(item, "wp-wall");
                assert!(problem.contains(said), "{problem}");
            }
            other => panic!("not a failed item: {:?}", other.map(|b| b.pack.id)),
        }
    }
}
