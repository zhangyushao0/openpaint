//! The CC0 texture libraries a list of crops takes pictures from: ambientCG
//! (`ambientcg.com/api/v2/full_json`, its materials as zip archives of maps) and Poly Haven
//! (`api.polyhaven.com`, each map its own file), S101c; Texture Ninja (`texture.ninja/api`, about
//! 5,000 photographs of walls, rust, paint and stone, each file at its full size), S159. Of the
//! first two only the colour map (ambientCG's `Color`, Poly Haven's `Diffuse`) is ever taken,
//! never a normal, roughness or height map, at 4K (4,096 pixels a side). No API states a licence
//! per asset: each library dedicates every asset it publishes to the public domain under CC0
//! (ambientCG: "All assets are provided under the Creative Commons CC0 1.0 Universal License",
//! docs.ambientcg.com/license; Poly Haven: "All assets on Poly Haven are CC0",
//! polyhaven.com/license; Texture Ninja: "All textures on Texture Ninja are released into the
//! public domain (CC0)", beside every texture on texture.ninja), so an asset the API answers for
//! is CC0; one the API does not know fails the build.

use openpaint_practice::Licence;

use super::fetch::{Fetcher, cache_name, query_text};
use super::sources::{Found, Full, Stated, lacking, text};
use crate::error::PackError;

/// The archive of an ambientCG material taken: its maps at 4K as JPEG.
const AMBIENTCG_ARCHIVE: &str = "4K-JPG";
/// The colour map's name in an ambientCG archive ends so.
pub const AMBIENTCG_COLOUR: &str = "_Color.jpg";
/// Poly Haven's resolution taken, 4,096 pixels a side.
const POLYHAVEN_SIZE: &str = "4k";
/// Where Texture Ninja keeps its files at their full size (its Download button's), and their
/// previews (its cards').
pub const TEXTURE_NINJA_FILES: &str = "https://texture-ninja.nyc3.cdn.digitaloceanspaces.com/";
pub const TEXTURE_NINJA_PREVIEWS: &str = "https://d1wumrtbuo797i.cloudfront.net/thumb-";

/// The ambientCG material `id` (`PaintedMetal006`): its title, page and archive.
///
/// # Errors
/// `Fetch` when the API cannot be reached or knows no such material, or the material has no
/// archive at 4K.
pub fn ambientcg(fetcher: &Fetcher, id: &str) -> Result<Found, PackError> {
    let url = format!(
        "https://ambientcg.com/api/v2/full_json?id={}&include=downloadData",
        query_text(id)
    );
    let answer = fetcher.json(&cache_name("ambientcg", id, "json"), &url)?;
    let asset = (answer["foundAssets"].as_array().into_iter().flatten())
        .find(|asset| asset["assetId"] == id)
        .ok_or_else(|| lacking(&url, "material of that id"))?;
    let downloads = &asset["downloadFolders"]["default"]["downloadFiletypeCategories"]["zip"];
    let archive = (downloads["downloads"].as_array().into_iter().flatten())
        .find(|download| download["attribute"] == AMBIENTCG_ARCHIVE)
        .and_then(|download| text(download, "fullDownloadPath"))
        .ok_or_else(|| lacking(&url, "archive at 4K"))?;
    let title = text(asset, "displayName").ok_or_else(|| lacking(&url, "name"))?;
    Ok(library(
        (title, "ambientCG".to_owned()),
        text(asset, "shortLink").unwrap_or_else(|| format!("https://ambientcg.com/a/{id}")),
        "ambientCG",
        (archive, Full::Archive),
    ))
}

/// The Poly Haven texture `id` (`rusty_metal_02`): its title, authors, page and colour map.
///
/// # Errors
/// `Fetch` when the API cannot be reached or knows no such texture, or the texture has no colour
/// map at 4K.
pub fn polyhaven(fetcher: &Fetcher, id: &str) -> Result<Found, PackError> {
    let info_url = format!("https://api.polyhaven.com/info/{}", query_text(id));
    let info = fetcher.json(&cache_name("polyhaven", id, "json"), &info_url)?;
    let title = text(&info, "name").ok_or_else(|| lacking(&info_url, "texture of that id"))?;
    let authors: Vec<&str> = (info["authors"].as_object().into_iter().flatten())
        .map(|(author, _)| author.as_str())
        .collect();
    let files_url = format!("https://api.polyhaven.com/files/{}", query_text(id));
    let files = fetcher.json(
        &cache_name("polyhaven", &format!("{id} files"), "json"),
        &files_url,
    )?;
    let colour = text(&files["Diffuse"][POLYHAVEN_SIZE]["jpg"], "url")
        .ok_or_else(|| lacking(&files_url, "colour map at 4K"))?;
    let creator = if authors.is_empty() {
        "Poly Haven".to_owned()
    } else {
        authors.join(", ")
    };
    Ok(library(
        (title, creator),
        format!("https://polyhaven.com/a/{id}"),
        "Poly Haven",
        (colour, Full::Picture),
    ))
}

