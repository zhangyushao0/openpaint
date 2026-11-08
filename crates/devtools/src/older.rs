//! A scenario's library put back as 1.16.0 kept it (`Step::RestartOnOldSheets`, S99), for the
//! store's upgrade to lay it anew at the next start: up to 1.16.0 every pair's frames were square
//! and a picture lay in its copy's square at 90 % of it, centred. Each stroke begun in the frame
//! of a copy of a picture the packs hold is moved back to where it lay on the picture in that
//! square, its clip that square (as a shape's, a division's or a whole copy's was); then each
//! practice record loses the layout byte 1.17.0 ends it with. Fills stay (a shape's copy has
//! none).

use std::path::Path;

use fjall::{Database, KeyspaceCreateOptions, PersistMode};
use openpaint_app::{PortError, ProjectId, ProjectStore};
use openpaint_core::{Artwork, MaskParts, RegionMask};
use openpaint_practice::{Frame, Pack, SessionRecord, Sheet};
use openpaint_practice_store::{PRACTICE, Packs, library_at, sessions};

use crate::error::ScenarioError;

/// Where a picture lay in its square frame up to 1.16.0: at this share of the frame, centred.
const OLD_SHOWN: f64 = 0.9;

/// Puts the closed library in `library` back as 1.16.0 kept it, its pictures as the packs in
/// `packs` hold them.
pub(crate) fn as_before(library: &Path, packs: &Path) -> Result<(), ScenarioError> {
    let failed = |_: PortError| ScenarioError::Missing("the library could not be put back");
    let held = Packs::at(packs).list().map_err(failed)?;
    let mut store = library_at(library, Some(Packs::at(packs)));
    for record in sessions(&mut store).map_err(failed)? {
        let backs = backs(&record, &held);
        if backs.is_empty() || record.canvas == 0 {
            continue;
        }
        let id = ProjectId::new(record.canvas);
        let artwork = store.artwork(id).map_err(failed)?;
        let moved = moved_back(&artwork, &backs).map_err(failed)?;
        store.save(id, &moved).map_err(failed)?;
    }
    drop(store);
    unlaid(library).map_err(|_| ScenarioError::Missing("the records could not be put back"))
}

/// A copy of a picture: its frame now, its square up to 1.16.0, and a point of the frame's place
/// in the square.
struct Back {
    frame: Frame,
    square: Frame,
    from: [f64; 2],
    factor: f64,
}

impl Back {
    fn at(&self, q: [f64; 2]) -> [f64; 2] {
        [0, 1].map(|axis| self.from[axis] + (q[axis] - self.frame.origin[axis]) / self.factor)
    }
}

/// The copies of pictures on `record`'s sheet that `packs` hold.
fn backs(record: &SessionRecord, packs: &[Pack]) -> Vec<Back> {
    if record.is_project() {
        return Vec::new();
    }
    let squares = Sheet::of(&vec![None; record.exercises.len()]);
    let laid = Sheet::of_session(record, packs);
    let mut backs = Vec::new();
    for (index, exercise) in record.exercises.iter().enumerate() {
        let item = exercise.picture.as_ref();
        let Some(found) = item.and_then(|item| packs.iter().find_map(|pack| pack.find(item)))
        else {
            continue;
        };
        let [width, height] = found.size.map(|side| f64::from(side.max(1)));
        let scale = OLD_SHOWN / width.max(height);
        let shown = [width * scale, height * scale];
        let (square, frame) = (squares.pairs[index].copy, laid.pairs[index].copy);
        backs.push(Back {
            frame,
            square,
            from: [0, 1]
                .map(|axis| square.origin[axis] + (1.0 - shown[axis]) / 2.0 * square.size[axis]),
            factor: frame.size[0] / (shown[0] * square.size[0]),
        });
    }
    backs
}

/// `artwork` with each stroke begun in one of `backs`' frames moved back into its square.
fn moved_back(artwork: &Artwork, backs: &[Back]) -> Result<Artwork, PortError> {
    let canvas = artwork.canvas();
    let mut parts = artwork.to_parts();
    for stroke in parts
        .layers
        .iter_mut()
        .flat_map(|layer| layer.strokes.iter_mut())
    {
        let Some(first) = stroke.samples.first() else {
            continue;
        };
        let begun = [f64::from(first.x), f64::from(first.y)];
        let Some(back) = backs.iter().find(|back| back.frame.holds(begun)) else {
            continue;
        };
        for sample in &mut stroke.samples {
            let [x, y] = back.at([f64::from(sample.x), f64::from(sample.y)]);
            (sample.x, sample.y) = (x as f32, y as f32);
        }
        let corners = back.square.corners().map(|p| p.map(|v| v as f32));
        let square: Option<MaskParts> =
            RegionMask::polygon(canvas, &corners).map(|mask| mask.to_parts());
        stroke.clip = stroke.clip.take().and(square);
    }
    Ok(Artwork::from_storage(parts)?)
}

/// Every practice record of the closed library in `dir` without its last byte, its layout.
fn unlaid(dir: &Path) -> Result<(), fjall::Error> {
    let db = Database::builder(dir).open()?;
    let library = db.keyspace("library", KeyspaceCreateOptions::default)?;
    let mut records = Vec::new();
    for guard in library.prefix([PRACTICE.byte()]) {
        let (key, value) = guard.into_inner()?;
        records.push((key.to_vec(), value.to_vec()));
    }
    for (key, mut value) in records {
        value.pop();
        library.insert(key, value)?;
    }
    db.persist(PersistMode::SyncAll)
}
