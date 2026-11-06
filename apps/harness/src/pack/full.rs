//! A picture's full pixels for its crops (S101b): never the packed picture, whose 2,048 pixels
//! leave a crop too small to show sharp on the tablet, but the source's own size fitted within
//! `FULL_SIDE` (`sources::Full`): the file downloaded when it is the source's own (Commons'
//! original, the Met's primary picture, Cleveland's print); a IIIF service's picture asked at
//! its full size (`!n,n`), its `info.json` read first so it is never enlarged nor asked past the
//! server's `maxArea` or the most it hands out (3,000 pixels at the Art Institute); a film's
//! still asked of Commons again at the video's own width. Every download comes through the cache
//! under the item's key and `full`, and decoding (`decode_image`: upright, into sRGB) and reducing
//! are exact, so the same list and cache give the same crops.

use image::RgbaImage;
use image::imageops::{self, FilterType};
use openpaint_store::decode_image;
use serde_json::Value;

use super::commons;
use super::fetch::{Fetcher, cache_name};
use super::list::Listed;
use super::sources::{Found, Full, lacking};
use crate::error::HarnessError;

/// The longest side a picture is cropped at, in pixels.
pub const FULL_SIDE: u32 = 4096;

/// A picture at its full size: its size and pixels (8-bit sRGB RGBA).
pub struct FullPicture {
    pub size: [u32; 2],
    pub rgba: Vec<u8>,
}

/// The cache's name of `listed`'s full picture or answer, ending `extension`.
pub fn full_name(listed: &Listed, extension: &str) -> String {
    cache_name(
        listed.source.name(),
        &format!("{} full", listed.key()),
        extension,
    )
}

/// `listed`'s picture (as `found` describes it) at its full size, its longer side at most
/// `FULL_SIDE`.
///
/// # Errors
/// `Fetch` when a download fails, an answer lacks the size it is read for, or the picture does
/// not decode.
pub fn full_picture(
    listed: &Listed,
    found: &Found,
    fetcher: &Fetcher,
) -> Result<FullPicture, HarnessError> {
    let (url, bytes) = match &found.full {
        Full::Picture => {
            let name = cache_name(listed.source.name(), &listed.key(), "img");
            let bytes = fetcher.get_with(&name, &found.picture, found.headers)?;
            (found.picture.clone(), bytes)
        }
        Full::Iiif { base, most } => {
            let info_url = format!("{base}/info.json");
            let info = fetcher.json_with(&full_name(listed, "json"), &info_url, found.headers)?;
            let side = iiif_side(&info, &info_url, *most)?;
            let url = format!("{base}/full/!{side},{side}/0/default.jpg");
            let bytes = fetcher.get_with(&full_name(listed, "img"), &url, found.headers)?;
            (url, bytes)
        }
        Full::Still { width } => {
            let at = (listed.at).ok_or_else(|| lacking(&found.page, "second of its still"))?;
            let answer_url = commons::still_url(&listed.object, at, (*width).min(FULL_SIDE));
            let answer = fetcher.json(&full_name(listed, "json"), &answer_url)?;
            let url = commons::found(&answer, &answer_url, Some(at))?.picture;
            let bytes = fetcher.get(&full_name(listed, "img"), &url)?;
            (url, bytes)
        }
    };
    let (size, rgba) = decode_image(&bytes).map_err(|error| HarnessError::Fetch {
        url: url.clone(),
        detail: format!("the picture: {error}"),
    })?;
    fitted(size, rgba).ok_or_else(|| HarnessError::Fetch {
        url,
        detail: "the picture's pixels do not match its size".to_owned(),
    })
}

/// The longer side a IIIF service's picture is asked at (fitted in a square of it): its full
/// size's (`width`, `height` of its `info.json`), within `most` and its `maxArea` when it states
/// one, never enlarged.
fn iiif_side(info: &Value, url: &str, most: u32) -> Result<u32, HarnessError> {
    let side = |field: &str| {
        (info[field].as_u64())
            .and_then(|side| u32::try_from(side).ok())
            .filter(|side| *side > 0)
            .map(f64::from)
            .ok_or_else(|| lacking(url, field))
    };
    let [width, height] = [side("width")?, side("height")?];
    let longer = width.max(height);
    // IIIF 3 states `maxArea` at the top, IIIF 2 in its profile's description.
    let max_area = (info["maxArea"].as_f64()).or_else(|| {
        (info["profile"].as_array()?.iter()).find_map(|profile| profile["maxArea"].as_f64())
    });
    let area = max_area.map_or(1.0, |area| (area / (width * height)).sqrt());
    let scale = (f64::from(most.min(FULL_SIDE)) / longer).min(area).min(1.0);
    Ok(((longer * scale).floor() as u32).max(1))
}

/// The picture of `size` and `rgba`, its longer side reduced to `FULL_SIDE` when longer
/// (Lanczos-3); `None` when the pixels do not fill the size.
fn fitted(size: [u32; 2], rgba: Vec<u8>) -> Option<FullPicture> {
    let longer = size[0].max(size[1]);
    if longer <= FULL_SIDE {
        return Some(FullPicture { size, rgba });
    }
    let image = RgbaImage::from_raw(size[0], size[1], rgba)?;
    let scaled = size
        .map(|side| ((u64::from(side) * u64::from(FULL_SIDE) / u64::from(longer)) as u32).max(1));
    let reduced = imageops::resize(&image, scaled[0], scaled[1], FilterType::Lanczos3);
    Some(FullPicture {
        size: scaled,
        rgba: reduced.into_raw(),
    })
}
