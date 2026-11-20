//! Imported practice packs, kept beside the library (ROADMAP S78): one file a pack,
//! `<dir>/<pack id>.pack`, holding the pack's bytes as imported (`pack.rs`), checked whole first.
//! The library's records are never touched: a pack is no artwork, and the library's format
//! stays as it is. Importing a pack whose id is kept replaces it whole (a newer edition); the
//! personal pack is never imported. The user's own pictures are added one at a time to the
//! personal pack (`PERSONAL`), written whole again with the new one at its end, and removed or
//! moved to another tier the same way (S83; its last picture removed, the file goes). A file is
//! written under a temporary name and renamed over the old one, so a reader finds the old pack or
//! the new one, never part of one.

use std::fs::{self, File};
use std::io::{ErrorKind, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use openpaint_app::PortError;
use openpaint_practice::{ItemRef, Outline, PERSONAL, Pack, PackItem, PictureKind, Tier, valid_id};

use crate::pack::{Placed, START, damaged, decode_head, encode_pack, head_length, open_pack};
use crate::pack_picture::PackPicture;

/// The personal pack's title, the start of its items' ids and of their titles, numbered from 1.
const PERSONAL_TITLE: &str = "My pictures";
const PERSONAL_ITEM: &str = "mine-";
const PERSONAL_TITLE_ITEM: &str = "Picture ";

/// The imported packs in one directory (the app's files directory's `packs`, beside its
/// `library`).
#[derive(Debug, Clone)]
pub struct Packs {
    dir: PathBuf,
}

fn device(error: &std::io::Error) -> PortError {
    PortError::Device {
        detail: error.to_string(),
    }
}

impl Packs {
    #[must_use]
    pub fn at(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// The file of the pack `id` (a valid id is a file name).
    #[must_use]
    pub fn path(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.pack"))
    }

    /// Checks the pack file `bytes` whole and keeps it, replacing a kept pack of its id. Returns
    /// its manifest.
    ///
    /// # Errors
    /// As `open_pack` (nothing is written then: a pack of another format is `OtherFormat`);
    /// `Device` when the file cannot be written (nothing is replaced then).
    pub fn import(&self, bytes: &[u8]) -> Result<Pack, PortError> {
        let (pack, _) = open_pack(bytes)?;
        if pack.personal() {
            log::warn!("a personal pack is never imported");
            return Err(damaged());
        }
        self.keep(&pack.id, bytes)?;
        log::info!(
            "practice pack {} imported: {} items, {} bytes",
            pack.id,
            pack.items.len(),
            bytes.len()
        );
        Ok(pack)
    }

    /// Adds `picture` (as `pack_picture` made it) with its `outline` at `tier` to the personal
    /// pack, made with it when there is none. Its id comes from its bytes (`mine-` and their
    /// CRC-32), so a removed picture's id names no other picture later: a lesson or copy that
    /// used it finds it only if the very picture comes back. Returns the personal pack as it is
    /// now, the picture its last item.
    ///
    /// # Errors
    /// `Device` when the pack cannot be read or written (nothing is replaced then), `Corrupt`
    /// when the kept personal pack is damaged.
    pub fn add_picture(
        &self,
        picture: &PackPicture,
        (outline, tier): (Option<Outline>, Tier),
    ) -> Result<Pack, PortError> {
        let (mut pack, mut pictures) = self.personal()?;
        let mut id = format!("{PERSONAL_ITEM}{:08x}", crc32fast::hash(&picture.file));
        for again in 2.. {
            if pack.item(&id).is_none() {
                break;
            }
            id = format!(
                "{PERSONAL_ITEM}{:08x}-{again}",
                crc32fast::hash(&picture.file)
            );
        }
        let number = 1
            + (pack.items.iter())
                .filter_map(|item| {
                    item.title
                        .strip_prefix(PERSONAL_TITLE_ITEM)?
                        .parse::<u32>()
                        .ok()
                })
                .max()
                .unwrap_or(0);
        let item = PackItem {
            id,
            kind: PictureKind::Shape,
            tier,
            size: picture.size,
            title: format!("{PERSONAL_TITLE_ITEM}{number}"),
            credit: None,
            outline,
            masses: None,
            areas: None,
        };
        log::info!(
            "picture {} added to the personal pack ({} items) at {}, outline {}",
            item.id,
            pack.items.len() + 1,
            tier.name(),
            item.outline.is_some()
        );
        pack.items.push(item);
        pictures.push(picture.file.clone());
        let bytes = encode_pack(&pack, &pictures)?;
        self.keep(PERSONAL, &bytes)?;
        Ok(pack)
    }

    /// The user's own picture `item` removed from the personal pack, which is written whole
    /// again, or removed with its last picture. Returns the personal pack as it is now (none:
    /// no picture left); a picture it does not hold changes nothing.
    ///
    /// # Errors
    /// As `add_picture`.
    pub fn remove_picture(&self, item: &str) -> Result<Option<Pack>, PortError> {
        let (mut pack, mut pictures) = self.personal()?;
        let Some(at) = pack.items.iter().position(|kept| kept.id == item) else {
            return Ok((!pack.items.is_empty()).then_some(pack));
        };
        pack.items.remove(at);
        pictures.remove(at);
        log::info!(
            "picture {item} removed from the personal pack ({} left)",
            pack.items.len()
        );
        if pack.items.is_empty() {
            return match fs::remove_file(self.path(PERSONAL)) {
                Ok(()) => Ok(None),
                Err(error) => Err(device(&error)),
            };
        }
        let bytes = encode_pack(&pack, &pictures)?;
        self.keep(PERSONAL, &bytes)?;
        Ok(Some(pack))
    }

    /// The user's own picture `item` moved to `tier`. Returns the personal pack as it is now
    /// (none: no picture at all); a picture it does not hold changes nothing.
    ///
    /// # Errors
    /// As `add_picture`.
    pub fn set_tier(&self, item: &str, tier: Tier) -> Result<Option<Pack>, PortError> {
        let (mut pack, pictures) = self.personal()?;
        let Some(kept) = pack.items.iter_mut().find(|kept| kept.id == item) else {
            return Ok((!pack.items.is_empty()).then_some(pack));
        };
        kept.tier = tier;
        let bytes = encode_pack(&pack, &pictures)?;
        self.keep(PERSONAL, &bytes)?;
        Ok(Some(pack))
    }

    /// The personal pack kept and its pictures' files; empty when there is none.
    fn personal(&self) -> Result<(Pack, Vec<Vec<u8>>), PortError> {
        let bytes = match fs::read(self.path(PERSONAL)) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                return Ok((personal_pack(), Vec::new()));
            }
            Err(error) => return Err(device(&error)),
        };
        let (pack, pictures) = open_pack(&bytes)?;
        let pictures = pictures.into_iter().map(<[u8]>::to_vec).collect();
        Ok((pack, pictures))
    }

    /// Writes `bytes` as the pack `id`'s file, replacing any kept one whole.
    fn keep(&self, id: &str, bytes: &[u8]) -> Result<(), PortError> {
        fs::create_dir_all(&self.dir).map_err(|error| device(&error))?;
        let path = self.path(id);
        let written = path.with_extension("pack.part");
        fs::write(&written, bytes).map_err(|error| device(&error))?;
        if let Err(error) = fs::rename(&written, &path) {
            if let Err(left) = fs::remove_file(&written) {
                log::warn!("{} stays: {left}", written.display());
            }
            return Err(device(&error));
        }
        Ok(())
    }

    /// Every kept pack's manifest, by id; a pack that cannot be read, one of another format among
    /// them, is left out as it is (logged).
    ///
    /// # Errors
    /// `Device` when the directory cannot be listed.
    pub fn list(&self) -> Result<Vec<Pack>, PortError> {
        let entries = match fs::read_dir(&self.dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(device(&error)),
        };
        let mut packs = Vec::new();
        for entry in entries {
            let path = entry.map_err(|error| device(&error))?.path();
            let Some(id) = pack_id(&path) else {
                continue;
            };
            match read_head(&path, id) {
                Ok((pack, _)) => packs.push(pack),
                Err(error) => log::warn!("{} is not read: {error}", path.display()),
            }
        }
        packs.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(packs)
    }

    /// The picture (a PNG or JPEG file's bytes) of the item `item` names; `None` when its pack is
    /// not kept or holds no such item.
    ///
    /// # Errors
    /// `Device` when the file cannot be read, `Corrupt` when the pack or the picture is damaged.
    pub fn picture(&self, item: &ItemRef) -> Result<Option<Vec<u8>>, PortError> {
        if !valid_id(&item.pack) {
            return Ok(None);
        }
        let path = self.path(&item.pack);
        if !path.exists() {
            return Ok(None);
        }
        let (pack, placed) = read_head(&path, &item.pack)?;
        let Some(index) = pack.items.iter().position(|i| i.id == item.item) else {
            return Ok(None);
        };
        read_picture(&path, placed[index]).map(Some)
    }
}

