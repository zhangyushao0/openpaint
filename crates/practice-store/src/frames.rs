//! The records' upgrade of the practice sheets laid before 1.17.0 (S99, D103). Up to 1.16.0 every
//! pair's frames were square and a picture lay in its copy's square at 90 % of it, centred; from
//! 1.17.0 a picture's frames are its shape (`Sheet::of_session`). A record written before ends
//! without its layout byte, and the upgrade lays its sheet anew once, when the library opens: each
//! stroke and fill begun in the square of a copy of a picture a pack holds moved by the picture's
//! place in the square onto the frame of its shape, so it lies where it lay on its picture; a
//! stroke's clip and a fill's region moved alike and kept to the new frame (ink in the old
//! margins kept, cut as any stroke past a frame is). A picture no pack holds keeps its square
//! frames, its copy where it was. The sheet and its record are written in one batch.

use openpaint_app::{PortError, ProjectId};
use openpaint_core::{
    Artwork, ArtworkParts, CanvasSize, EditParts, MaskParts, RegionMask, StrokeParts,
};
use openpaint_practice::{Frame, Pack, Point, SessionRecord, Sheet};
use openpaint_store::Upgrading;

use crate::records::{PRACTICE, encode_session, practice_key};

/// Where a picture lay in its square frame up to 1.16.0: at this share of the frame, centred.
const OLD_SHOWN: f64 = 0.9;

/// An old copy's move: its square frame, the picture's top left in it and its new frame (canvas
/// pixels), and how much larger the picture is there.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Move {
    square: Frame,
    from: Point,
    frame: Frame,
    factor: f64,
}

impl Move {
    /// The copy of a picture of `size` pixels from its `square` onto its `frame`.
    fn of(square: Frame, frame: Frame, [width, height]: [u32; 2]) -> Self {
        let [width, height] = [width, height].map(|side| f64::from(side.max(1)));
        let scale = OLD_SHOWN / width.max(height);
        let shown = [width * scale, height * scale];
        let from =
            [0, 1].map(|axis| square.origin[axis] + (1.0 - shown[axis]) / 2.0 * square.size[axis]);
        Self {
            square,
            from,
            frame,
            factor: frame.size[0] / (shown[0] * square.size[0]),
        }
    }

    fn at(&self, p: Point) -> Point {
        [0, 1].map(|axis| self.frame.origin[axis] + (p[axis] - self.from[axis]) * self.factor)
    }

    fn back(&self, q: Point) -> Point {
        [0, 1].map(|axis| self.from[axis] + (q[axis] - self.frame.origin[axis]) / self.factor)
    }
}

/// The moves `record`'s sheet owes as `packs` hold its pictures: one for each copy of a picture
/// held; none for a colour copy, whose frames always were its picture's.
fn moves(record: &SessionRecord, packs: &[Pack]) -> Vec<Move> {
    if record.is_project() {
        return Vec::new();
    }
    let squares = Sheet::of(&vec![None; record.exercises.len()]);
    let laid = Sheet::of_session(record, packs);
    (record.exercises.iter().enumerate())
        .filter_map(|(index, exercise)| {
            let item = exercise.picture.as_ref()?;
            let size = packs.iter().find_map(|pack| pack.find(item))?.size;
            let square = squares.pairs.get(index)?.copy;
            Some(Move::of(square, laid.pairs.get(index)?.copy, size))
        })
        .collect()
}

/// `session`'s record (written before 1.17.0) and its sheet laid anew as `packs` hold its
/// pictures, in one batch; the record alone when nothing moves. A sheet that cannot be read
/// keeps what it holds (logged).
pub(crate) fn relay(
    library: &Upgrading<'_>,
    session: &SessionRecord,
    packs: &[Pack],
) -> Result<(), PortError> {
    let (key, bytes) = (practice_key(session.id), encode_session(session));
    let record = || library.put(PRACTICE, &[(key.clone(), bytes.clone())]);
    let moves = moves(session, packs);
    if moves.is_empty() || session.canvas == 0 {
        return record();
    }
    let id = ProjectId::new(session.canvas);
    let artwork = match library.artwork(id) {
        Ok(artwork) => artwork,
        Err(error) => {
            log::warn!("practice sheet {} not laid anew: {error}", id.get());
            return record();
        }
    };
    let mut parts = artwork.to_parts();
    let moved = relaid(&mut parts, artwork.canvas(), &moves);
    if moved == 0 {
        return record();
    }
    let laid = Artwork::from_storage(parts)?;
    log::info!(
        "practice sheet {}: {moved} strokes and fills laid anew",
        id.get()
    );
    library.replace_owned((id, &laid), PRACTICE, (&key, &bytes))
}

