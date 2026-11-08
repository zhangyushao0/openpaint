//! The composition pack's sources on a cache of made-up answers (no network): the Art Institute,
//! the National Gallery's tables, the Rijksmuseum's Linked Art and a Commons video's still each
//! give their item with the licence read from their own field, a wrong one fails naming the item,
//! no outline is ever derived for a composition, and the official list reads.

use std::fs;

use openpaint_app::ImageFormat;
use openpaint_practice::{PictureKind, Tier};
use openpaint_practice_store::{encode_pack, open_pack};
use openpaint_store::encode_image;

use super::build::{Massed, Outlined, build};
use super::fetch::{Fetcher, cache_name};
use super::list::SourceList;
use super::nga::table_name;
use crate::error::PackError;
use crate::repo_root;
use crate::scratch::Scratch;

const LIST: &str = r#"
id = "test-compositions"
title = "Test compositions"
kind = "composition"

[[item]]
id = "stacks"
tier = "simple"
source = "aic"
object = "64818"
licence = "CC0"
reason = "two big stacks against a band of field and sky"

[[item]]
id = "mill"
tier = "simple"
source = "nga"
object = "1201"
licence = "CC0"
reason = "one dark mill against a light sky"

[[item]]
id = "skaters"
tier = "hard"
source = "rijks"
object = "SK-A-1718"
licence = "PD-old"
reason = "hundreds of figures on the ice"

[[item]]
id = "forest"
tier = "medium"
source = "commons"
object = "File:Spring - Blender Open Movie.webm"
at = 120
licence = "CC BY 4.0"
reason = "a stone lit in the fog at the end of a path"
"#;

const FILM: &str = "File:Spring - Blender Open Movie.webm";
const SKATERS: &str = "https://id.rijksmuseum.nl/200108163";
const SWAN: &str = "https://id.rijksmuseum.nl/200108146";
const AVERCAMP: &str = "https://id.rijksmuseum.nl/2101568";

fn aic(public: bool) -> String {
    format!(
        r#"{{"data": {{"id": 64818, "title": "Stacks of Wheat (End of Summer)",
        "artist_title": "Claude Monet", "is_public_domain": {public},
        "image_id": "abc-123"}}, "config": {{"iiif_url": "https://www.artic.edu/iiif/2"}}}}"#
    )
}

fn nga_pictures(open: &str) -> String {
    format!(
        "uuid,iiifurl,viewtype,sequence,openaccess,depictstmsobjectid\n\
         a1,https://api.nga.gov/iiif/a1,alternate,0,1,1201\n\
         b2,https://api.nga.gov/iiif/b2,primary,1,1,1201\n\
         c3,https://api.nga.gov/iiif/c3,primary,0,{open},1201\n"
    )
}

/// A Rijksmuseum record numbered `number`, its picture under `rights` and made by Avercamp.
fn rijks(number: &str, rights: &str) -> String {
    format!(
        r#"{{"identified_by": [
          {{"type": "Identifier", "content": "{number}"}},
          {{"type": "Name", "content": "Winterlandschap", "language":
            [{{"id": "http://vocab.getty.edu/aat/300388256"}}]}},
          {{"type": "Name", "content": "Winter Landscape with Ice Skaters", "language":
            [{{"id": "http://vocab.getty.edu/aat/300388277"}}]}}],
        "produced_by": {{"part": [{{"assigned_by": [{{"assigned":
          [{{"id": "{AVERCAMP}", "type": "Person"}}]}}]}}]}},
        "subject_of": [{{"digitally_carried_by": [{{"access_point":
          [{{"id": "https://iiif.micr.io/aXnzA/full/max/0/default.jpg"}}],
          "referred_to_by": [{{"content": "{rights}"}}]}}]}}]}}"#
    )
}

fn commons_film(short: &str, code: &str) -> String {
    format!(
        r#"{{"query": {{"pages": [{{"title": "{FILM}", "templates": [],
        "imageinfo": [{{"mime": "video/webm", "width": 1920,
        "url": "https://upload.example.org/spring.webm",
        "thumburl": "https://upload.example.org/spring-seek120.jpg",
        "descriptionurl": "https://commons.wikimedia.org/wiki/File:Spring_-_Blender_Open_Movie.webm",
        "extmetadata": {{"LicenseShortName": {{"value": "{short}"}},
        "License": {{"value": "{code}"}}, "ObjectName": {{"value": "Spring - Blender Open Movie"}},
        "Artist": {{"value": "Andy Goralczyk / Blender Foundation"}},
        "Attribution": {{"value": "© Blender Foundation | cloud.blender.org/spring"}}}}}}]}}]}}}}"#
    )
}

fn answers() -> [String; 4] {
    [
        aic(true),
        nga_pictures("1"),
        rijks(
            "SK-A-1718",
            "https://creativecommons.org/publicdomain/mark/1.0/",
        ),
        commons_film("CC BY 4.0", "cc-by-4.0"),
    ]
}

