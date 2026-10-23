//! A pack built from its source list: each item looked up through its source's API, the licence
//! the API states checked against the list's (a licence that disagrees fails the build and names
//! the item), its picture downloaded, reduced and written as the pack holds it. Every download
//! comes through the cache, and decoding, reducing and encoding are exact, so the same list and
//! cache give the same pack byte for byte.

use image::RgbaImage;
use image::imageops::{self, FilterType};
use openpaint_app::ImageFormat;
use openpaint_practice::{Licence, MAX_PICTURE_SIDE, Pack, PackItem, PictureKind};
use openpaint_store::{decode_image, encode_image};

use super::fetch::{Fetcher, cache_name};
use super::list::{Listed, SourceList};
use super::sources::{Found, Stated, look_up};
use crate::error::HarnessError;

/// The pack `list` names and its pictures, in the list's order.
///
/// # Errors
/// `Pack` naming the first item that cannot go in: its licence disagrees with the list's, its
/// source cannot be reached or its picture does not decode, or it breaks a rule of a pack.
pub fn build(list: &SourceList, fetcher: &Fetcher) -> Result<(Pack, Vec<Vec<u8>>), HarnessError> {
    let mut items = Vec::with_capacity(list.items.len());
    let mut pictures = Vec::with_capacity(list.items.len());
    for listed in &list.items {
        let (item, picture) = item(list.kind, listed, fetcher).map_err(|error| match error {
            HarnessError::Pack { .. } => error,
            other => HarnessError::Pack {
                item: listed.id.clone(),
                problem: other.to_string(),
            },
        })?;
        items.push(item);
        pictures.push(picture);
    }
    let pack = Pack {
        id: list.id.clone(),
        title: list.title.clone(),
        items,
    };
    pack.check().map_err(|problem| HarnessError::Pack {
        item: problem.item.unwrap_or_else(|| list.id.clone()),
        problem: problem.rule.to_owned(),
    })?;
    Ok((pack, pictures))
}

fn item(
    kind: PictureKind,
    listed: &Listed,
    fetcher: &Fetcher,
) -> Result<(PackItem, Vec<u8>), HarnessError> {
    let found = look_up(fetcher, listed.source, &listed.object)?;
    if found.stated != Stated::Licence(listed.licence) {
        return Err(HarnessError::Pack {
            item: listed.id.clone(),
            problem: format!(
                "its source states the licence {}, the list expects {}",
                found.stated.name(),
                listed.licence.name()
            ),
        });
    }
    let name = cache_name(listed.source.name(), &listed.object, "img");
    let original = fetcher.get(&name, &found.picture)?;
    let (size, picture) = fitted(&original, &found.picture)?;
    let item = PackItem {
        id: listed.id.clone(),
        kind,
        tier: listed.tier,
        size,
        attribution: attribution(&found, listed.licence),
        source: found.page,
        title: found.title,
        creator: found.creator,
        licence: listed.licence,
        notes: listed.reason.clone(),
    };
    Ok((item, picture))
}

/// The credit shown with the picture: for an Attribution licence what it asks for (title,
/// creator with the attribution the author asked for, source, licence and its deed, the change
/// made); for the others a courtesy line naming the collection.
pub fn attribution(found: &Found, licence: Licence) -> String {
    let Found {
        title,
        creator,
        page,
        credit,
        collection,
        ..
    } = found;
    let by = if credit == creator {
        creator.clone()
    } else {
        format!("{creator} ({credit})")
    };
    match (licence, licence.deed()) {
        (licence, Some(deed)) if licence.requires_attribution() => format!(
            "“{title}” by {by}, {collection} ({page}), licensed {} ({deed}); resized.",
            licence.name()
        ),
        (_, Some(deed)) => format!("“{title}”, {creator}. {collection} ({page}), CC0 ({deed})."),
        (Licence::PdUsGov, None) => format!(
            "“{title}”, {creator}. {collection} ({page}); public domain, a work of the US \
             federal government."
        ),
        (_, None) => format!("“{title}”, {creator}. {collection} ({page}); public domain by age."),
    }
}

/// A picture file as a pack holds it: decoded into sRGB and turned upright (the store's
/// `decode_image`), its long side reduced to `MAX_PICTURE_SIDE` when longer (Lanczos-3 over
/// premultiplied pixels, so no transparent pixel's colour bleeds in), then a PNG when a pixel is
/// transparent, else a JPEG (the app's export encoders, sRGB declared, no other metadata).
///
/// # Errors
/// `Fetch` naming `url` when the file does not decode or encode.
pub fn fitted(original: &[u8], url: &str) -> Result<([u32; 2], Vec<u8>), HarnessError> {
    let failed = |detail: String| HarnessError::Fetch {
        url: url.to_owned(),
        detail,
    };
    let ([width, height], rgba) =
        decode_image(original).map_err(|error| failed(format!("the picture: {error}")))?;
    let mut picture = RgbaImage::from_raw(width, height, rgba)
        .ok_or_else(|| failed("the picture's pixels".to_owned()))?;
    let long = width.max(height);
    if long > MAX_PICTURE_SIDE {
        let scale = f64::from(MAX_PICTURE_SIDE) / f64::from(long);
        let side = |length: u32| ((f64::from(length) * scale).round() as u32).clamp(1, long);
        picture = reduced(&picture, [side(width), side(height)]);
    }
    let opaque = picture.pixels().all(|pixel| pixel[3] == 255);
    let format = if opaque {
        ImageFormat::Jpeg
    } else {
        ImageFormat::Png
    };
    let size = [picture.width(), picture.height()];
    let bytes = encode_image(format, size, picture.as_raw())
        .map_err(|error| failed(format!("the reduced picture: {error}")))?;
    Ok((size, bytes))
}

/// `picture` resized to `size` through premultiplied pixels.
fn reduced(picture: &RgbaImage, size: [u32; 2]) -> RgbaImage {
    let mut premultiplied = picture.clone();
    for pixel in premultiplied.pixels_mut() {
        let alpha = u32::from(pixel[3]);
        for channel in &mut pixel.0[..3] {
            *channel = ((u32::from(*channel) * alpha + 127) / 255) as u8;
        }
    }
    let mut out = imageops::resize(&premultiplied, size[0], size[1], FilterType::Lanczos3);
    for pixel in out.pixels_mut() {
        let alpha = u32::from(pixel[3]);
        for channel in &mut pixel.0[..3] {
            *channel = match alpha {
                0 => 0,
                255 => *channel,
                _ => ((u32::from(*channel) * 255 + alpha / 2) / alpha).min(255) as u8,
            };
        }
    }
    out
}
