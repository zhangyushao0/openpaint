//! Reading the tar archives the device's `tar` writes (ustar headers in 512-byte blocks), enough
//! to list a backup and compare two of them file by file.

use super::adb::{DeviceError, Result};

const BLOCK: usize = 512;

/// One archive member: its path, its kind (`'0'` a file, `'5'` a directory, ...) and its bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub path: String,
    pub kind: u8,
    pub bytes: Vec<u8>,
}

/// Every member of `archive`, in order. An empty archive, or one cut short, is an error.
pub fn entries(archive: &[u8]) -> Result<Vec<Entry>> {
    let mut out = Vec::new();
    let mut at = 0;
    while at + BLOCK <= archive.len() {
        let header = &archive[at..at + BLOCK];
        if header.iter().all(|byte| *byte == 0) {
            break;
        }
        let size = octal(&header[124..136])?;
        let start = at + BLOCK;
        let bytes = archive
            .get(start..start + size)
            .ok_or_else(|| DeviceError::Tar(format!("{} is cut short", name(header))))?;
        out.push(Entry {
            path: name(header),
            kind: header[156],
            bytes: bytes.to_vec(),
        });
        at = start + size.div_ceil(BLOCK) * BLOCK;
    }
    if out.is_empty() {
        let head = String::from_utf8_lossy(&archive[..archive.len().min(200)]).into_owned();
        return Err(DeviceError::Tar(format!("no members; it begins {head:?}")));
    }
    Ok(out)
}

/// What differs between `backup` and `now`, member by member: missing, extra or other bytes.
pub fn differences(backup: &[Entry], now: &[Entry]) -> Vec<String> {
    let mut out = Vec::new();
    for entry in backup {
        match now.iter().find(|other| other.path == entry.path) {
            None => out.push(format!("{}: missing", entry.path)),
            Some(other) if other.kind != entry.kind || other.bytes != entry.bytes => {
                out.push(format!(
                    "{}: {} bytes, the backup's {}",
                    entry.path,
                    other.bytes.len(),
                    entry.bytes.len()
                ))
            }
            Some(_) => {}
        }
    }
    for other in now {
        if !backup.iter().any(|entry| entry.path == other.path) {
            out.push(format!("{}: not in the backup", other.path));
        }
    }
    out
}

/// The member's path: the ustar prefix, then the name.
fn name(header: &[u8]) -> String {
    let text = |field: &[u8]| {
        let end = field
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(field.len());
        String::from_utf8_lossy(&field[..end]).into_owned()
    };
    let (prefix, name) = (text(&header[345..500]), text(&header[..100]));
    if prefix.is_empty() {
        name
    } else {
        format!("{prefix}/{name}")
    }
}

fn octal(field: &[u8]) -> Result<usize> {
    let text = String::from_utf8_lossy(field);
    let digits = text.trim_matches(|c: char| c == '\0' || c == ' ');
    usize::from_str_radix(digits, 8).map_err(|_| DeviceError::Tar(format!("size {digits:?}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn member(path: &str, kind: u8, bytes: &[u8]) -> Vec<u8> {
        let mut header = vec![0u8; BLOCK];
        header[..path.len()].copy_from_slice(path.as_bytes());
        let size = format!("{:011o}\0", bytes.len());
        header[124..136].copy_from_slice(size.as_bytes());
        header[156] = kind;
        header.extend_from_slice(bytes);
        header.resize(BLOCK + bytes.len().div_ceil(BLOCK) * BLOCK, 0);
        header
    }

    fn archive(members: &[(&str, u8, &[u8])]) -> Vec<u8> {
        let mut out: Vec<u8> = members
            .iter()
            .flat_map(|(path, kind, bytes)| member(path, *kind, bytes))
            .collect();
        out.extend([0; 2 * BLOCK]);
        out
    }

    #[test]
    fn members_are_listed_and_compared_by_their_bytes() {
        let backup = archive(&[
            ("files/library", b'5', b""),
            ("files/library/a", b'0', &[7; 600]),
        ]);
        let backup = entries(&backup).expect("a backup");
        assert_eq!(backup.len(), 2);
        assert_eq!(
            (backup[1].path.as_str(), backup[1].bytes.len()),
            ("files/library/a", 600)
        );
        assert!(differences(&backup, &backup).is_empty());
        let now = archive(&[
            ("files/library/a", b'0', &[8; 600]),
            ("files/library/b", b'0', b"x"),
        ]);
        let now = entries(&now).expect("an archive");
        assert_eq!(
            differences(&backup, &now),
            [
                "files/library: missing",
                "files/library/a: 600 bytes, the backup's 600",
                "files/library/b: not in the backup"
            ]
        );
    }

    #[test]
    fn what_is_no_archive_is_refused() {
        assert!(entries(b"run-as: package not debuggable: dev.openpaint\n").is_err());
        assert!(entries(&[]).is_err());
        let mut cut = archive(&[("a", b'0', &[1; 600])]);
        cut.truncate(BLOCK + 100);
        assert!(entries(&cut).is_err());
    }
}