/// A cache holding every answer and picture `LIST` needs, with the answers given.
fn cache(name: &str, [aic, nga, rijks_object, film]: [String; 4]) -> Scratch {
    let dir = Scratch::new(name).expect("a directory");
    let put = |file: String, bytes: &[u8]| {
        let path = dir.join(file);
        fs::create_dir_all(path.parent().expect("a directory")).expect("the cache");
        fs::write(path, bytes).expect("a cached file");
    };
    let picture = |size: [u32; 2]| {
        let pixels: Vec<u8> = (0..size[0] * size[1])
            .flat_map(|i| [(i % 251) as u8, (i % 241) as u8, 90, 255])
            .collect();
        encode_image(ImageFormat::Jpeg, size, &pixels).expect("a picture")
    };
    put(cache_name("aic", "64818", "json"), aic.as_bytes());
    put(cache_name("aic", "64818", "img"), &picture([300, 180]));
    let objects = "objectid,title,attribution\n1201,\"The Mill, at evening\",Rembrandt van Rijn\n";
    put(table_name("objects"), objects.as_bytes());
    put(table_name("published_images"), nga.as_bytes());
    put(cache_name("nga", "1201", "img"), &picture([240, 200]));
    let search = format!(r#"{{"orderedItems": [{{"id": "{SWAN}"}}, {{"id": "{SKATERS}"}}]}}"#);
    put(cache_name("rijks", "SK-A-1718", "json"), search.as_bytes());
    let swan = rijks(
        "SK-A-4",
        "https://creativecommons.org/publicdomain/mark/1.0/",
    );
    put(cache_name("rijks", SWAN, "json"), swan.as_bytes());
    put(
        cache_name("rijks", SKATERS, "json"),
        rijks_object.as_bytes(),
    );
    let person = r#"{"identified_by": [{"type": "Name", "content": "Hendrick Avercamp",
        "language": [{"id": "http://vocab.getty.edu/aat/300388277"}]}]}"#;
    put(cache_name("rijks", AVERCAMP, "json"), person.as_bytes());
    put(
        cache_name("rijks", "SK-A-1718", "img"),
        &picture([320, 190]),
    );
    let frame = format!("{FILM}@120");
    put(cache_name("commons", &frame, "json"), film.as_bytes());
    put(cache_name("commons", &frame, "img"), &picture([480, 201]));
    // Each picture's full pixels, for its corners (S101b): a IIIF service's size, the film's
    // still asked again at its own width.
    let info = |size: [u32; 2]| format!(r#"{{"width": {}, "height": {}}}"#, size[0], size[1]);
    for (source, key, answer, size) in [
        ("aic", "64818", info([600, 360]), [600, 360]),
        ("nga", "1201", info([480, 400]), [480, 400]),
        ("rijks", "SK-A-1718", info([640, 380]), [640, 380]),
        ("commons", frame.as_str(), film.clone(), [960, 402]),
    ] {
        let name = |extension| cache_name(source, &format!("{key} full"), extension);
        put(name("json"), answer.as_bytes());
        put(name("img"), &picture(size));
    }
    dir
}

/// Never called: a composition is not outlined.
fn no_segment(_: [u32; 2], _: &[u8]) -> Result<openpaint_practice::Matte, PackError> {
    panic!("a composition's outline step ran")
}

#[test]
fn a_composition_pack_takes_every_new_source_and_derives_no_outline() {
    let list = SourceList::parse(LIST).expect("the list");
    let dir = cache("compositions", answers());
    let fetcher = Fetcher::cache_only(&dir);
    let built = build(&list, &fetcher, &no_segment).expect("built");
    let bytes = encode_pack(&built.pack, &built.pictures).expect("encoded");
    let again = build(&list, &fetcher, &no_segment).expect("built again");
    assert!(encode_pack(&again.pack, &again.pictures).expect("encoded") == bytes);
    assert!(built.outlines.iter().all(|o| *o == Outlined::Never));
    // Every composition's value masses made and kept (S96).
    assert!(built.masses.iter().all(|m| *m == Massed::Kept));

    let (opened, _) = open_pack(&bytes).expect("a whole pack");
    assert!(
        (opened.items.iter()).all(|i| i.kind == PictureKind::Composition && i.outline.is_none())
    );
    assert!(opened.items.iter().all(|i| i.masses.is_some()));
    // One listed `no_masses`: made for the sheet, never in the pack.
    let marked = LIST.replacen(
        "licence = \"CC0\"",
        "licence = \"CC0\"\nno_masses = \"a test\"",
        1,
    );
    let marked = SourceList::parse(&marked).expect("the list");
    let built = build(&marked, &fetcher, &no_segment).expect("built");
    assert!(matches!(&built.masses[0], Massed::Listed { why, .. } if why == "a test"));
    assert!(built.pack.items[0].masses.is_none() && built.pack.items[1].masses.is_some());
    let credits: Vec<String> = (opened.items.iter())
        .map(|item| {
            let credit = item.credit.as_ref().expect("credited");
            let licence = credit.licence.name();
            format!(
                "{} | {} | {} | {licence}",
                item.title, credit.creator, credit.source
            )
        })
        .collect();
    assert_eq!(
        credits,
        [
            "Stacks of Wheat (End of Summer) | Claude Monet | \
             https://www.artic.edu/artworks/64818 | CC0",
            "The Mill, at evening | Rembrandt van Rijn | \
             https://www.nga.gov/collection/art-object-page.1201.html | CC0",
            format!("Winter Landscape with Ice Skaters | Hendrick Avercamp | {SKATERS} | PD-old")
                .as_str(),
            "Spring - Blender Open Movie, 2:00 | Andy Goralczyk / Blender Foundation | \
             https://commons.wikimedia.org/wiki/File:Spring_-_Blender_Open_Movie.webm | CC BY 4.0",
        ]
    );
    let shown = opened.items[3].shown_credit().expect("an attribution");
    assert!(
        shown.ends_with("; a still of the film, resized."),
        "{shown}"
    );
    assert!(
        shown.contains("(© Blender Foundation | cloud.blender.org/spring)"),
        "{shown}"
    );
    assert_eq!(opened.items[2].tier, Tier::Hard);
}

