//! `library-hashes <library dir>`: the proof of S163's format change (D125), deleted with the
//! conversion step by the slice after it. Opens a copy of the library in `dir`
//! (`launch::library_copy`; opening writes to the copy, and a format-2 build converts a format-1
//! copy at its first opening, never `dir`) and prints one line per artwork and per kept version:
//! the project, the version (`-` for the artwork as stored), its layers, strokes and edits (every
//! list: paint, masks, merged layers' records) and the FNV-1a hash of its 1x export's pixels on
//! white paper, sorted. Run by the build of each format on its own fresh copy of the same backup's
//! library: the two outputs must be equal line for line.

use openpaint_app::{ProjectId, ProjectStore, VersionId};
use openpaint_core::{Artwork, Color, LayerMask, RegionEdit, StrokeList};
use openpaint_render::{Banding, Core};
use openpaint_store::Library;

use crate::error::HarnessError;
use crate::launch::library_copy;

pub fn run(dir: &str) -> Result<(), HarnessError> {
    let mut library = Library::at(library_copy(dir, "library-hashes")?);
    let core = Core::headless()?;
    let listed = library.list().map_err(HarnessError::Port)?;
    let mut lines = Vec::new();
    for summary in &listed {
        let artwork = library.artwork(summary.id).map_err(HarnessError::Port)?;
        lines.push(line(&core, (summary.id, None), &artwork)?);
        for kept in &summary.versions {
            let copy = library
                .restore_version(summary.id, kept.id, &summary.title)
                .map_err(HarnessError::Port)?;
            let artwork = library.artwork(copy).map_err(HarnessError::Port)?;
            lines.push(line(&core, (summary.id, Some(kept.id)), &artwork)?);
        }
    }
    lines.sort();
    for line in &lines {
        println!("{line}");
    }
    println!("{} artworks and kept versions", lines.len());
    Ok(())
}

/// "<project> <version or -> <w>x<h> layers <n> strokes <n> edits <n> pixels <hash>".
fn line(
    core: &Core,
    (project, version): (ProjectId, Option<VersionId>),
    artwork: &Artwork,
) -> Result<String, HarnessError> {
    let version = version.map_or_else(|| "-".to_owned(), |version| version.get().to_string());
    let canvas = artwork.canvas();
    let [strokes, edits] = artwork.layers().iter().fold([0, 0], |[s, e], layer| {
        let [ls, le] = counted((layer.strokes(), layer.edits()), layer.mask());
        [s + ls, e + le]
    });
    let mut hash = Fnv::default();
    for band in &mut core.export_bands(artwork, Color::WHITE, 1, Banding::DEFAULT)? {
        hash.add(band?.pixels());
    }
    Ok(format!(
        "{:>6} {version:>3} {}x{} layers {} strokes {strokes} edits {edits} pixels {:016x}",
        project.get(),
        canvas.width(),
        canvas.height(),
        artwork.layers().len(),
        hash.0
    ))
}

/// The strokes and edits of a list, its mask's and its merged records' at any depth.
fn counted((strokes, edits): (&StrokeList, &[RegionEdit]), mask: Option<&LayerMask>) -> [usize; 2] {
    let mut count = [strokes.len(), edits.len()];
    if let Some(mask) = mask {
        let [s, e] = counted((mask.strokes(), mask.edits()), None);
        count = [count[0] + s, count[1] + e];
    }
    for nested in edits.iter().filter_map(RegionEdit::nested) {
        let layer = nested.layer();
        let [s, e] = counted((layer.strokes(), layer.edits()), layer.mask());
        count = [count[0] + s, count[1] + e];
    }
    count
}

/// FNV-1a over bytes, 64 bits.
struct Fnv(u64);

impl Default for Fnv {
    fn default() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }
}

impl Fnv {
    fn add(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 = (self.0 ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
}
