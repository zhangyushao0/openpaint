//! What the gate's build superseded, removed (S106c). Cargo never deletes a build: each new
//! version, feature set or profile builds every test binary and workspace library under a new
//! hash beside the old ones, and `target` held 274 GB on 2026-11-07. The gate's last step asks
//! cargo for its test build's units again (fresh by then) and removes, each path named from
//! cargo's messages and the folders' listings, never a pattern, and printed:
//! - every other linked build of each test binary: `<stem>-<hash>.exe`, `.pdb`, `.d` and its
//!   fingerprint folder, whatever its age (`cargo test -p` relinks its own when next run), and
//!   alike every linked build of a test its package no longer has (renamed, moved, deleted);
//! - every other build of each workspace library not built for `STALE`: `lib<name>-<hash>.rlib`,
//!   `.rmeta`, `<name>-<hash>.d` and its fingerprint folder (`cargo check -p`, `cargo run` and
//!   clippy build their own, in use while their crates are worked on);
//! - the incremental caches of the profiles the gate builds not written for `STALE`: rustc's
//!   names do not say which build a cache belongs to, and each compile rewrites its cache.
//!
//! A build's fingerprint folder (`.fingerprint/<package>-<hash>`) names its target, so a build of
//! another package or kind under the same stem (a `cargo run` binary, clippy's check of a test)
//! is never taken. Dependency libraries are never touched.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::env::consts::EXE_SUFFIX;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, SystemTime};

use super::{string, strings, tested};
use crate::gate::ANDROID_TRIPLE;
use crate::process::{cargo_command, root};

/// How long a library's other build or an incremental cache stays unwritten before it counts as
/// superseded: a new version comes with each slice, several a day.
const STALE: Duration = Duration::from_secs(24 * 60 * 60);

/// A unit of the gate's test build: a test binary or a workspace library.
struct Unit {
    /// The folder of its outputs (`target/debug/deps`).
    deps: PathBuf,
    /// Its outputs' name before the hash: `fill`, `libopenpaint_core`.
    stem: String,
    hash: String,
    test: bool,
}

pub fn run() -> Result<(), String> {
    let output = cargo_command()
        .args(["test", "--workspace", "--no-run", "--message-format=json"])
        .current_dir(root())
        .stderr(Stdio::inherit())
        .output()
        .map_err(|error| format!("cannot start cargo test --no-run: {error}"))?;
    if !output.status.success() {
        return Err("cargo test --workspace --no-run failed".to_owned());
    }
    let messages = String::from_utf8_lossy(&output.stdout);
    let units: Vec<Unit> = messages.lines().filter_map(unit).collect();
    let target = root().join("target");
    let now = SystemTime::now();
    let mut paths = superseded(&units, now);
    let profiles = [
        target.join("debug"),
        target.join(ANDROID_TRIPLE).join("debug"),
    ];
    paths.extend(stale_caches(&profiles, now));
    let removed = remove(&paths);
    println!(
        "  removed {} files and {} folders, {:.2} GB, that the build of {} test binaries and \
         libraries superseded{}; target holds {:.1} GB",
        removed.files,
        removed.folders,
        gb(removed.bytes),
        units.len(),
        if removed.kept > 0 {
            format!(" ({} could not be removed)", removed.kept)
        } else {
            String::new()
        },
        gb(size(&target))
    );
    Ok(())
}

/// The workspace unit a cargo message announces: a test binary, or a library with its rlib.
fn unit(message: &str) -> Option<Unit> {
    let test = tested(message)?;
    if !string(message, "package_id")?.starts_with("path+file:") {
        return None;
    }
    let output = if test {
        PathBuf::from(string(message, "executable")?)
    } else {
        let mut files = strings(message, "filenames").into_iter();
        PathBuf::from(files.find(|file| file.ends_with(".rlib"))?)
    };
    let (stem, hash) = split(output.file_stem()?.to_str()?)?;
    Some(Unit {
        deps: output.parent()?.to_path_buf(),
        stem: stem.to_owned(),
        hash: hash.to_owned(),
        test,
    })
}

/// `<stem>-<hash>` split, the hash cargo's 16 hex digits.
fn split(name: &str) -> Option<(&str, &str)> {
    let (stem, hash) = name.rsplit_once('-')?;
    let hex = hash.len() == 16 && hash.bytes().all(|byte| byte.is_ascii_hexdigit());
    hex.then_some((stem, hash))
}

