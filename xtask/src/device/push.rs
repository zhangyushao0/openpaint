//! `cargo device-push [--download] <file>`: a picture for the system picker, an OpenPaint file
//! for `device-input open-file` or a practice pack for the practice options' import, into the
//! test folder `device-clean` removes; with `--download` into the device's Download folder
//! itself, for the author to pick (a pack left for them), which no chore removes.

use std::path::Path;

use super::adb::{Adb, DeviceError, Result, read_file};
use super::plain;
use crate::process::root;

/// Where `push` puts a file: one of the test locations `clean` removes.
pub(super) const PUSHED: &str = "/sdcard/Download/openpaint-test";
/// Where `push --download` puts a file for the author.
const DOWNLOAD: &str = "/sdcard/Download";

const USAGE: &str = "cargo device-push [--download] <picture.png|.jpg|artwork.openpaint|\
                     pack.pack>, named with a-z, 0-9 and '-'";

/// The file name a pushed file keeps: a-z, 0-9 and '-', then `.png`, `.jpg`, `.jpeg`,
/// `.openpaint` (a test artwork from `cargo test-art`) or `.pack` (a practice pack).
pub(super) fn pushed_name(path: &Path) -> Result<String> {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    let (stem, extension) = name.rsplit_once('.').unwrap_or((name, ""));
    let known = matches!(extension, "png" | "jpg" | "jpeg" | "openpaint" | "pack");
    if known && plain(stem).is_ok() {
        Ok(name.to_owned())
    } else {
        Err(DeviceError::Usage(USAGE))
    }
}

/// `cargo device-push [--download] <file>`: the file into `PUSHED` (or `DOWNLOAD`), made known to
/// the device's media index so the picker lists it.
pub fn push(args: &[String]) -> Result<()> {
    let (folder, file) = match args {
        [file] => (PUSHED, file),
        [flag, file] if flag == "--download" => (DOWNLOAD, file),
        _ => return Err(DeviceError::Usage(USAGE)),
    };
    let local = Path::new(file);
    let local = if local.exists() {
        local.to_path_buf()
    } else {
        root().join(local)
    };
    let name = pushed_name(&local)?;
    let bytes = read_file(&local)?.len();
    let adb = Adb::connected()?;
    adb.shell(&["mkdir", "-p", folder])?;
    let target = format!("{folder}/{name}");
    let local_text = local.to_string_lossy().into_owned();
    adb.text(&["push", &local_text, &target])?;
    let uri = format!("file://{target}");
    adb.shell(&[
        "am",
        "broadcast",
        "-a",
        "android.intent.action.MEDIA_SCANNER_SCAN_FILE",
        "-d",
        &uri,
    ])?;
    println!("pushed {} ({bytes} bytes) to {target}", local.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::TEST_DIRS;

    #[test]
    fn pushed_files_keep_plain_names_in_the_test_folder() {
        let name = |path: &str| pushed_name(Path::new(path)).ok();
        assert_eq!(
            name("tests/fixtures/s26-photo.jpg").as_deref(),
            Some("s26-photo.jpg")
        );
        assert_eq!(
            name("artifacts/test-art/a4-180.openpaint").as_deref(),
            Some("a4-180.openpaint")
        );
        assert!(name("a b.png").is_none() && name("x.txt").is_none() && name("png").is_none());
        assert!(name("A4.openpaint").is_none() && name("x.openpaint.zip").is_none());
        assert_eq!(
            name("artifacts/packs/openpaint-shapes.pack").as_deref(),
            Some("openpaint-shapes.pack")
        );
        assert!(PUSHED.starts_with("/sdcard/") && TEST_DIRS.contains(&PUSHED));
        assert!(
            !TEST_DIRS.contains(&DOWNLOAD),
            "what is left for the author stays"
        );
    }
}
