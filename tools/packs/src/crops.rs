//! A pack's crops (S101, V大's 色彩组合: a corner of an abstract oil painting or of a rusted old
//! thing, never a concrete object): the crops a picture gives (`list::cropping`: a composition's
//! best corner, a texture's best two places), found and held to a combination's rules by the
//! practice crate (`openpaint_practice::candidates`, `best`), each cut from the picture at its full
//! size (`full.rs`, S101b: at least `LEAST_LONGER` long, never the packed picture's 2,048
//! pixels), reduced to `LONGEST`, made as a pack holds a picture and answered anew from those
//! pixels (`Areas::found`), so the answer is the picture's the app shows. A crop the list leaves
//! out (`no_crop`) is still cut and drawn on the sheets, never packed. Their contact sheets:
//! `<pack id>-crops-<page>.png`, `PAGE` crops a page, each beside its areas, numbered as the
//! command lists them.

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, mpsc};
use std::thread;

use image::RgbaImage;
use image::imageops::{self, FilterType};
use openpaint_app::ImageFormat;
use openpaint_practice::{
    Areas, Credit, Crop, Found, LEAST_LONGER, PackItem, PictureKind, Places, best, candidate, cut,
    rectangles, refusal, tier_of,
};
use openpaint_practice_store::pack_picture;
use openpaint_store::encode_image;

use super::build::attribution;
use super::full::FullPicture;
use super::list::Listed;
use super::sheet::{Mark, Picture, write_sheet};
use super::sources;
use crate::error::PackError;

/// A crop's longest side in the pack, in pixels: as large as the app shows a picture.
const LONGEST: u32 = 1024;
/// Crops a contact sheet's page holds.
const PAGE: usize = 18;

/// A crop kept: its item and its picture's file.
pub type Packed = (PackItem, Vec<u8>);

/// A crop as cut: its id, title and picture as the pack holds it (decoded), its answer, and why
/// the list leaves it out, if it does.
pub struct Cropped {
    pub id: String,
    pub title: String,
    pub size: [u32; 2],
    pub rgba: Vec<u8>,
    pub found: Found,
    pub left_out: Option<String>,
}

/// The crops `listed`'s picture (at its full size; `source` what its API says of it) gives at
/// `places`, `count` at most, best first; each with its item and file unless the list leaves it
/// out or its answer, made anew from the packed pixels, breaks a rule.
///
/// # Errors
/// `Pack` when a crop cannot be made as a pack holds a picture.
pub fn crops_of(
    listed: &Listed,
    (source, picture): (&sources::Found, &FullPicture),
    (places, count): (Places, usize),
) -> Result<Vec<(Cropped, Option<Packed>)>, PackError> {
    let all = found_side_by_side(picture, places);
    let kept = best(all.clone(), count);
    if kept.is_empty() {
        say_none(listed, &all);
    }
    let mut made = Vec::with_capacity(kept.len());
    for (number, crop) in (1..).zip(kept) {
        let pixels = cut(picture.size, &picture.rgba, crop.at, crop.size);
        let (size, pixels) = reduced(crop.size, pixels);
        let failed = |detail: String| PackError::Pack {
            item: listed.id.clone(),
            problem: format!("its crop {number}: {detail}"),
        };
        let file =
            encode_image(ImageFormat::Png, size, &pixels).map_err(|e| failed(e.to_string()))?;
        let packed = pack_picture(&file).map_err(|e| failed(e.to_string()))?;
        let found = Areas::found(packed.size, &packed.rgba);
        let left_out = (listed.no_crop.get(&number).cloned())
            .or_else(|| refusal(&found).map(|rule| format!("{rule} once packed")));
        let cropped = Cropped {
            id: format!("{}-{number}", listed.id),
            title: format!("{} (detail {number})", source.title),
            size: packed.size,
            rgba: packed.rgba.clone(),
            found,
            left_out,
        };
        let item = cropped.left_out.is_none().then(|| {
            let item = item(listed, source, &cropped);
            (item, packed.file)
        });
        made.push((cropped, item));
    }
    Ok(made)
}

/// Every candidate crop of `picture` at `places` with its areas found (`candidates`), the
/// candidates shared out over the machine's threads, each found alone, so in the same order and
/// the same as one after another (a 4,096-pixel picture's 81 take about a minute alone).
fn found_side_by_side(picture: &FullPicture, places: Places) -> Vec<Crop> {
    let rectangles = rectangles(picture.size, places);
    let workers = thread::available_parallelism().map_or(1, usize::from);
    let next = AtomicUsize::new(0);
    let (send, receive) = mpsc::channel();
    thread::scope(|scope| {
        let (rectangles, next) = (&rectangles, &next);
        for _ in 0..workers.min(rectangles.len()) {
            let send = send.clone();
            scope.spawn(move || {
                loop {
                    let at = next.fetch_add(1, Ordering::Relaxed);
                    let Some(rectangle) = rectangles.get(at) else {
                        break;
                    };
                    let found = candidate(picture.size, &picture.rgba, *rectangle);
                    if send.send((at, found)).is_err() {
                        break;
                    }
                }
            });
        }
    });
    drop(send);
    let mut found: Vec<(usize, Crop)> = receive.into_iter().collect();
    found.sort_by_key(|(at, _)| *at);
    found.into_iter().map(|(_, crop)| crop).collect()
}

