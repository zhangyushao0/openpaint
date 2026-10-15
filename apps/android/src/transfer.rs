//! Files the system hands the app, as the runtime's ports: a document opened for writing an
//! export (the save dialog's, or one the host makes for sharing) and a picture opened for
//! reading. Also the one-line form of the requests the Kotlin host serves. Plain Rust; the file
//! descriptors become `File`s in the JNI layer.

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use openpaint_runtime::{HostRequest, ImageSink, ImageSource, PortError};

/// What the Kotlin host was asked to show and the next answer is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pick {
    /// The save dialog: the answer is a file to write.
    Save,
    /// The image picker: the answer is a file to read.
    Open,
}

/// A file to write the export into. Writing replaces what it held.
pub struct FileSink(pub File);

impl ImageSink for FileSink {
    fn write(self: Box<Self>, bytes: &[u8]) -> Result<(), PortError> {
        let mut file = self.0;
        file.set_len(0).ok(); // A pipe from a provider cannot be truncated; it is empty anyway.
        file.write_all(bytes).map_err(device)?;
        file.flush().map_err(device)?;
        file.sync_all().ok(); // Not every provider's descriptor can sync; the bytes are written.
        Ok(())
    }
}

/// A picture file the user chose.
pub struct FileSource(pub File);

impl ImageSource for FileSource {
    fn read(self: Box<Self>, limit: u64) -> Result<Vec<u8>, PortError> {
        let mut bytes = Vec::new();
        self.0
            .take(limit.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(device)?;
        Ok(bytes)
    }
}

/// A file's bytes already read (debug builds' `open-file`, S58: the instrumentation's shell read
/// it, as the app may not).
pub struct BytesSource(pub Vec<u8>);

impl ImageSource for BytesSource {
    fn read(self: Box<Self>, limit: u64) -> Result<Vec<u8>, PortError> {
        let mut bytes = self.0;
        bytes.truncate(usize::try_from(limit.saturating_add(1)).unwrap_or(usize::MAX));
        Ok(bytes)
    }
}

fn device(error: std::io::Error) -> PortError {
    PortError::Device {
        detail: error.to_string(),
    }
}

/// A new file `name` in `dir`, the host's place for files it shares (only the newest is kept:
/// the share sheet's receiver reads it soon after). `name` must be a plain file name.
///
/// # Errors
/// `Device` when the directory or the file cannot be made, or `name` is not a plain file name.
pub fn share_file(dir: &Path, name: &str) -> Result<(PathBuf, File), PortError> {
    if name.is_empty() || name.contains(['/', '\\']) || name == "." || name == ".." {
        return Err(PortError::Device {
            detail: format!("not a file name: {name}"),
        });
    }
    fs::create_dir_all(dir).map_err(device)?;
    for entry in fs::read_dir(dir).map_err(device)?.flatten() {
        // Earlier shares; one that cannot go is overwritten or left for the system to clear.
        let _ = fs::remove_file(entry.path());
    }
    let path = dir.join(name);
    let file = File::create(&path).map_err(device)?;
    Ok((path, file))
}

/// The media type an OpenPaint file is saved under (DECISIONS D62).
const ARTWORK_MIME: &str = "application/octet-stream";

/// The line the Kotlin host reads for `request` (fields separated by tabs; names never hold
/// one), and what its answer will be for; `None` for a request the Rust side serves itself.
#[must_use]
pub fn request_line(request: &HostRequest, share_dir: &Path) -> Option<(String, Option<Pick>)> {
    match request {
        HostRequest::SaveFile { name, format } => {
            Some((format!("save\t{}\t{name}", format.mime()), Some(Pick::Save)))
        }
        HostRequest::OpenImage => Some(("open".to_owned(), Some(Pick::Open))),
        // Android knows no type for `.openpaint`: as octet-stream the dialog keeps the name.
        HostRequest::SaveArtwork { name } => {
            Some((format!("save\t{ARTWORK_MIME}\t{name}"), Some(Pick::Save)))
        }
        HostRequest::OpenArtwork => Some(("open-file".to_owned(), Some(Pick::Open))),
        HostRequest::Send { name, format } => {
            let path = share_dir.join(name);
            Some((format!("send\t{}\t{}", format.mime(), path.display()), None))
        }
        HostRequest::ShareFile { .. } => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openpaint_runtime::ImageFormat;

    #[test]
    fn requests_become_tab_separated_lines() {
        let dir = Path::new("/cache/share");
        let save = HostRequest::SaveFile {
            name: "Harbour.png".into(),
            format: ImageFormat::Png,
        };
        assert_eq!(
            request_line(&save, dir),
            Some(("save\timage/png\tHarbour.png".into(), Some(Pick::Save)))
        );
        let send = HostRequest::Send {
            name: "Harbour.jpg".into(),
            format: ImageFormat::Jpeg,
        };
        let (line, pick) = request_line(&send, dir).expect("a line");
        assert!(line.starts_with("send\timage/jpeg\t"), "{line}");
        assert!(line.ends_with("Harbour.jpg"), "{line}");
        assert_eq!(pick, None);
        let open = request_line(&HostRequest::OpenImage, dir);
        assert_eq!(open, Some(("open".into(), Some(Pick::Open))));
        let save = HostRequest::SaveArtwork {
            name: "Harbour.openpaint".into(),
        };
        assert_eq!(
            request_line(&save, dir),
            Some((
                "save\tapplication/octet-stream\tHarbour.openpaint".into(),
                Some(Pick::Save)
            ))
        );
        let open = request_line(&HostRequest::OpenArtwork, dir);
        assert_eq!(open, Some(("open-file".into(), Some(Pick::Open))));
        let share = HostRequest::ShareFile {
            name: "a.png".into(),
            format: ImageFormat::Png,
        };
        assert_eq!(request_line(&share, dir), None);
    }

    #[test]
    fn a_shared_file_replaces_the_one_before_and_round_trips() {
        let dir = std::env::temp_dir().join(format!("openpaint-share-{}", std::process::id()));
        let (first, file) = share_file(&dir, "first.png").expect("first");
        Box::new(FileSink(file)).write(b"one").expect("write");
        let (second, file) = share_file(&dir, "second.png").expect("second");
        Box::new(FileSink(file)).write(b"two").expect("write");
        assert!(!first.exists(), "only the newest share is kept");
        let read = Box::new(FileSource(File::open(&second).expect("open")));
        assert_eq!(read.read(2).expect("read"), b"two");
        assert!(share_file(&dir, "../escape.png").is_err());
        fs::remove_dir_all(&dir).expect("clean up");
    }
}
