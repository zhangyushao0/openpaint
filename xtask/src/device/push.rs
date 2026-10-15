//! `cargo device-push <file>` (S26, S58): a picture for the system picker, or an OpenPaint file
//! for `device-input open-file`, into the test folder `device-clean` removes.

use std::path::Path;

use super::adb::{Adb, DeviceError, Result, read_file};
use super::plain;
use crate::process::root;

/// Where `push` puts a file: one of the test locations `clean` removes.
pub(super) const PUSHED: &str = "/sdcard/Download/openpaint-test";

const USAGE: &str =
    "cargo device-push <picture.png|.jpg|artwork.openpaint>, named with a-z, 0-9 and '-'";

/// The file name a pushed file keeps: a-z, 0-9 and '-', then `.png`, `.jpg`, `.jpeg` or
/// `.openpaint` (S58: a test artwork from `cargo test-art`).
pub(super) fn pushed_name(path: &Path) -> Result<String> {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    let (stem, extension) = name.rsplit_once('.').unwrap_or((name, ""));
    let known = matches!(extension, "png" | "jpg" | "jpeg" | "openpaint");
    if known && plain(stem).is_ok() {
        Ok(name.to_owned())
    } else {
        Err(DeviceError::Usage(USAGE))
    }
}

/// `cargo device-push <file>`: the file into `PUSHED`, made known to the device's media index so
/// the picker lists a picture.
pub fn push(args: &[String]) -> Result<()> {
    let [file] = args else {
        return Err(DeviceError::Usage(USAGE));
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
    adb.shell(&["mkdir", "-p", PUSHED])?;
    let target = format!("{PUSHED}/{name}");
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
        assert!(PUSHED.starts_with("/sdcard/") && TEST_DIRS.contains(&PUSHED));
    }
}
