//! The app's version (S52): semantic, from the workspace manifest's `[workspace.package]
//! version` (which every crate, xtask too, carries as `CARGO_PKG_VERSION`), with the library's
//! schema (`SCHEMA` in `crates/store/src/records.rs`, its owner) as build metadata:
//! `versionName` "0.7.0+schema22", `versionCode` major x 10,000 + minor x 100 + patch. The
//! Android build hands both to Gradle (`android.rs`); the app shows the same two from its own
//! compiled constants. Policy: a minor per stage, a patch per later slice, 1.0.0 at G7's exit.

use std::path::Path;

/// Where the library's schema is defined, and the line that defines it.
const SCHEMA_FILE: &str = "crates/store/src/records.rs";
const SCHEMA_LINE: &str = "pub const SCHEMA: u32 = ";

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

    /// Android's `versionName`: the version with the library's schema as build metadata.
    pub fn name(self, schema: u32) -> String {
        format!(
            "{}.{}.{}+schema{schema}",
            self.major, self.minor, self.patch
        )
    }
}

/// The library's schema as its source under `root` defines it.
pub fn schema(root: &Path) -> Result<u32, String> {
    let path = root.join(SCHEMA_FILE);
    let text =
        std::fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    schema_in(&text).ok_or_else(|| format!("{}: no line `{SCHEMA_LINE}N;`", path.display()))
}

fn schema_in(text: &str) -> Option<u32> {
    text.lines()
        .find_map(|line| line.trim().strip_prefix(SCHEMA_LINE))
        .and_then(|rest| rest.strip_suffix(';'))
        .and_then(|number| number.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_version_code_and_name_follow_the_policy() {
        let version = Version::parse("0.7.0").expect("0.7.0");
        assert_eq!(version.code(), 700);
        assert_eq!(version.name(22), "0.7.0+schema22");
        assert_eq!(Version::parse("1.0.0").map(Version::code), Ok(10_000));
        assert_eq!(Version::parse("1.2.3").map(Version::code), Ok(10_203));
        assert!(Version::parse("0.7").is_err() && Version::parse("0.7.0.1").is_err());
        assert!(Version::parse("0.100.0").is_err() && Version::parse("0.7.100").is_err());
        assert!(Version::parse("0.7.0+schema22").is_err() && Version::parse("v1.0.0").is_err());
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
    fn this_workspace_has_a_version_and_the_store_a_schema() {
        assert!(Version::current().is_ok(), "{}", env!("CARGO_PKG_VERSION"));
        let schema = schema(&crate::process::root()).expect("the store's schema");
        assert!(schema >= 22, "{schema}");
        assert_eq!(schema_in("/// x\npub const SCHEMA: u32 = 23;\n"), Some(23));
        assert_eq!(schema_in("pub const SCHEMA: u32 = 2x;"), None);
        assert_eq!(schema_in("const SCHEMA: u32 = 4;"), None);
    }
}
