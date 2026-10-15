//! `test-art <pile|a4|session> [n] [out]` (S58): test artworks made on the development machine as
//! OpenPaint files (DECISIONS D62), so a device slice pushes one (`cargo device-push`) and opens
//! it (`cargo device-input open-file`) instead of drawing it by injection, which took minutes.
//!
//! - `pile [n]` (300): `n` strokes of the ten built-ins in turn at their first sizes, each along
//!   the path S48's injected pile had (`pile_path`: 52 samples 8 ms apart, pressed 0.8), turned
//!   and moved about the middle of a screen-size canvas: one dense place.
//! - `a4 [n]` (180): an A4 page of `n` strokes of the author's brushes over the whole page, dense
//!   around one place, on five layers: fills, a picture inserted, 9 smudges, a mask, a Multiply
//!   layer (`test_art_kinds.rs`), like the author's busiest artwork.
//! - `session [minutes]` (60): an hour's painting at the author's pace as records: a stroke every
//!   1.2 s of 342 samples 3.3 ms apart, the built-ins in turn ten strokes each, every tenth a
//!   smudge, a layer more every 500 strokes. Undo and redo are not records: the device's
//!   `bench session` undoes every twentieth stroke as it appends them and redoes every other one.
//!
//! Deterministic: every place, pressure, size, colour and seed comes from SplitMix64 of its index,
//! and the file is written through a library of its own, whose new artwork's head holds no save id
//! and which keeps no gallery picture, so one command makes one file byte for byte. The default
//! file is `artifacts/test-art/<kind>-<n>.openpaint`, a name `device-push` takes.

use std::fs;
use std::path::PathBuf;

use openpaint_app::{BrushLibrary, ProjectStore, Title};
use openpaint_core::{Artwork, CanvasPoint, CanvasSize, StrokeSample, StrokeStyle};
use openpaint_runtime::{drawn, pile_path, style_of};
use openpaint_store::Library;

use crate::error::HarnessError;
use crate::output::workspace_root;

/// The tablet's screen-size canvas: the area under the top band at fit (STATUS, export sizes).
pub const SCREEN: [u32; 2] = [2880, 1716];
/// A4 at 300 dpi, the author's busiest artwork's size.
pub const A4: [u32; 2] = [3508, 2480];
/// The built-ins in picker order.
pub const BRUSHES: [&str; 10] = [
    "pencil",
    "ink-pen",
    "inker",
    "monoline",
    "hard-round",
    "soft-brush",
    "textured-round",
    "flat",
    "opaque-paint",
    "speckle",
];

const USAGE: &str = "test-art <pile|a4|session> [n] [out.openpaint]";

/// What `test-art` makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Pile(u32),
    A4(u32),
    /// Minutes of painting.
    Session(u32),
}

impl Kind {
    fn parse(kind: &str, count: Option<&str>) -> Result<Self, HarnessError> {
        let count = match count {
            Some(count) => Some(count.parse().map_err(|_| HarnessError::Missing(USAGE))?),
            None => None,
        };
        match kind {
            "pile" => Ok(Self::Pile(count.unwrap_or(300))),
            "a4" => Ok(Self::A4(count.unwrap_or(180))),
            "session" => Ok(Self::Session(count.unwrap_or(60))),
            _ => Err(HarnessError::Missing(USAGE)),
        }
    }

    /// The file's name and the artwork's title.
    fn names(self) -> (String, String) {
        let (kind, n) = match self {
            Self::Pile(n) => ("pile", n),
            Self::A4(n) => ("a4", n),
            Self::Session(n) => ("session", n),
        };
        (format!("{kind}-{n}"), format!("Test art {kind} {n}"))
    }
}

pub fn run(args: &[&str]) -> Result<(), HarnessError> {
    let (kind, count, out) = match args {
        [kind] => (*kind, None, None),
        [kind, count] => (*kind, Some(*count), None),
        [kind, count, out] => (*kind, Some(*count), Some(PathBuf::from(out))),
        _ => return Err(HarnessError::Missing(USAGE)),
    };
    let kind = Kind::parse(kind, count)?;
    let (name, title) = kind.names();
    let out = out.unwrap_or_else(|| {
        workspace_root()
            .join("artifacts")
            .join("test-art")
            .join(format!("{name}.openpaint"))
    });
    let artwork = make(kind)?;
    let bytes = file_bytes(&artwork, &title)?;
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&out, &bytes)?;
    let canvas = artwork.canvas();
    let strokes: usize = (artwork.layers().iter())
        .map(|layer| layer.strokes().len() + layer.mask().map_or(0, |mask| mask.strokes().len()))
        .sum();
    println!(
        "{} \"{title}\": {} x {}, {} layers, {strokes} strokes, {} bytes",
        out.display(),
        canvas.width(),
        canvas.height(),
        artwork.layers().len(),
        bytes.len()
    );
    Ok(())
}

/// The artwork `kind` names.
///
/// # Errors
/// As the artwork refuses an edit (a bug in the generator).
pub fn make(kind: Kind) -> Result<Artwork, HarnessError> {
    let library = BrushLibrary::built_in()
        .map_err(|_| HarnessError::Missing("the built-in brushes did not load"))?;
    match kind {
        Kind::Pile(n) => pile(&library, n),
        Kind::A4(n) => crate::test_art_kinds::a4(&library, n),
        Kind::Session(minutes) => crate::test_art_kinds::session(&library, minutes),
    }
}

