//! The app's version (DECISIONS D79): semantic, from the workspace manifest's
//! `[workspace.package] version` (which every crate, xtask too, carries as `CARGO_PKG_VERSION`),
//! with the storage format (`FORMAT` in `crates/store/src/records.rs`, its owner) as build
//! metadata: `versionName` "1.0.0+format1", `versionCode` major x 10,000 + minor x 100 + patch.
//! The Android build hands both to Gradle (`android.rs`); the app shows the same two from its own
//! compiled constants.

use std::path::Path;

/// Where the storage format is defined, and the line that defines it.
const FORMAT_FILE: &str = "crates/store/src/records.rs";
const FORMAT_LINE: &str = "pub const FORMAT: u32 = ";

/// A semantic version without pre-release or build parts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    /// "major.minor.patch"; minor and patch below 100, so the version code stays ordered.
    pub fn parse(text: &str) -> Result<Self, String> {
        let parts: Vec<u32> = text
            .split('.')
            .map(|part| part.parse().map_err(|_| format!("not a version: {text}")))
            .collect::<Result<_, _>>()?;
        let [major, minor, patch] = parts[..] else {
            return Err(format!("not major.minor.patch: {text}"));
        };
        if minor >= 100 || patch >= 100 {
            return Err(format!(
                "{text}: minor and patch stay below 100 (the version code)"
            ));
        }
        Ok(Self {
            major,
            minor,
            patch,
        })
    }

    /// This workspace's version, as its manifest says.
    pub fn current() -> Result<Self, String> {
        Self::parse(env!("CARGO_PKG_VERSION"))
    }

    /// Android's `versionCode`: major x 10,000 + minor x 100 + patch.
    pub const fn code(self) -> u32 {
        self.major * 10_000 + self.minor * 100 + self.patch
    }

    /// Android's `versionName`: the version with the storage format as build metadata.
    pub fn name(self, format: u32) -> String {
        format!(
            "{}.{}.{}+format{format}",
            self.major, self.minor, self.patch
        )
    }
}

/// The storage format as its source under `root` defines it.
pub fn format(root: &Path) -> Result<u32, String> {
    let path = root.join(FORMAT_FILE);
    let text =
        std::fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    format_in(&text).ok_or_else(|| format!("{}: no line `{FORMAT_LINE}N;`", path.display()))
}

fn format_in(text: &str) -> Option<u32> {
    text.lines()
        .find_map(|line| line.trim().strip_prefix(FORMAT_LINE))
        .and_then(|rest| rest.strip_suffix(';'))
        .and_then(|number| number.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_version_code_and_name_follow_the_policy() {
        let version = Version::parse("0.8.0").expect("0.8.0");
        assert_eq!(version.code(), 800);
        assert_eq!(version.name(1), "0.8.0+format1");
        assert_eq!(Version::parse("1.0.0").map(Version::code), Ok(10_000));
        assert_eq!(Version::parse("1.2.3").map(Version::code), Ok(10_203));
        assert!(Version::parse("0.7").is_err() && Version::parse("0.7.0.1").is_err());
        assert!(Version::parse("0.100.0").is_err() && Version::parse("0.7.100").is_err());
        assert!(Version::parse("0.8.0+format1").is_err() && Version::parse("v1.0.0").is_err());
        // Every later version has a larger code.
        let order = [
            "0.7.0", "0.7.1", "0.7.99", "0.8.0", "1.0.0", "1.0.1", "2.0.0",
        ];
        let codes: Vec<u32> = order
            .iter()
            .map(|text| Version::parse(text).expect("a version").code())
            .collect();
        assert!(codes.windows(2).all(|pair| pair[0] < pair[1]), "{codes:?}");
    }

    #[test]
    fn this_workspace_has_a_version_and_the_store_a_format() {
        assert!(Version::current().is_ok(), "{}", env!("CARGO_PKG_VERSION"));
        let format = format(&crate::process::root()).expect("the store's format");
        assert_eq!(format, 1);
        assert_eq!(format_in("/// x\npub const FORMAT: u32 = 2;\n"), Some(2));
        assert_eq!(format_in("pub const FORMAT: u32 = 2x;"), None);
        assert_eq!(format_in("const FORMAT: u32 = 4;"), None);
    }
}
