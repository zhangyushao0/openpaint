//! The practice packs a scenario's picker offers (`SystemAnswer::TestPack`, `TestFrames`):
//! pictures drawn here for the tests (no third-party picture), built as the harness builds the
//! official pack: each made as a pack holds it and its outline derived. The test pack: two entry
//! figures on transparency (one CC BY with its credit, so the credit shows under it), the one
//! simple picture on a busy ground, which gets no outline (so a first stage-2 session meets it),
//! and objects on a studio sweep and on flat white in the harder tiers, all square. The frames
//! pack (S99): an upright vase and a long fish on studio sweeps, pictures that are not square.
//! The same bytes every run.

use openpaint_practice::{Credit, Licence, Pack, PackItem, PictureKind, Random, Tier, outline};
use openpaint_practice_store::{encode_pack, pack_picture};
use openpaint_runtime::ImageFormat;
use openpaint_store::encode_image;

/// Where the test pictures say they come from.
const SOURCE: &str = "https://github.com/zhangyushao0/openpaint";
/// A test picture's side, in pixels.
const SIDE: u32 = 480;

/// The frames pack's pictures: id, tier, size, ground and figure.
const FRAMES: [(&str, Tier, [u32; 2], Ground, Figure); 2] = [
    (
        "tall-vase",
        Tier::Entry,
        [360, 480],
        Ground::Studio,
        Figure::Vase,
    ),
    (
        "long-fish",
        Tier::Simple,
        [480, 300],
        Ground::Studio,
        Figure::Fish,
    ),
];

/// The pictures: id, tier, licence, ground and figure.
const PICTURES: [(&str, Tier, Licence, Ground, Figure); 6] = [
    (
        "blob-cat",
        Tier::Entry,
        Licence::CcBy4,
        Ground::Clear,
        Figure::Cat,
    ),
    (
        "round-bird",
        Tier::Entry,
        Licence::Cc0,
        Ground::Clear,
        Figure::Bird,
    ),
    (
        "vase",
        Tier::Medium,
        Licence::Cc0,
        Ground::Studio,
        Figure::Vase,
    ),
    (
        "lamp",
        Tier::Hard,
        Licence::Cc0,
        Ground::White,
        Figure::Lamp,
    ),
    (
        "jar-on-cloth",
        Tier::Simple,
        Licence::Cc0,
        Ground::Busy,
        Figure::Vase,
    ),
    (
        "lamp-on-sweep",
        Tier::Medium,
        Licence::Cc0,
        Ground::Studio,
        Figure::Lamp,
    ),
];

/// The test pack file; `None` when it cannot be made (a bug).
pub(crate) fn test_pack() -> Option<Vec<u8>> {
    let pictures = PICTURES.map(|(id, tier, licence, ground, figure)| {
        (id, tier, licence, draw(ground, figure, [SIDE; 2]))
    });
    built(("test-shapes", "Test shapes"), pictures)
}

/// The frames pack's file (S99): pictures that are not square; `None` when it cannot be made.
pub(crate) fn test_frames() -> Option<Vec<u8>> {
    let pictures = FRAMES.map(|(id, tier, size, ground, figure)| {
        (id, tier, Licence::Cc0, draw(ground, figure, size))
    });
    built(("test-frames", "Test frames"), pictures)
}

/// A picture to pack: its id, tier, licence and its size and pixels.
type Drawn<'a> = (&'a str, Tier, Licence, ([u32; 2], Vec<u8>));