/// The personal pack before its first picture.
fn personal_pack() -> Pack {
    Pack {
        id: PERSONAL.to_owned(),
        title: PERSONAL_TITLE.to_owned(),
        items: Vec::new(),
    }
}

/// The pack id a kept file's name gives; `None` for any other file (a `.part` left by a write
/// that failed).
fn pack_id(path: &Path) -> Option<&str> {
    let name = path.file_name()?.to_str()?;
    name.strip_suffix(".pack").filter(|id| valid_id(id))
}

/// The manifest of the pack file at `path` and where its pictures lie, its id checked to be `id`.
fn read_head(path: &Path, id: &str) -> Result<(Pack, Vec<Placed>), PortError> {
    let mut file = File::open(path).map_err(|error| device(&error))?;
    let mut start = [0; START];
    file.read_exact(&mut start).map_err(|error| short(&error))?;
    let mut head = vec![0; head_length(&start)? + 4];
    file.read_exact(&mut head).map_err(|error| short(&error))?;
    let (pack, placed) = decode_head(&head)?;
    if pack.id != id {
        return Err(damaged());
    }
    Ok((pack, placed))
}

/// The picture lying at `place` in the pack file at `path`, its sum checked.
fn read_picture(path: &Path, place: Placed) -> Result<Vec<u8>, PortError> {
    let mut file = File::open(path).map_err(|error| device(&error))?;
    file.seek(SeekFrom::Start(place.offset))
        .map_err(|error| device(&error))?;
    let mut picture = vec![0; place.length as usize];
    file.read_exact(&mut picture)
        .map_err(|error| short(&error))?;
    if crc32fast::hash(&picture) != place.check {
        return Err(damaged());
    }
    Ok(picture)
}

/// A read that ended early is a cut file; any other failure is the device's.
fn short(error: &std::io::Error) -> PortError {
    if error.kind() == ErrorKind::UnexpectedEof {
        damaged()
    } else {
        device(error)
    }
}
