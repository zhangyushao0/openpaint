//! The display cache (S54, DECISIONS D81): for each artwork one file holding the pixels its
//! canvas showed complete, exactly as the screen's canvas target held them (ten-bit codes on the
//! tablet, 8-bit values elsewhere), so opening it shows the artwork at once. Derived data, never
//! the truth: the records are, and nothing but the opening canvas reads it.
//!
//! A file `<dir>/<project>.bin` is a header of the magic `OPDISP01`, the save id, the renderer's
//! identity (u64 each), the format code, the width and the height (u32 each), the CRC-32 of the
//! body (u32) and the body's length (u64), little-endian, then the body: the pixels, 4 bytes
//! each, rows top first, as one LZ4 block (lz4_flex). It is written whole under a temporary name,
//! then renamed over the old one, so a reader finds the old file or the new one, never part of
//! one; a damaged one fails its check sum.

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use openpaint_app::{PortError, ProjectId, SaveId};

use crate::codec::{Reader, Writer};

const MAGIC: &[u8; 8] = b"OPDISP01";
/// Header bytes after the magic.
const HEADER: usize = 8 + 8 + 4 + 4 + 4 + 4 + 8;
/// The longest side a cached canvas may have: a header naming more is damage.
const MAX_SIDE: u32 = 16_384;

/// What a cached canvas must match to be shown complete: the save that wrote the artwork's head,
/// the renderer that evaluated it, the canvas target's format and its size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheKey {
    pub saved: SaveId,
    pub renderer: u64,
    pub format: u32,
    pub size: [u32; 2],
}

/// A cached canvas: its key and its pixels, 4 bytes each, rows top first.
pub struct Cached {
    pub key: CacheKey,
    pub pixels: Vec<u8>,
}

impl std::fmt::Debug for Cached {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Cached({:?}, {} bytes)", self.key, self.pixels.len())
    }
}

/// The display cache in one directory (the app's cache directory's `canvas`).
#[derive(Debug, Clone)]
pub struct DisplayCache {
    dir: PathBuf,
}

fn device(error: &std::io::Error) -> PortError {
    PortError::Device {
        detail: error.to_string(),
    }
}

const fn damaged() -> PortError {
    PortError::Corrupt {
        what: "display cache",
    }
}

impl DisplayCache {
    #[must_use]
    pub fn at(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// The file of `project`'s canvas.
    #[must_use]
    pub fn path(&self, project: ProjectId) -> PathBuf {
        self.dir.join(format!("{}.bin", project.get()))
    }

    /// `project`'s cached canvas; `None` when there is none.
    ///
    /// # Errors
    /// `Device` when the file cannot be read, `Corrupt` when it is not a whole cache file.
    pub fn read(&self, project: ProjectId) -> Result<Option<Cached>, PortError> {
        match fs::read(self.path(project)) {
            Ok(bytes) => decode(&bytes).map(Some),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
            Err(error) => Err(device(&error)),
        }
    }

    /// Writes `pixels` (4 bytes each at `key.size`) as `project`'s cached canvas, replacing the
    /// one there whole. Returns the file's length in bytes.
    ///
    /// # Errors
    /// `Device` when the directory or the file cannot be written (nothing is replaced then),
    /// `Corrupt` when `pixels` do not fill the size.
    pub fn write(
        &self,
        project: ProjectId,
        key: &CacheKey,
        pixels: &[u8],
    ) -> Result<u64, PortError> {
        if Some(pixels.len()) != length(key.size) {
            return Err(damaged());
        }
        let bytes = encode(key, pixels);
        fs::create_dir_all(&self.dir).map_err(|error| device(&error))?;
        let path = self.path(project);
        let written = path.with_extension("bin.part");
        fs::write(&written, &bytes).map_err(|error| device(&error))?;
        if let Err(error) = fs::rename(&written, &path) {
            remove(&written);
            return Err(device(&error));
        }
        Ok(bytes.len() as u64)
    }

    /// Removes `project`'s cached canvas (its file, by its exact path), if there is one.
    ///
    /// # Errors
    /// `Device` when it is there and cannot be removed.
    pub fn delete(&self, project: ProjectId) -> Result<(), PortError> {
        match fs::remove_file(self.path(project)) {
            Err(error) if error.kind() != ErrorKind::NotFound => Err(device(&error)),
            _ => Ok(()),
        }
    }
}

/// Removes a temporary file left by a failed write; a failure is only logged.
fn remove(path: &Path) {
    if let Err(error) = fs::remove_file(path) {
        log::warn!("{} stays: {error}", path.display());
    }
}

/// Bytes of the pixels of a canvas of `size`, if that is one a cache may hold.
fn length([width, height]: [u32; 2]) -> Option<usize> {
    let side = 1..=MAX_SIDE;
    (side.contains(&width) && side.contains(&height)).then(|| width as usize * height as usize * 4)
}

fn encode(key: &CacheKey, pixels: &[u8]) -> Vec<u8> {
    let body = lz4_flex::compress(pixels);
    let mut writer = Writer::with_capacity(MAGIC.len() + HEADER + body.len());
    writer.bytes(MAGIC);
    writer.u64(key.saved.get());
    writer.u64(key.renderer);
    writer.u32(key.format);
    writer.u32(key.size[0]);
    writer.u32(key.size[1]);
    writer.u32(crc32fast::hash(&body));
    writer.u64(body.len() as u64);
    writer.bytes(&body);
    writer.finish()
}

fn decode(bytes: &[u8]) -> Result<Cached, PortError> {
    let rest = bytes.strip_prefix(MAGIC).ok_or_else(damaged)?;
    let (header, body) = rest.split_at_checked(HEADER).ok_or_else(damaged)?;
    let mut reader = Reader::new(header, "display cache");
    let saved = SaveId::new(reader.u64()?).ok_or_else(damaged)?;
    let renderer = reader.u64()?;
    let format = reader.u32()?;
    let size = [reader.u32()?, reader.u32()?];
    let (check, length_of_body) = (reader.u32()?, reader.u64()?);
    reader.end()?;
    if length_of_body != body.len() as u64 || crc32fast::hash(body) != check {
        return Err(damaged());
    }
    let mut pixels = vec![0; length(size).ok_or_else(damaged)?];
    let unpacked = lz4_flex::decompress_into(body, &mut pixels).map_err(|_| damaged())?;
    if unpacked != pixels.len() {
        return Err(damaged());
    }
    let key = CacheKey {
        saved,
        renderer,
        format,
        size,
    };
    Ok(Cached { key, pixels })
}