/// The Texture Ninja photograph `key` (`PaintFlaking_0327.jpg`): its page and file, titled by its
/// name.
///
/// # Errors
/// `Fetch` when the API cannot be reached or knows no such texture.
pub fn texture_ninja(fetcher: &Fetcher, key: &str) -> Result<Found, PackError> {
    let url = format!(
        "https://texture.ninja/api/textures?textureKey={}",
        query_text(key)
    );
    let answer = fetcher.json(&cache_name("textureninja", key, "json"), &url)?;
    (answer.as_array().into_iter().flatten())
        .find(|texture| texture["key"] == key)
        .ok_or_else(|| lacking(&url, "texture of that key"))?;
    let stem = key.rsplit_once('.').map_or(key, |(stem, _)| stem);
    let title = stem.replace(['_', '-'], " ").trim().to_owned();
    let mut found = library(
        (title, "Texture Ninja".to_owned()),
        format!("https://texture.ninja/search?texture={}", query_text(key)),
        "Texture Ninja",
        (format!("{TEXTURE_NINJA_FILES}{key}"), Full::Picture),
    );
    found.changes = "cropped";
    Ok(found)
}

/// A library's asset as found: CC0 by the library's own dedication (see the module).
fn library(
    (title, creator): (String, String),
    page: String,
    collection: &'static str,
    (picture, full): (String, Full),
) -> Found {
    Found {
        title,
        credit: creator.clone(),
        creator,
        page,
        stated: Stated::Licence(Licence::Cc0),
        picture,
        collection,
        changes: "its colour map, cropped",
        headers: &[],
        full,
    }
}

/// The file whose name ends `suffix` in the zip archive `archive`, stored without compression
/// (as ambientCG stores its maps); why not when the archive holds none or compresses it.
///
/// # Errors
/// What is wrong with the archive, in words.
pub fn stored_entry(archive: &[u8], suffix: &str) -> Result<Vec<u8>, String> {
    let read = |at: usize, width: usize| -> Option<usize> {
        let bytes = archive.get(at..at + width)?;
        Some(
            bytes
                .iter()
                .rev()
                .fold(0, |value, byte| value << 8 | usize::from(*byte)),
        )
    };
    // The end of central directory record: its signature within the archive's last 64 KiB.
    let end = (archive.len().saturating_sub(65_557)..archive.len().saturating_sub(21))
        .rev()
        .find(|at| read(*at, 4) == Some(0x0605_4b50))
        .ok_or("no zip archive")?;
    let (entries, mut at) = (
        read(end + 10, 2).ok_or("a damaged zip archive")?,
        read(end + 16, 4).ok_or("a damaged zip archive")?,
    );
    for _ in 0..entries {
        if read(at, 4) != Some(0x0201_4b50) {
            return Err("a damaged zip directory".to_owned());
        }
        let field = |offset: usize, width: usize| read(at + offset, width).unwrap_or(0);
        let (method, size, name_length) = (field(10, 2), field(20, 4), field(28, 2));
        let (extra, comment, local) = (field(30, 2), field(32, 2), field(42, 4));
        let name = archive
            .get(at + 46..at + 46 + name_length)
            .unwrap_or_default();
        if name.ends_with(suffix.as_bytes()) {
            if method != 0 {
                return Err(format!("{suffix} is compressed"));
            }
            let data = local
                + 30
                + read(local + 26, 2).ok_or("a damaged zip archive")?
                + read(local + 28, 2).ok_or("a damaged zip archive")?;
            return (archive.get(data..data + size).map(<[u8]>::to_vec))
                .ok_or_else(|| format!("{suffix} runs past the archive"));
        }
        at += 46 + name_length + extra + comment;
    }
    Err(format!("no file ending {suffix} in the archive"))
}

#[cfg(test)]
mod tests {
    use super::stored_entry;

    /// A zip archive of `files`, each stored, as a zip writer lays it out.
    fn archive(files: &[(&str, &[u8])]) -> Vec<u8> {
        let (mut out, mut directory) = (Vec::new(), Vec::new());
        let le = |value: usize, width: usize| (0..width).map(move |k| (value >> (8 * k)) as u8);
        for (name, data) in files {
            let at = out.len();
            out.extend(
                le(0x0403_4b50, 4)
                    .chain(le(10, 2))
                    .chain(le(0, 2))
                    .chain(le(0, 2)),
            );
            out.extend(le(0, 8).chain(le(data.len(), 4)).chain(le(data.len(), 4)));
            out.extend(le(name.len(), 2).chain(le(0, 2)));
            out.extend(name.bytes().chain(data.iter().copied()));
            directory.extend(le(0x0201_4b50, 4).chain(le(20, 2)).chain(le(10, 2)));
            directory.extend(le(0, 4).chain(le(0, 8)).chain(le(data.len(), 4)));
            directory.extend(le(data.len(), 4).chain(le(name.len(), 2)).chain(le(0, 8)));
            directory.extend(le(0, 4).chain(le(at, 4)).chain(name.bytes()));
        }
        let start = out.len();
        out.extend(&directory);
        out.extend(le(0x0605_4b50, 4).chain(le(0, 4)).chain(le(files.len(), 2)));
        out.extend(
            le(files.len(), 2)
                .chain(le(directory.len(), 4))
                .chain(le(start, 4)),
        );
        out.extend(le(0, 2));
        out
    }

    #[test]
    fn the_colour_map_is_taken_out_of_its_archive_and_nothing_else() {
        let zip = archive(&[
            ("Rust001_4K-JPG_NormalGL.jpg", &b"normal"[..]),
            ("Rust001_4K-JPG_Color.jpg", &b"colour"[..]),
            ("Rust001_4K-JPG_Roughness.jpg", &b"rough"[..]),
        ]);
        assert_eq!(stored_entry(&zip, "_Color.jpg"), Ok(b"colour".to_vec()));
        assert!(stored_entry(&zip, "_Albedo.jpg").is_err());
        assert!(stored_entry(b"not a zip at all, only words", "_Color.jpg").is_err());
    }
}
