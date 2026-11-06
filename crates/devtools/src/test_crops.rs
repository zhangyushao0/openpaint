//! The crops pack a scenario's picker offers (`SystemAnswer::TestCrops`, S101): colour
//! combinations whose areas are known, S82's generated pictures (no third-party picture), square
//! so their frames lie where a generated combination's did, built as the harness builds the
//! official packs: each made as a pack holds it, its answer found from it by the builder's rule
//! (`Areas::found`), its tier by its areas. Four crops; the same bytes every run.

use std::sync::Arc;

use openpaint_practice::{
    Areas, Arrangement, Credit, GeneratedAreas, Licence, Pack, PackItem, PictureKind, tier_of,
};
use openpaint_runtime::ImageFormat;
use openpaint_store::{encode_image, encode_pack, pack_picture};

/// Where the test pictures say they come from.
const SOURCE: &str = "https://github.com/zhangyushao0/openpaint";
/// A test crop's side, in pixels.
const SIDE: u32 = 480;

/// The crops: id, arrangement and seed.
const CROPS: [(&str, Arrangement, u64); 4] = [
    ("rusted-bands", Arrangement::Strata, 0x5EED_0101_0001),
    ("lichen-patches", Arrangement::Patches, 0x5EED_0101_0002),
    ("painted-blocks", Arrangement::Blocks, 0x5EED_0101_0003),
    ("weathered-bands", Arrangement::Strata, 0x5EED_0101_0004),
];

/// The test crops pack's file; `None` when it cannot be made (a bug).
pub(crate) fn test_crops() -> Option<Vec<u8>> {
    let mut items = Vec::new();
    let mut files = Vec::new();
    for (id, arrangement, seed) in CROPS {
        let generated = GeneratedAreas::generate(arrangement, seed);
        let file = encode_image(ImageFormat::Png, [SIDE; 2], &generated.picture(SIDE)).ok()?;
        let picture = pack_picture(&file).ok()?;
        let areas = Areas::found(picture.size, &picture.rgba).areas;
        let title = format!("Test {id}");
        items.push(PackItem {
            id: id.to_owned(),
            kind: PictureKind::Crop,
            tier: tier_of(&areas),
            size: picture.size,
            credit: Some(Credit {
                source: SOURCE.to_owned(),
                creator: "The OpenPaint tests".to_owned(),
                licence: Licence::Cc0,
                attribution: format!("“{title}” by the OpenPaint tests ({SOURCE})."),
                notes: "generated for the scenarios: areas known".to_owned(),
            }),
            title,
            outline: None,
            masses: None,
            areas: Some(Arc::new(areas)),
        });
        files.push(picture.file);
    }
    let pack = Pack {
        id: "test-crops".to_owned(),
        title: "Test crops".to_owned(),
        items,
    };
    encode_pack(&pack, &files).ok()
}
