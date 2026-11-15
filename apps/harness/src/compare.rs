//! Two scenario runs' files compared (S129): a scenario's directory in the kept baseline against
//! the same directory of this run, file by file. Byte-identical files are alike; two PNGs of one
//! size that differ say how many pixels differ, the largest step of a channel and the box the
//! differing pixels lie in; anything else says only that it differs. A scenario's library and
//! packs (directories, removed when it passes) are not compared.

use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::path::Path;

use crate::output::read_rgba;

/// How one file of a scenario differs between the baseline and this run.
#[derive(Debug, PartialEq, Eq)]
pub enum Difference {
    /// Kept in the baseline, not written by this run.
    OnlyInBaseline,
    /// Written by this run, not in the baseline.
    OnlyInRun,
    /// Two PNGs of one size whose pixels differ.
    Pixels(Pixels),
    /// Different bytes that are not two PNGs of one size: their lengths.
    Bytes { baseline: u64, run: u64 },
}

/// Where and by how much two images of one size differ.
#[derive(Debug, PartialEq, Eq)]
pub struct Pixels {
    /// The pixels with any channel different.
    pub count: u64,
    /// The largest difference of one channel.
    pub step: u8,
    /// The box the differing pixels lie in: its first and last column and row.
    pub left_top: [u32; 2],
    pub right_bottom: [u32; 2],
}

impl fmt::Display for Difference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OnlyInBaseline => write!(f, "only in the baseline"),
            Self::OnlyInRun => write!(f, "only in this run"),
            Self::Pixels(pixels) => {
                let ([left, top], [right, bottom]) = (pixels.left_top, pixels.right_bottom);
                write!(
                    f,
                    "{} pixels, largest step {}, in x {left} to {right}, y {top} to {bottom}",
                    pixels.count, pixels.step
                )
            }
            Self::Bytes { baseline, run } => {
                write!(f, "bytes differ ({baseline} in the baseline, {run} now)")
            }
        }
    }
}

/// How many files the scenario `name` has in `baseline/<name>` and `run/<name>` together, and
/// each that differs, named `<name>/<file>`, in name order.
pub fn scenario(baseline: &Path, run: &Path, name: &str) -> (usize, Vec<(String, Difference)>) {
    let (kept, made) = (baseline.join(name), run.join(name));
    let names: BTreeSet<String> = files(&kept).into_iter().chain(files(&made)).collect();
    let differing = (names.iter())
        .filter_map(|file| {
            let difference = file_difference(&kept.join(file), &made.join(file))?;
            Some((format!("{name}/{file}"), difference))
        })
        .collect();
    (names.len(), differing)
}

/// The names of the files directly in `dir` (none when it is missing).
fn files(dir: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    (entries.flatten())
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect()
}

/// How the file at `baseline` differs from the one at `run`; `None` when they are alike.
fn file_difference(baseline: &Path, run: &Path) -> Option<Difference> {
    let (kept, made) = match (fs::read(baseline), fs::read(run)) {
        (Ok(kept), Ok(made)) => (kept, made),
        (Ok(_), Err(_)) => return Some(Difference::OnlyInBaseline),
        (Err(_), _) => return Some(Difference::OnlyInRun),
    };
    if kept == made {
        return None;
    }
    let bytes = Difference::Bytes {
        baseline: kept.len() as u64,
        run: made.len() as u64,
    };
    let (Some((size, old)), Some((size_now, new))) = (read_rgba(baseline), read_rgba(run)) else {
        return Some(bytes);
    };
    if size != size_now {
        return Some(bytes);
    }
    // Same pixels in different bytes (an encoder's choice) are alike.
    pixels(size[0], &old, &new).map(Difference::Pixels)
}

/// How two 8-bit RGBA images `width` pixels wide differ; `None` when every pixel is the same.
pub fn pixels(width: u32, old: &[u8], new: &[u8]) -> Option<Pixels> {
    let mut found: Option<Pixels> = None;
    let (old, new) = (old.as_chunks::<4>().0, new.as_chunks::<4>().0);
    for (index, (a, b)) in old.iter().zip(new).enumerate() {
        let step = a.iter().zip(b).map(|(a, b)| a.abs_diff(*b)).max()?;
        if step == 0 {
            continue;
        }
        let at = u32::try_from(index).ok()?;
        let (x, y) = (at % width.max(1), at / width.max(1));
        let found = found.get_or_insert(Pixels {
            count: 0,
            step: 0,
            left_top: [x, y],
            right_bottom: [x, y],
        });
        found.count += 1;
        found.step = found.step.max(step);
        found.left_top = [found.left_top[0].min(x), found.left_top[1].min(y)];
        found.right_bottom = [found.right_bottom[0].max(x), found.right_bottom[1].max(y)];
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::write_rgba;
    use crate::scratch::Scratch;

    #[test]
    fn differing_pixels_are_counted_with_their_largest_step_and_box() {
        let old = vec![10; 4 * 4 * 3];
        let mut new = old.clone();
        // Pixel (1, 0): one step in red; pixel (3, 2): five in alpha.
        new[4] = 11;
        new[(2 * 4 + 3) * 4 + 3] = 5;
        let found = pixels(4, &old, &new).expect("they differ");
        assert_eq!(
            found,
            Pixels {
                count: 2,
                step: 5,
                left_top: [1, 0],
                right_bottom: [3, 2]
            }
        );
        assert_eq!(
            Difference::Pixels(found).to_string(),
            "2 pixels, largest step 5, in x 1 to 3, y 0 to 2"
        );
        assert_eq!(pixels(4, &old, &old), None);
    }

    #[test]
    fn a_scenarios_files_are_compared_by_bytes_then_by_pixels() {
        let (baseline, run) = (
            Scratch::new("compare-kept").unwrap(),
            Scratch::new("compare-run").unwrap(),
        );
        for dir in [&baseline, &run] {
            fs::create_dir_all(dir.join("s1").join("library")).unwrap();
            fs::write(dir.join("s1").join("library").join("journal"), b"x").unwrap();
            fs::write(dir.join("s1").join("same.txt"), b"same").unwrap();
        }
        let pixels = vec![200; 3 * 2 * 4];
        write_rgba(&baseline.join("s1").join("shot.png"), [3, 2], &pixels).unwrap();
        let mut moved = pixels.clone();
        moved[4 * 4] = 199;
        write_rgba(&run.join("s1").join("shot.png"), [3, 2], &moved).unwrap();
        fs::write(baseline.join("s1").join("gone.png"), b"old").unwrap();
        fs::write(run.join("s1").join("new.png"), b"new").unwrap();
        fs::write(baseline.join("s1").join("text.txt"), b"ab").unwrap();
        fs::write(run.join("s1").join("text.txt"), b"abc").unwrap();
        let (count, found) = scenario(&baseline, &run, "s1");
        assert_eq!(count, 5, "the library's files are not compared");
        let lines: Vec<String> = found
            .iter()
            .map(|(file, d)| format!("{file}: {d}"))
            .collect();
        assert_eq!(
            lines,
            [
                "s1/gone.png: only in the baseline",
                "s1/new.png: only in this run",
                "s1/shot.png: 1 pixels, largest step 1, in x 1 to 1, y 1 to 1",
                "s1/text.txt: bytes differ (2 in the baseline, 3 now)",
            ]
        );
        assert_eq!(scenario(&baseline, &run, "absent"), (0, Vec::new()));
    }
}
