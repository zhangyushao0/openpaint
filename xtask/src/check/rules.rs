//! The numbers and tables behind `cargo repo-check`. Changing a rule is a reviewed code change,
//! not a per-file exception.

/// Longest allowed Rust source file, tests included. Split by responsibility, not by size.
pub const MAX_RUST_LINES: usize = 400;

/// The only markdown files allowed outside `docs/archive/`, with their line limits.
pub const DOCS: &[(&str, usize)] = &[
    ("README.md", 60),
    ("AGENTS.md", 150),
    ("docs/STATUS.md", 150),
    ("docs/ROADMAP.md", 250),
    ("docs/ARCHITECTURE.md", 300),
    ("docs/DECISIONS.md", 300),
];

/// Archived documents: written when a document overflows, never read, no limit.
pub const ARCHIVE_DIR: &str = "docs/archive/";

/// Every workspace crate a composition root may use.
pub const ALL_PRODUCT: [&str; 7] = [
    "openpaint-core",
    "openpaint-raster",
    "openpaint-app",
    "openpaint-store",
    "openpaint-render",
    "openpaint-ui",
    "openpaint-runtime",
];

/// Every crate directory and the workspace crates it may depend on. Dependencies point down.
pub const LAYERS: &[(&str, &[&str])] = &[
    ("crates/core", &[]),
    // Pure pixel geometry (flood fill): no GPU, no threads, no file system.
    ("crates/raster", &["openpaint-core"]),
    ("crates/app", &["openpaint-core"]),
    ("crates/store", &["openpaint-core", "openpaint-app"]),
    ("crates/render", &["openpaint-core"]),
    (
        "crates/ui",
        &["openpaint-core", "openpaint-app", "openpaint-render"],
    ),
    (
        "crates/runtime",
        &[
            "openpaint-core",
            "openpaint-raster",
            "openpaint-app",
            "openpaint-store",
            "openpaint-render",
            "openpaint-ui",
        ],
    ),
    // Diagnostics and automation: never a dependency of a product crate (DECISIONS D10).
    ("crates/devtools", &ALL_PRODUCT),
    ("apps/harness", &WITH_DEVTOOLS),
    ("apps/android", &ALL_PRODUCT),
];

/// The harness assembles the product crates and the devtools.
const WITH_DEVTOOLS: [&str; 8] = [
    "openpaint-core",
    "openpaint-raster",
    "openpaint-app",
    "openpaint-store",
    "openpaint-render",
    "openpaint-ui",
    "openpaint-runtime",
    "openpaint-devtools",
];

/// Substrings that must not appear in a crate's sources; they keep the lower layers pure.
pub const FORBIDDEN_TOKENS: &[(&str, &[&str])] = &[
    (
        "crates/core",
        &[
            "wgpu",
            "egui",
            "winit",
            "jni",
            "std::fs",
            "std::thread",
            "std::time::Instant",
            "serde",
        ],
    ),
    (
        "crates/raster",
        &[
            "wgpu",
            "egui",
            "winit",
            "jni",
            "std::fs",
            "std::thread",
            "std::time::Instant",
        ],
    ),
    ("crates/app", &["wgpu", "egui", "winit", "jni", "std::fs"]),
    ("crates/store", &["wgpu", "egui", "winit", "jni"]),
    ("crates/render", &["winit", "jni", "std::fs"]),
    ("crates/ui", &["wgpu", "winit", "jni", "std::fs"]),
    ("crates/runtime", &["jni"]),
    ("crates/devtools", &["jni", "std::fs"]),
];

/// The renderer may mention the UI framework only inside its adapter directory.
pub const RENDER_DIR: &str = "crates/render/";
pub const RENDER_EGUI_DIR: &str = "crates/render/src/adapter/";

/// Crate roots that may skip `#![forbid(unsafe_code)]`: JNI needs unsafe, and so does the
/// harness's counting global allocator (S51, never shipped; the harness denies unsafe code and
/// only `heap.rs` expects it).
pub const UNSAFE_EXEMPT: &[&str] = &["apps/android/", "apps/harness/"];

/// Paths that are never tracked: notes, measurements, captures, scratch work, extra doc trees.
pub const FORBIDDEN_PREFIXES: &[&str] = &[
    "local/",
    "artifacts/",
    "notes/",
    "tmp/",
    "scratch/",
    "scripts/",
    "docs/research/",
    "docs/decisions/",
];

/// Scripts are never tracked; tooling is Rust in `xtask`.
pub const FORBIDDEN_EXTENSIONS: &[&str] = &["py", "sh", "ps1", "bat", "cmd"];

/// Text files that must use LF line endings.
pub const TEXT_EXTENSIONS: &[&str] = &[
    "rs", "md", "toml", "yml", "yaml", "json", "wgsl", "txt", "lock",
];