/// The pack `(id, title)` of `pictures`.
fn built<const N: usize>((id, title): (&str, &str), pictures: [Drawn<'_>; N]) -> Option<Vec<u8>> {
    let mut items = Vec::new();
    let mut files = Vec::new();
    for (id, tier, licence, picture) in pictures {
        let (item, file) = item(id, tier, licence, picture)?;
        items.push(item);
        files.push(file);
    }
    let pack = Pack {
        id: id.to_owned(),
        title: title.to_owned(),
        items,
    };
    encode_pack(&pack, &files).ok()
}

/// The item `id` of `pixels` (`size` pixels) as a pack holds it, with its credit and the outline
/// derived from it, and its picture's file.
fn item(
    id: &str,
    tier: Tier,
    licence: Licence,
    (size, pixels): ([u32; 2], Vec<u8>),
) -> Option<(PackItem, Vec<u8>)> {
    let file = encode_image(ImageFormat::Png, size, &pixels).ok()?;
    let picture = pack_picture(&file).ok()?;
    let title = format!("Test {id}");
    let attribution = format!(
        "“{title}” by the OpenPaint tests ({SOURCE}), licensed {} ({}).",
        licence.name(),
        licence.deed().unwrap_or(SOURCE)
    );
    let item = PackItem {
        id: id.to_owned(),
        kind: PictureKind::Shape,
        tier,
        size: picture.size,
        title,
        credit: Some(Credit {
            source: SOURCE.to_owned(),
            creator: "The OpenPaint tests".to_owned(),
            licence,
            attribution,
            notes: "drawn for the scenarios".to_owned(),
        }),
        outline: outline(picture.size, &picture.rgba).ok(),
        masses: None,
        areas: None,
    };
    Some((item, picture.file))
}

#[derive(Debug, Clone, Copy)]
enum Ground {
    Clear,
    Studio,
    White,
    Busy,
}

#[derive(Debug, Clone, Copy)]
enum Figure {
    Cat,
    Bird,
    Vase,
    Lamp,
    Fish,
}

/// Whether `figure` covers the point `(x, y)` of a picture, in shares of its width and height.
fn covers(figure: Figure, x: f64, y: f64) -> bool {
    let ellipse = |cx: f64, cy: f64, rx: f64, ry: f64| {
        ((x - cx) / rx).powi(2) + ((y - cy) / ry).powi(2) <= 1.0
    };
    match figure {
        Figure::Cat => {
            let ears = (y > 0.18 && y < 0.36)
                && ((x - 0.36).abs() < (y - 0.18) * 0.5 || (x - 0.64).abs() < (y - 0.18) * 0.5);
            let tail = (x > 0.66 && x < 0.86) && (y - (0.78 - 0.9 * (x - 0.66))).abs() < 0.035;
            ellipse(0.5, 0.42, 0.18, 0.15) || ellipse(0.5, 0.68, 0.22, 0.2) || ears || tail
        }
        Figure::Bird => {
            let beak = x > 0.66 && x < 0.8 && (y - 0.42).abs() < (0.8 - x) * 0.35;
            ellipse(0.48, 0.55, 0.26, 0.22) || ellipse(0.6, 0.38, 0.11, 0.1) || beak
        }
        Figure::Vase => {
            let neck =
                (y > 0.18 && y < 0.42) && (x - 0.5).abs() < 0.07 + 0.08 * (0.42 - y).max(0.0);
            ellipse(0.5, 0.6, 0.22, 0.24)
                || neck
                || ((y - 0.84).abs() < 0.03 && (x - 0.5).abs() < 0.14)
        }
        Figure::Lamp => {
            let shade = (y > 0.18 && y < 0.42) && (x - 0.5).abs() < 0.1 + 0.4 * (y - 0.18);
            let stem = (0.42..0.78).contains(&y) && (x - 0.5).abs() < 0.03;
            let foot = (0.78..0.84).contains(&y) && (x - 0.5).abs() < 0.16;
            shade || stem || foot
        }
        Figure::Fish => {
            let tail = (0.74..0.92).contains(&x) && (y - 0.5).abs() < (x - 0.7) * 1.4;
            let fin = (0.36..0.56).contains(&x) && y > 0.14 + (x - 0.36) * 0.55 && y < 0.3;
            ellipse(0.44, 0.5, 0.32, 0.27) || tail || fin
        }
    }
}

/// The picture: `figure` on `ground`, `size` pixels, and its pixels as 8-bit RGBA.
fn draw(ground: Ground, figure: Figure, size: [u32; 2]) -> ([u32; 2], Vec<u8>) {
    let mut noise = Random::new(0x5EED_0080);
    let colour = match figure {
        Figure::Cat => [232.0, 140.0, 60.0],
        Figure::Bird => [70.0, 120.0, 200.0],
        Figure::Vase => [70.0, 110.0, 90.0],
        Figure::Lamp => [120.0, 80.0, 50.0],
        Figure::Fish => [196.0, 96.0, 70.0],
    };
    let [width, height] = size;
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for row in 0..height {
        for column in 0..width {
            let (x, y) = (
                (f64::from(column) + 0.5) / f64::from(width),
                (f64::from(row) + 0.5) / f64::from(height),
            );
            let grain = noise.range(-3.0, 3.0);
            let under = match ground {
                Ground::Clear => [0.0, 0.0, 0.0, 0.0],
                Ground::White => [255.0, 255.0, 255.0, 255.0],
                Ground::Studio => {
                    let base = 205.0 - 50.0 * y - 30.0 * (x - 0.5).powi(2) + grain;
                    [base, base, base + 3.0, 255.0]
                }
                Ground::Busy => {
                    let stripe = ((x * 23.0).sin() * (y * 17.0).cos() * 90.0) + 120.0;
                    [stripe, 255.0 - stripe, 90.0 + stripe / 3.0, 255.0]
                }
            };
            let pixel = if covers(figure, x, y) {
                let shade = 25.0 * (y - 0.5);
                colour
                    .map(|c| c - shade + grain)
                    .into_iter()
                    .chain([255.0])
                    .collect::<Vec<f64>>()
            } else {
                under.to_vec()
            };
            pixels.extend(pixel.iter().map(|c| c.clamp(0.0, 255.0).round() as u8));
        }
    }
    (size, pixels)
}
