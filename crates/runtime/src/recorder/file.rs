//! The recorder's file: each line appended on a worker of its own (`worker::Queue`), never on the
//! frame's thread, the file opened for each and closed again (a chore may empty it meanwhile); it
//! never holds more than `FRAMES_KEPT` lines, the newest: full, its oldest `DROP` go at once.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::error::RuntimeError;
use crate::worker::Queue;

/// The most lines the file holds.
pub const FRAMES_KEPT: usize = 5_000;
/// Lines dropped at once when the file is full, so it is rewritten once every this many lines.
const DROP: usize = 500;

/// Appends lines to the file at a path, on its worker; dropping it writes what it was handed.
pub(super) struct FramesFile {
    queue: Queue<String, ()>,
}

impl FramesFile {
    pub(super) fn at(path: PathBuf) -> Result<Self, RuntimeError> {
        let mut lines = None;
        let queue = Queue::spawn("frames-file", move |line: String| {
            let held = lines.get_or_insert_with(|| count(&path));
            if *held >= FRAMES_KEPT {
                *held = keep_newest(&path, FRAMES_KEPT - DROP);
            }
            match append(&path, &line) {
                Ok(()) => *held += 1,
                Err(error) => log::warn!("{}: {error}", path.display()),
            }
        })?;
        Ok(Self { queue })
    }

    /// `line` to the file's end, on the worker.
    pub(super) fn keep(&mut self, line: String) {
        // The worker's answers carry nothing; taken so none piles up.
        while self.queue.take().is_some() {}
        self.queue.submit(line);
    }
}

/// The lines the file holds; none when there is no file.
fn count(path: &Path) -> usize {
    std::fs::read_to_string(path).map_or(0, |text| text.lines().count())
}

fn append(path: &Path, line: &str) -> std::io::Result<()> {
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{line}")
}

/// The file rewritten with its newest `kept` lines (beside it first, then moved over it); how many
/// it holds then.
fn keep_newest(path: &Path, kept: usize) -> usize {
    let Ok(text) = std::fs::read_to_string(path) else {
        return 0;
    };
    let lines: Vec<&str> = text.lines().collect();
    let newest = &lines[lines.len().saturating_sub(kept)..];
    let mut written = newest.join("\n");
    if !newest.is_empty() {
        written.push('\n');
    }
    let part = path.with_extension("part");
    let moved = std::fs::write(&part, written).and_then(|()| std::fs::rename(&part, path));
    if let Err(error) = moved {
        log::warn!("{}: {error}", path.display());
        return lines.len();
    }
    newest.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_file_never_holds_more_than_its_cap_and_keeps_the_newest() {
        let dir = std::env::temp_dir().join(format!("openpaint-frames-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a test directory");
        let path = dir.join("frames.txt");
        // Found nearly full: ten lines short of the cap.
        let found: Vec<String> = (0..FRAMES_KEPT - 10).map(|n| format!("old {n}")).collect();
        std::fs::write(&path, found.join("\n") + "\n").expect("written");
        let mut file = FramesFile::at(path.clone()).expect("a worker");
        for n in 0..20 {
            file.keep(format!("new {n}"));
        }
        // Dropping it writes what it was handed.
        drop(file);
        let text = std::fs::read_to_string(&path).expect("read");
        let lines: Vec<&str> = text.lines().collect();
        // Full after ten: the oldest 500 went, then ten more came.
        assert_eq!(lines.len(), FRAMES_KEPT - DROP + 10);
        assert_eq!(lines[0], format!("old {DROP}"));
        assert_eq!(lines[lines.len() - 21], format!("old {}", FRAMES_KEPT - 11));
        assert_eq!(
            lines[lines.len() - 20..],
            *(0..20).map(|n| format!("new {n}")).collect::<Vec<_>>()
        );
        assert!(!dir.join("frames.part").exists());
        std::fs::remove_dir_all(&dir).expect("removed");
    }
}