/// Every stroke and fill of `parts` begun in one of `moves`' squares moved onto its frame, on a
/// canvas of `canvas`; how many moved.
fn relaid(parts: &mut ArtworkParts, canvas: CanvasSize, moves: &[Move]) -> usize {
    let mut moved = 0;
    for layer in &mut parts.layers {
        moved += strokes(&mut layer.strokes, canvas, moves);
        moved += fills(&mut layer.edits, canvas, moves);
        if let Some(mask) = layer.mask.as_mut() {
            moved += strokes(&mut mask.strokes, canvas, moves);
            moved += fills(&mut mask.edits, canvas, moves);
        }
    }
    moved
}

fn strokes(strokes: &mut [StrokeParts], canvas: CanvasSize, moves: &[Move]) -> usize {
    let mut moved = 0;
    for stroke in strokes {
        let Some(first) = stroke.samples.first() else {
            continue;
        };
        let begun = [f64::from(first.x), f64::from(first.y)];
        let Some(by) = moves.iter().find(|m| m.square.holds(begun)) else {
            continue;
        };
        for sample in &mut stroke.samples {
            let [x, y] = by.at([f64::from(sample.x), f64::from(sample.y)]);
            (sample.x, sample.y) = (x as f32, y as f32);
        }
        if let Some(clip) = stroke.clip.as_mut() {
            *clip = moved_mask(clip, canvas, by).unwrap_or_else(|| clip.clone());
        }
        moved += 1;
    }
    moved
}

fn fills(edits: &mut [EditParts], canvas: CanvasSize, moves: &[Move]) -> usize {
    let mut moved = 0;
    for edit in edits {
        let EditParts::Fill(fill) = edit else {
            continue;
        };
        let middle =
            [0, 1].map(|axis| f64::from(fill.origin[axis]) + f64::from(fill.size[axis]) / 2.0);
        let Some(by) = moves.iter().find(|m| m.square.contains(middle, 0.0)) else {
            continue;
        };
        let region = MaskParts {
            origin: fill.origin,
            size: fill.size,
            runs: fill.runs.clone(),
        };
        if let Some(MaskParts { origin, size, runs }) = moved_mask(&region, canvas, by) {
            (fill.origin, fill.size, fill.runs) = (origin, size, runs);
            moved += 1;
        }
    }
    moved
}

/// `mask` moved by `by` and kept to its new frame: each pixel of the frame whose middle comes
/// from a pixel of `mask`; none when no pixel does (or `mask` does not read).
fn moved_mask(mask: &MaskParts, canvas: CanvasSize, by: &Move) -> Option<MaskParts> {
    let old = RegionMask::from_runs(canvas, mask.origin, mask.size, &mask.runs).ok()?;
    let [width, height] = [canvas.width(), canvas.height()].map(f64::from);
    let low = [0, 1].map(|axis| {
        by.frame.origin[axis]
            .floor()
            .clamp(0.0, [width, height][axis])
    });
    let high = [0, 1].map(|axis| {
        (by.frame.origin[axis] + by.frame.size[axis])
            .ceil()
            .clamp(0.0, [width, height][axis])
    });
    let [x0, y0] = low.map(|v| v as u32);
    let size = [0, 1].map(|axis| (high[axis] - low[axis]) as u32);
    let mut runs = Vec::new();
    let (mut inside, mut length, mut any) = (false, 0_u32, false);
    for y in y0..y0 + size[1] {
        for x in x0..x0 + size[0] {
            let middle = [f64::from(x) + 0.5, f64::from(y) + 0.5];
            let [px, py] = by.back(middle);
            let from = px >= 0.0 && py >= 0.0 && old.contains([px as u32, py as u32]);
            let kept = from && by.frame.contains(middle, 0.0);
            any |= kept;
            if kept != inside {
                runs.push(length);
                (inside, length) = (kept, 0);
            }
            length += 1;
        }
    }
    runs.push(length);
    any.then_some(MaskParts {
        origin: [x0, y0],
        size,
        runs,
    })
}

#[cfg(test)]
#[path = "frames_tests.rs"]
mod tests;