/// `n` strokes of the built-ins in turn along S48's injected paths about the canvas's middle.
fn pile(library: &BrushLibrary, n: u32) -> Result<Artwork, HarnessError> {
    let mut artwork = Artwork::new(CanvasSize::new(SCREEN[0], SCREEN[1])?);
    let middle = SCREEN.map(|side| side as f32 / 2.0);
    for k in 0..n {
        let key = BRUSHES[k as usize % BRUSHES.len()];
        let style = style_of(library, key, None)?.with_seed(k);
        artwork = drawn(&artwork, style, &pile_path(k, middle), |_| 0.8)?;
    }
    Ok(artwork)
}

/// The bytes of `artwork` as an OpenPaint file titled `title`: stored in a library of its own
/// (emptied first, removed after) and written out by the store, as Save as OpenPaint file does.
///
/// # Errors
/// The library's, and the file system's.
pub fn file_bytes(artwork: &Artwork, title: &str) -> Result<Vec<u8>, HarnessError> {
    let dir = scratch("test-art-library");
    let title = Title::new(title).map_err(|_| HarnessError::Missing("not a title"))?;
    let bytes = {
        let mut library = Library::at(&dir);
        let id = library
            .create(artwork, &title)
            .map_err(HarnessError::Port)?;
        library.artwork_file(id).map_err(HarnessError::Port)?
    };
    fs::remove_dir_all(&dir)?;
    Ok(bytes)
}

/// A directory of this process under `target/tmp`, named `name` and a count, that does not exist.
pub fn scratch(name: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU32, Ordering};
    static MADE: AtomicU32 = AtomicU32::new(0);
    let count = MADE.fetch_add(1, Ordering::Relaxed);
    let dir = workspace_root()
        .join("target")
        .join("tmp")
        .join(format!("{name}-{}-{count}", std::process::id()));
    if dir.exists() {
        let _ = fs::remove_dir_all(&dir);
    }
    dir
}

/// A number from 0 to 1 drawn from `k` and `salt` (SplitMix64): the same on every run.
#[must_use]
pub fn unit(k: u64, salt: u64) -> f32 {
    let mut z = k
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(salt.wrapping_mul(0xD1B5_4A32_D192_ED03));
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 40) as f32 / (1u64 << 24) as f32
}

/// A hand's stroke: `count` samples `spacing` canvas pixels apart from `start` heading `angle`
/// (radians), turning `turn` a sample, kept inside `bounds` (left, top, right, bottom), pressure
/// rising and falling, the pen leaning 40 degrees, 3.3 ms apart.
#[derive(Debug, Clone, Copy)]
pub struct Hand {
    pub start: [f32; 2],
    pub angle: f32,
    pub turn: f32,
    pub count: u32,
    pub spacing: f32,
    pub bounds: [f32; 4],
}

impl Hand {
    /// The stroke's samples.
    ///
    /// # Errors
    /// `InvalidValue` for a sample the core refuses (none here).
    pub fn samples(&self) -> Result<Vec<StrokeSample>, HarnessError> {
        let [left, top, right, bottom] = self.bounds;
        let (mut at, mut angle) = (self.start, self.angle);
        let tilt = 40f32.to_radians();
        (0..self.count)
            .map(|i| {
                let t = i as f32 / (self.count.max(2) - 1) as f32;
                if i > 0 {
                    angle += self.turn;
                    at = [
                        (at[0] + self.spacing * angle.cos()).clamp(left, right),
                        (at[1] + self.spacing * angle.sin()).clamp(top, bottom),
                    ];
                }
                let pressure = 0.3 + 0.6 * (std::f32::consts::PI * t).sin();
                let position = CanvasPoint::new(at[0], at[1])?;
                Ok(StrokeSample::new(
                    position,
                    pressure,
                    tilt,
                    0.0,
                    i * 10 / 3,
                )?)
            })
            .collect()
    }
}

/// `artwork` with a stroke of `style` through `samples` on its target, committed.
///
/// # Errors
/// As the artwork refuses the stroke.
pub fn stroke(
    artwork: &Artwork,
    style: StrokeStyle,
    samples: &[StrokeSample],
) -> Result<Artwork, HarnessError> {
    let mut session = artwork.begin_stroke(style, samples[0])?;
    session.extend(&samples[1..])?;
    Ok(session.finish().artwork)
}

#[cfg(test)]
mod tests {
    use super::*;
    use openpaint_app::ProjectStore;

    #[test]
    fn the_same_command_makes_the_same_file_and_it_opens() {
        for kind in [Kind::Pile(12), Kind::A4(20), Kind::Session(1)] {
            let artwork = make(kind).expect("made");
            let first = file_bytes(&artwork, "Test").expect("written");
            let again = file_bytes(&make(kind).expect("made again"), "Test").expect("again");
            assert!(first == again, "{kind:?}: two runs, two files");
            let dir = scratch("test-art-open");
            let mut library = Library::at(&dir);
            let id = library.import_file(&first).expect("the file opens");
            let opened = library.load(id).expect("loaded").artwork;
            let count = |artwork: &Artwork| -> usize {
                (artwork.layers().iter())
                    .map(|l| l.strokes().len() + l.mask().map_or(0, |m| m.strokes().len()))
                    .sum()
            };
            assert_eq!(count(&opened), count(&artwork), "{kind:?}");
            assert_eq!(opened.layers().len(), artwork.layers().len(), "{kind:?}");
            drop(library);
            fs::remove_dir_all(&dir).expect("removed");
        }
    }
}
