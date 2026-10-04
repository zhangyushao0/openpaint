//! Colour-managed pictures (S27): Display P3 fixtures come in as their sRGB colours, untagged
//! files keep every value, and exports declare sRGB in a profile a reader finds again.

use std::io::Cursor;

use image::{ImageDecoder, ImageReader};
use openpaint_app::ImageFormat;
use openpaint_core::CanvasSize;
use openpaint_store::{decode_image, encode_image, open_picture, open_reference};

/// 512 x 256, eight 128-pixel patches in Display P3 values with the Display P3 profile in
/// `iCCP` (PNG) and APP2 (JPEG, quality 95), made once by a throwaway tool with moxcms.
const P3_PNG: &[u8] = include_bytes!("../../../tests/fixtures/s27-p3.png");
const P3_JPEG: &[u8] = include_bytes!("../../../tests/fixtures/s27-p3.jpg");
const CUTOUT: &[u8] = include_bytes!("../../../tests/fixtures/s26-cutout.png");
const PHOTO: &[u8] = include_bytes!("../../../tests/fixtures/s26-photo.jpg");

/// The fixtures' patches, left to right, top row first, in Display P3 values; the last one is
/// half transparent in the PNG.
const PATCHES: [[u8; 4]; 8] = [
    [200, 100, 50, 255],
    [60, 160, 90, 255],
    [70, 90, 200, 255],
    [128, 128, 128, 255],
    [255, 0, 0, 255],
    [0, 255, 0, 255],
    [240, 200, 40, 255],
    [200, 100, 50, 128],
];

/// Each patch in sRGB, worked by hand (decode, the P3-D65 to sRGB matrix, clip, encode): the
/// orange, green, blue and yellow move; grey stays; pure P3 red and green clip to sRGB's.
const IN_SRGB: [[u8; 3]; 8] = [
    [215, 93, 31],
    [0, 163, 82],
    [64, 91, 207],
    [128, 128, 128],
    [255, 0, 0],
    [0, 255, 0],
    [248, 198, 0],
    [215, 93, 31],
];

/// The middle pixel of each patch.
fn patches(size: [u32; 2], pixels: &[u8]) -> Vec<[u8; 4]> {
    assert_eq!(size, [512, 256]);
    (0..8)
        .map(|i| {
            let (x, y) = (i % 4 * 128 + 64, i / 4 * 128 + 64);
            let at = ((y * 512 + x) * 4) as usize;
            [pixels[at], pixels[at + 1], pixels[at + 2], pixels[at + 3]]
        })
        .collect()
}

fn assert_near(got: &[[u8; 4]], tolerance: u8, alpha: bool) {
    for ((pixel, want), p3) in got.iter().zip(IN_SRGB).zip(PATCHES) {
        for channel in 0..3 {
            assert!(
                pixel[channel].abs_diff(want[channel]) <= tolerance,
                "P3 {p3:?} came in as {pixel:?}, sRGB is {want:?}"
            );
        }
        assert_eq!(pixel[3], if alpha { p3[3] } else { 255 });
    }
}

#[test]
fn display_p3_pictures_come_in_with_their_srgb_colours() {
    let (size, png) = decode_image(P3_PNG).expect("png");
    assert_near(&patches(size, &png), 1, true);
    let (size, jpeg) = decode_image(P3_JPEG).expect("jpeg");
    assert_near(&patches(size, &jpeg), 3, false);
    // Both flows take the converted colours: the picture to insert (premultiplied) and the
    // reference.
    let canvas = CanvasSize::new(1024, 512).expect("canvas");
    let picture = open_picture(P3_PNG, canvas).expect("picture");
    assert_eq!((picture.origin(), picture.size()), ([256, 128], [512, 256]));
    assert_eq!(&picture.pixels()[..4], [215, 93, 31, 255]);
    let reference = open_reference(P3_JPEG).expect("reference");
    let orange = reference.color_at([64, 64]).expect("inside");
    let red = (orange.r() * 255.0).round() as u8;
    assert!(red.abs_diff(215) <= 3, "{orange:?}");
}

/// The file's own values, read without any colour management.
fn raw(bytes: &[u8]) -> Vec<u8> {
    image::load_from_memory(bytes)
        .expect("decodes")
        .into_rgba8()
        .into_raw()
}

#[test]
fn untagged_pictures_keep_every_value() {
    for fixture in [CUTOUT, PHOTO] {
        let mut decoder = ImageReader::new(Cursor::new(fixture))
            .with_guessed_format()
            .expect("format")
            .into_decoder()
            .expect("decoder");
        assert_eq!(decoder.icc_profile().expect("read"), None, "untagged");
    }
    assert_eq!(decode_image(CUTOUT).expect("cutout").1, raw(CUTOUT));
    // The photo is turned upright, so compare its pixel count and a value-for-value sum.
    let (_, photo) = decode_image(PHOTO).expect("photo");
    let sum = |pixels: &[u8]| pixels.iter().map(|&v| u64::from(v)).sum::<u64>();
    assert_eq!(sum(&photo), sum(&raw(PHOTO)));
    // The P3 fixture's values without its profile are the P3 numbers, unconverted.
    assert_eq!(patches([512, 256], &raw(P3_PNG))[0], PATCHES[0]);
}

#[test]
fn exports_declare_srgb_and_come_back_exactly() {
    let pixels: Vec<u8> = (0..32 * 16)
        .flat_map(|i| [(i % 251) as u8, (i * 7 % 256) as u8, 90, 255])
        .collect();
    for format in [ImageFormat::Png, ImageFormat::Jpeg] {
        let file = encode_image(format, [32, 16], &pixels).expect("encode");
        let mut decoder = ImageReader::new(Cursor::new(&file))
            .with_guessed_format()
            .expect("format")
            .into_decoder()
            .expect("decoder");
        let profile = decoder.icc_profile().expect("read").expect("a profile");
        let srgb = moxcms::ColorProfile::new_from_slice(&profile).expect("an ICC profile");
        let reference = moxcms::ColorProfile::new_srgb();
        assert_eq!(srgb.color_space, moxcms::DataColorSpace::Rgb);
        assert_eq!(
            [srgb.red_colorant, srgb.green_colorant, srgb.blue_colorant],
            [
                reference.red_colorant,
                reference.green_colorant,
                reference.blue_colorant
            ],
            "{format:?} declares sRGB's primaries"
        );
        let description = format!("{:?}", srgb.description);
        assert!(description.contains("sRGB IEC61966-2.1"), "{description}");
        if format == ImageFormat::Png {
            assert!(file.windows(4).any(|chunk| chunk == b"iCCP"));
            // Read back through the app's own path, the sRGB file keeps every byte.
            assert_eq!(decode_image(&file).expect("decode").1, pixels);
        } else {
            assert!(file.windows(12).any(|segment| segment == b"ICC_PROFILE\0"));
        }
    }
}
