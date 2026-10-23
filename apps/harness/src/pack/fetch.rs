//! Downloads for `practice-pack`, kept in a cache (`artifacts/packs/cache/`, untracked): an
//! API's answer or a picture is fetched once and read from the cache ever after, so a rebuild
//! from the same list and cache makes the same pack without the network. Requests go one at a
//! time with a descriptive User-Agent, as Wikimedia's and the museums' API policies ask.

use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use crate::error::HarnessError;

/// Who is asking, and where to find out more.
pub const USER_AGENT: &str = concat!(
    "OpenPaint-practice-pack/",
    env!("CARGO_PKG_VERSION"),
    " (https://github.com/zhangyushao0/openpaint; practice pictures from open-access collections)"
);
/// The largest download taken: a museum's print picture or a Commons original.
const MOST_BYTES: u64 = 120 << 20;
/// A pause after every request that reached the network.
const PAUSE: Duration = Duration::from_millis(150);

/// Downloads through a cache directory.
pub struct Fetcher {
    cache: PathBuf,
    /// `None`: the cache only (tests), a missing entry is an error.
    agent: Option<ureq::Agent>,
}

impl Fetcher {
    /// Downloads cached in `cache`, fetched from the network when missing.
    pub fn new(cache: impl Into<PathBuf>) -> Self {
        let agent = ureq::Agent::config_builder()
            .user_agent(USER_AGENT)
            .timeout_global(Some(Duration::from_secs(180)))
            .build()
            .into();
        Self {
            cache: cache.into(),
            agent: Some(agent),
        }
    }

    /// The cache in `cache` alone: nothing is fetched.
    #[cfg(test)]
    pub fn cache_only(cache: impl Into<PathBuf>) -> Self {
        Self {
            cache: cache.into(),
            agent: None,
        }
    }

    /// The bytes at `url`, kept in the cache as `name` (a path relative to the cache).
    ///
    /// # Errors
    /// `Fetch` when the download fails or, without the network, the cache lacks it; `Io` when
    /// the cache cannot be written.
    pub fn get(&self, name: &str, url: &str) -> Result<Vec<u8>, HarnessError> {
        let path = self.cache.join(name);
        if let Ok(bytes) = fs::read(&path) {
            return Ok(bytes);
        }
        let failed = |detail: String| HarnessError::Fetch {
            url: url.to_owned(),
            detail,
        };
        let agent = self
            .agent
            .as_ref()
            .ok_or_else(|| failed(format!("not in the cache as {name}")))?;
        // The Met's picture URLs may hold spaces, which a request line cannot.
        let bytes = agent
            .get(url.replace(' ', "%20"))
            .call()
            .and_then(|mut answer| {
                answer
                    .body_mut()
                    .with_config()
                    .limit(MOST_BYTES)
                    .read_to_vec()
            })
            .map_err(|error| failed(error.to_string()))?;
        std::thread::sleep(PAUSE);
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let part = path.with_extension("part");
        fs::write(&part, &bytes)?;
        fs::rename(&part, &path)?;
        Ok(bytes)
    }

    /// The JSON answer at `url`, cached as `name`.
    ///
    /// # Errors
    /// As `get`; `Fetch` when the answer is not JSON (the cached file stays for a look).
    pub fn json(&self, name: &str, url: &str) -> Result<serde_json::Value, HarnessError> {
        let bytes = self.get(name, url)?;
        serde_json::from_slice(&bytes).map_err(|error| HarnessError::Fetch {
            url: url.to_owned(),
            detail: format!("not JSON ({error}); cached as {name}"),
        })
    }
}

/// A cache file's name for `object` (a museum's number, a Commons file's title) of `source`,
/// ending `extension`: its safe characters and a hash of the whole, so names never collide.
pub fn cache_name(source: &str, object: &str, extension: &str) -> String {
    let safe: String = object
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .take(60)
        .collect();
    format!("{source}/{safe}-{:08x}.{extension}", fnv(object) as u32)
}

/// FNV-1a over the bytes of `text`.
pub fn fnv(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// `text` for a URL's query: unreserved characters as they are, every other byte `%XX`.
pub fn query_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.~".contains(&byte) {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}