/// The item the build of `LIST` fails on with `answers`, and why.
fn failure(name: &str, answers: [String; 4]) -> (String, String) {
    let list = SourceList::parse(LIST).expect("the list");
    let dir = cache(name, answers);
    let failed = build(&list, &Fetcher::cache_only(&dir), &no_segment);
    match failed {
        Err(PackError::Pack { item, problem }) => (item, problem),
        other => panic!("not a failed item: {:?}", other.map(|built| built.pack.id)),
    }
}

#[test]
fn a_wrong_licence_from_each_new_source_fails_naming_the_item() {
    let wrong = |at: usize, answer: String| {
        let mut all = answers();
        all[at] = answer;
        all
    };
    let (item, problem) = failure("comp-aic", wrong(0, aic(false)));
    assert_eq!(item, "stacks");
    assert!(problem.contains("not public domain"), "{problem}");
    let (item, problem) = failure("comp-nga", wrong(1, nga_pictures("0")));
    assert_eq!(item, "mill");
    assert!(problem.contains("openaccess 0"), "{problem}");
    let in_copyright = rijks("SK-A-1718", "http://rightsstatements.org/vocab/InC/1.0/");
    let (item, problem) = failure("comp-rijks", wrong(2, in_copyright));
    assert_eq!(item, "skaters");
    assert!(problem.contains("rightsstatements.org"), "{problem}");
    let zero = rijks(
        "SK-A-1718",
        "https://creativecommons.org/publicdomain/zero/1.0/",
    );
    let (item, problem) = failure("comp-rijks-cc0", wrong(2, zero));
    assert_eq!(item, "skaters");
    assert!(
        problem.contains("states the licence CC0, the list expects PD-old"),
        "{problem}"
    );
    let (item, problem) = failure(
        "comp-film",
        wrong(3, commons_film("CC BY-SA 3.0", "cc-by-sa-3.0")),
    );
    assert_eq!(item, "forest");
    assert!(problem.contains("CC BY-SA 3.0"), "{problem}");
}

#[test]
fn a_composition_list_refuses_an_outline_mark_entry_and_frames_off_commons() {
    let refused = |from: &str, to: &str| {
        let text = LIST.replacen(from, to, 1);
        match SourceList::parse(&text) {
            Err(PackError::List(why)) => why,
            other => panic!("read: {:?}", other.map(|list| list.id)),
        }
    };
    let why = refused("tier = \"simple\"", "tier = \"entry\"");
    assert!(
        why.contains("stacks: its tier is not one its kind takes"),
        "{why}"
    );
    let why = refused(
        "licence = \"CC0\"",
        "licence = \"CC0\"\nno_outline = \"a test\"",
    );
    assert!(why.contains("its kind never has an outline"), "{why}");
    let why = refused("licence = \"CC0\"", "licence = \"CC0\"\nno_masses = \" \"");
    assert!(why.contains("no value masses and says not why"), "{why}");
    let why = refused("object = \"64818\"", "object = \"64818\"\nat = 3");
    assert!(why.contains("a frame comes from Commons"), "{why}");
    let why = refused("at = 120", "at = -1");
    assert!(why.contains("its frame's second"), "{why}");
}

/// The official list in `tools/packs/lists/compositions.toml`: about 60 items in three tiers,
/// every one with its reason.
#[test]
fn the_official_compositions_list_holds_about_60_items_in_three_tiers() {
    let path = repo_root().join("tools/packs/lists/compositions.toml");
    let list = SourceList::parse(&fs::read_to_string(&path).expect("the list")).expect("reads");
    assert_eq!(list.kind, PictureKind::Composition);
    for tier in PictureKind::Composition.tiers() {
        let count = list
            .items
            .iter()
            .filter(|item| item.tier == Some(*tier))
            .count();
        assert!((15..=25).contains(&count), "{}: {count} items", tier.name());
    }
    assert!(
        (55..=65).contains(&list.items.len()),
        "{} items",
        list.items.len()
    );
    assert!(list.items.iter().all(|item| item.reason.len() >= 24));
}