/// The paths of every build the `units` supersede, folder by folder: their targets' other builds,
/// and the test builds of targets their packages no longer have.
fn superseded(units: &[Unit], now: SystemTime) -> Vec<PathBuf> {
    let mut folders: HashMap<&Path, (Folder, HashSet<Target>)> = HashMap::new();
    let mut paths = Vec::new();
    for unit in units {
        let (folder, targets) = (folders.entry(&unit.deps))
            .or_insert_with(|| (Folder::read(&unit.deps), HashSet::new()));
        if let Some(target) = folder.target(unit) {
            paths.extend(folder.siblings(unit, &target, now));
            targets.insert(target);
        }
    }
    for (folder, targets) in folders.values() {
        paths.extend(folder.orphans(targets));
    }
    paths
}

/// A build's target as its fingerprint folder names it: the package, and the fingerprint's
/// name for the target (`test-integration-test-fill`).
type Target = (String, String);

/// An output folder as listed once: its files, and its builds' fingerprint folders.
struct Folder {
    deps: PathBuf,
    files: HashSet<String>,
    /// The hashes of every file named `<stem>-<hash>[.ext]`, by stem.
    hashes: HashMap<String, BTreeSet<String>>,
    fingerprint: PathBuf,
    /// The name of each fingerprint folder, `<package>-<hash>`, by hash.
    packages: HashMap<String, String>,
}

impl Folder {
    fn read(deps: &Path) -> Folder {
        let files: HashSet<String> = names(deps).into_iter().collect();
        let mut hashes: HashMap<String, BTreeSet<String>> = HashMap::new();
        for file in &files {
            let name = Path::new(file).file_stem().and_then(|stem| stem.to_str());
            if let Some((stem, hash)) = name.and_then(split) {
                let stem = hashes.entry(stem.to_owned()).or_default();
                stem.insert(hash.to_owned());
            }
        }
        let fingerprint = deps.with_file_name(".fingerprint");
        let packages = (names(&fingerprint).into_iter())
            .filter_map(|name| Some((split(&name)?.1.to_owned(), name.clone())))
            .collect();
        Folder {
            deps: deps.to_path_buf(),
            files,
            hashes,
            fingerprint,
            packages,
        }
    }

    /// The target of the build of `unit` here.
    fn target(&self, unit: &Unit) -> Option<Target> {
        let built = self.packages.get(&unit.hash)?;
        let package = &built[..built.len() - unit.hash.len() - 1];
        Some((package.to_owned(), identity(&self.fingerprint.join(built))?))
    }

    /// The files and fingerprint folders of the other builds of `unit`'s target here that it
    /// supersedes.
    fn siblings(&self, unit: &Unit, (package, target): &Target, now: SystemTime) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        for hash in self.hashes.get(&unit.stem).into_iter().flatten() {
            let (shown_by, files) = outputs(unit, hash);
            let built = format!("{package}-{hash}");
            let fingerprint = self.fingerprint.join(&built);
            let superseded = hash != &unit.hash
                && files[..shown_by]
                    .iter()
                    .any(|file| self.files.contains(file))
                && self.packages.get(hash) == Some(&built)
                && fingerprint.join(target).is_file()
                && (unit.test || stale(&fingerprint.join("invoked.timestamp"), now));
            if superseded {
                paths.extend(self.present(&files));
                paths.push(fingerprint);
            }
        }
        paths
    }

    /// The linked test builds here of the targets of the `current` packages that the current
    /// build does not have (a test renamed, moved or deleted).
    fn orphans(&self, current: &HashSet<Target>) -> Vec<PathBuf> {
        let packages: HashSet<&str> = current
            .iter()
            .map(|(package, _)| package.as_str())
            .collect();
        let mut paths = Vec::new();
        for (stem, hashes) in &self.hashes {
            for hash in hashes {
                let files = [EXE_SUFFIX, ".pdb", ".d"].map(|ext| format!("{stem}-{hash}{ext}"));
                let Some(built) = self.packages.get(hash) else {
                    continue;
                };
                let package = &built[..built.len() - hash.len() - 1];
                if !self.files.contains(&files[0]) || !packages.contains(package) {
                    continue;
                }
                let fingerprint = self.fingerprint.join(built);
                let Some(target) = identity(&fingerprint) else {
                    continue;
                };
                if target.starts_with("test-") && !current.contains(&(package.to_owned(), target)) {
                    paths.extend(self.present(&files));
                    paths.push(fingerprint);
                }
            }
        }
        paths
    }

    /// Those of `files` this folder holds, as paths.
    fn present<'a>(&'a self, files: &'a [String]) -> impl Iterator<Item = PathBuf> + 'a {
        let present = files.iter().filter(|file| self.files.contains(*file));
        present.map(|file| self.deps.join(file))
    }
}

