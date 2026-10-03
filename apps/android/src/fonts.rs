//! The system's text faces for the UI: the Kotlin host finds the regular sans (Roboto) and the
//! regular Simplified Chinese face (Noto Sans CJK) in `/system/etc/fonts.xml` and hands each as
//! "<face index>\n<path>"; this reads the file, within a budget, and the runtime gets the bytes.
//! Plain Rust, so it is tested on the development machine.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use openpaint_runtime::FontFile;

/// Largest font file read, in bytes: the Chinese collection is about 31 MiB.
pub const FONT_BUDGET: u64 = 40 * 1024 * 1024;

/// The face that `line` ("<index>\n<path>") names, read from its file.
pub fn read_face(line: &str) -> Result<FontFile, String> {
    let (index, path) = line
        .split_once('\n')
        .ok_or_else(|| format!("not a font line: {line:?}"))?;
    let index = index
        .trim()
        .parse::<u32>()
        .map_err(|error| format!("font index {index:?}: {error}"))?;
    let path = Path::new(path.trim());
    let file = File::open(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let mut bytes = Vec::new();
    file.take(FONT_BUDGET + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("{}: {error}", path.display()))?;
    if bytes.is_empty() || bytes.len() as u64 > FONT_BUDGET {
        return Err(format!(
            "{}: {} bytes, outside the font budget",
            path.display(),
            bytes.len()
        ));
    }
    Ok(FontFile { bytes, index })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_names_the_face_and_the_file() {
        let path = std::env::temp_dir().join("openpaint-font-line-test.bin");
        std::fs::write(&path, [1, 2, 3]).expect("write");
        let face = read_face(&format!("2\n{}", path.display())).expect("face");
        assert_eq!((face.index, face.bytes), (2, vec![1, 2, 3]));
        assert!(read_face("no newline").is_err());
        assert!(read_face("x\n/nowhere").is_err());
        assert!(read_face("0\n/no/such/font.ttf").is_err());
        std::fs::remove_file(path).expect("remove");
    }
}
