//! The composition pack a scenario's picker offers (`SystemAnswer::TestCompositions`, S95): small
//! landscapes of a few masses painted by the practice crate (`Masses`: a sky, a ridge, a field, a
//! tree and a house, by day or in the evening), no third-party picture, built as the harness builds
//! the official pack (each made as a pack holds it, never outlined, its value masses made from it,
//! S96). Three simple pictures and two medium ones, one of them upright; the same bytes every run.

use std::sync::Arc;

use openpaint_practice::{Credit, Licence, Masses, Pack, PackItem, PictureKind, Tier, ValueMasses};
use openpaint_runtime::ImageFormat;
use openpaint_store::{encode_image, encode_pack, pack_picture};

/// Where the test pictures say they come from.
const SOURCE: &str = "https://github.com/zhangyushao0/openpaint";
/// A test picture's width, in pixels.
const WIDTH: u32 = 640;

/// The pictures: id, tier, the landscape's seed and its height for a width of one.
const PICTURES: [(&str, Tier, u64, f64); 5] = [
    ("valley-house", Tier::Simple, 0x5EED_0095_0001, 0.75),
    ("ridge-farm", Tier::Simple, 0x5EED_0095_0002, 0.66),
    ("evening-field", Tier::Simple, 0x5EED_0095_0003, 0.8),
    ("high-meadow", Tier::Medium, 0x5EED_0095_0004, 1.25),
    ("hill-cottage", Tier::Medium, 0x5EED_0095_0005, 0.72),
];

/// The test composition pack's file; `None` when it cannot be made (a bug).
pub(crate) fn test_compositions() -> Option<Vec<u8>> {
    let mut items = Vec::new();
    let mut files = Vec::new();
    for (id, tier, seed, aspect) in PICTURES {
        let (size, pixels) = Masses::generate(seed, aspect).picture(WIDTH);
        let file = encode_image(ImageFormat::Png, size, &pixels).ok()?;
        let picture = pack_picture(&file).ok()?;
        let title = format!("Test {id}");
        items.push(PackItem {
            id: id.to_owned(),
            kind: PictureKind::Composition,
            tier,
            size: picture.size,
            credit: Some(Credit {
                source: SOURCE.to_owned(),
                creator: "The OpenPaint tests".to_owned(),
                licence: Licence::Cc0,
                attribution: format!("“{title}” by the OpenPaint tests ({SOURCE})."),
                notes: "painted for the scenarios".to_owned(),
            }),
            title,
            outline: None,
            masses: Some(Arc::new(ValueMasses::of(picture.size, &picture.rgba))),
        });
        files.push(picture.file);
    }
    let pack = Pack {
        id: "test-compositions".to_owned(),
        title: "Test compositions".to_owned(),
        items,
    };
    encode_pack(&pack, &files).ok()
}