/// The files a build of `unit`'s target under `hash` leaves in its folder, and how many of the
/// first show that it was built (not only checked).
fn outputs(unit: &Unit, hash: &str) -> (usize, [String; 3]) {
    let stem = &unit.stem;
    if unit.test {
        let [exe, pdb, d] = [EXE_SUFFIX, ".pdb", ".d"].map(|ext| format!("{stem}-{hash}{ext}"));
        (1, [exe, pdb, d])
    } else {
        let name = stem.strip_prefix("lib").unwrap_or(stem);
        let rlib = format!("{stem}-{hash}.rlib");
        (
            2,
            [
                rlib,
                format!("{stem}-{hash}.rmeta"),
                format!("{name}-{hash}.d"),
            ],
        )
    }
}

/// The target a fingerprint folder fingerprints: the file beside its `.json`
/// (`test-integration-test-fill`, `lib-openpaint_core`, `bin-openpaint-harness`).
fn identity(fingerprint: &Path) -> Option<String> {
    let names = names(fingerprint);
    let target = names
        .iter()
        .find(|name| names.contains(&format!("{name}.json")));
    target.cloned()
}

/// The incremental caches of the `profiles` (`target/debug`) not written for `STALE`.
fn stale_caches(profiles: &[PathBuf], now: SystemTime) -> Vec<PathBuf> {
    let caches = profiles.iter().map(|profile| profile.join("incremental"));
    let caches = caches.flat_map(|folder| names(&folder).into_iter().map(move |n| folder.join(n)));
    caches
        .filter(|cache| cache.is_dir() && stale(cache, now))
        .collect()
}

/// Whether `path` was last written `STALE` or longer before `now`.
fn stale(path: &Path, now: SystemTime) -> bool {
    let modified = fs::metadata(path).and_then(|metadata| metadata.modified());
    modified.is_ok_and(|modified| now.duration_since(modified).is_ok_and(|age| age >= STALE))
}

/// The names in a folder; none when it cannot be read.
fn names(folder: &Path) -> Vec<String> {
    let entries = fs::read_dir(folder).into_iter().flatten().flatten();
    entries
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect()
}

/// What `remove` took away and could not.
#[derive(Default)]
struct Removed {
    files: usize,
    folders: usize,
    bytes: u64,
    kept: usize,
}

/// Removes each path, a file or a folder with everything in it, printing it; a path that cannot
/// be removed (a running binary) is printed and kept.
fn remove(paths: &[PathBuf]) -> Removed {
    let mut removed = Removed::default();
    for path in paths {
        let Ok(metadata) = fs::symlink_metadata(path) else {
            continue;
        };
        let folder = metadata.is_dir();
        let bytes = if folder { size(path) } else { metadata.len() };
        let result = if folder {
            fs::remove_dir_all(path)
        } else {
            fs::remove_file(path)
        };
        match result {
            Ok(()) => {
                println!("  removed {}", path.display());
                removed.bytes += bytes;
                if folder {
                    removed.folders += 1;
                } else {
                    removed.files += 1;
                }
            }
            Err(error) => {
                println!("  kept {}: {error}", path.display());
                removed.kept += 1;
            }
        }
    }
    removed
}

/// The bytes of a file, or of everything in a folder; links not followed.
fn size(path: &Path) -> u64 {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return 0;
    };
    if !metadata.is_dir() {
        return metadata.len();
    }
    let entries = fs::read_dir(path).into_iter().flatten().flatten();
    entries.map(|entry| size(&entry.path())).sum()
}

fn gb(bytes: u64) -> f64 {
    bytes as f64 / f64::from(1 << 30)
}

#[cfg(test)]
#[path = "prune_tests.rs"]
mod tests;