/// That `listed`'s picture gives no crop, and why: the rule its best candidate breaks.
fn say_none(listed: &Listed, all: &[Crop]) {
    let nearest = all.iter().max_by(|a, b| a.score().total_cmp(&b.score()));
    let why = nearest.map_or_else(
        || format!("too small: no crop of it reaches {LEAST_LONGER} px"),
        |crop| {
            let rule = refusal(&crop.found).map_or_else(String::new, |rule| rule.to_string());
            let areas = crop.found.areas.count();
            format!(
                "{rule}; its best of {} has {areas} areas, busy {:.3}",
                all.len(),
                crop.found.busy
            )
        },
    );
    println!("no crop    {:<36} {why}", listed.id);
}

/// `pixels` of `size` reduced to `LONGEST` on their longer side, if longer.
fn reduced(size: [u32; 2], pixels: Vec<u8>) -> ([u32; 2], Vec<u8>) {
    let longer = size[0].max(size[1]);
    let Some(image) = (longer > LONGEST)
        .then(|| RgbaImage::from_raw(size[0], size[1], pixels.clone()))
        .flatten()
    else {
        return (size, pixels);
    };
    let scaled = size.map(|side| (u64::from(side) * u64::from(LONGEST) / u64::from(longer)) as u32);
    let resized = imageops::resize(&image, scaled[0], scaled[1], FilterType::Lanczos3);
    (scaled, resized.into_raw())
}

/// The pack's item of `cropped`, credited as its picture with the change said.
fn item(listed: &Listed, source: &sources::Found, cropped: &Cropped) -> PackItem {
    let detail = sources::Found {
        changes: "a detail cropped from it and resized",
        ..source.clone()
    };
    PackItem {
        id: cropped.id.clone(),
        kind: PictureKind::Crop,
        tier: tier_of(&cropped.found.areas),
        size: cropped.size,
        title: cropped.title.clone(),
        credit: Some(Credit {
            attribution: attribution(&detail, listed.licence),
            source: source.page.clone(),
            creator: source.creator.clone(),
            licence: listed.licence,
            notes: listed.reason.clone(),
        }),
        outline: None,
        masses: None,
        areas: Some(Arc::new(cropped.found.areas.clone())),
    }
}

/// The crops' contact sheets in `dir`, `<pack>-crops-<page>.png`, each crop beside its areas,
/// numbered from 1 in the pack's order.
///
/// # Errors
/// `Io` or `Png` when a sheet cannot be written.
pub fn write_crop_sheets(crops: &[Cropped], pack: &str, dir: &Path) -> Result<(), PackError> {
    for (page, chunk) in crops.chunks(PAGE).enumerate() {
        let pictures: Vec<Picture> = (chunk.iter())
            .map(|crop| {
                let mark = if crop.left_out.is_some() {
                    Mark::Dropped
                } else {
                    Mark::Kept
                };
                Picture {
                    size: crop.size,
                    rgba: crop.rgba.clone(),
                    outline: None,
                    masses: None,
                    areas: Some((crop.found.areas.clone(), mark)),
                }
            })
            .collect();
        let path = dir.join(format!("{pack}-crops-{}.png", page + 1));
        write_sheet(&pictures, page * PAGE + 1, &path)?;
        println!("{}: {} crops", path.display(), pictures.len());
    }
    Ok(())
}

/// One line per crop, numbered as its sheet numbers it: its id, size, tier, areas, weak
/// contrasts and how busy, and whether it goes in.
pub fn describe_crops(crops: &[Cropped]) {
    for (number, crop) in (1..).zip(crops) {
        let areas = &crop.found.areas;
        let fate = crop
            .left_out
            .as_ref()
            .map_or_else(|| "kept".to_owned(), |why| format!("none: {why}"));
        println!(
            "crop {number:>3}  {:<36} {:>4} x {:<4} {:<6} areas {} weak {} busy {:.3} | {fate}",
            crop.id,
            crop.size[0],
            crop.size[1],
            tier_of(areas).name(),
            areas.count(),
            areas.weak_pairs().len(),
            crop.found.busy,
        );
    }
}
