//! The numbers and tables behind `cargo repo-check`. Changing a rule is a reviewed code change,
//! not a per-file exception.

/// Longest allowed Rust source file, tests included. Split by responsibility, not by size.
pub const MAX_RUST_LINES: usize = 400;

/// A markdown file allowed outside `docs/archive/` and its limits.
pub struct Doc {
    pub path: &'static str,
    /// The most lines it may hold.
    pub lines: usize,
    /// The most characters (Unicode scalar values) it may hold: set for the four documents,
    /// which also hold no line longer than `MAX_DOC_LINE_CHARS`.
    pub chars: Option<usize>,
}

/// The only markdown files allowed outside `docs/archive/`, with their limits. STATUS and
/// ROADMAP are read whole at every session's start; ARCHITECTURE and DECISIONS by section.
pub const DOCS: &[Doc] = &[
    Doc {
        path: "README.md",
        lines: 60,
        chars: None,
    },
    Doc {
        path: "AGENTS.md",
        lines: 150,
        chars: None,
    },
    Doc {
        path: "docs/STATUS.md",
        lines: 150,
        chars: Some(40_000),
    },
    Doc {
        path: "docs/ROADMAP.md",
        lines: 250,
        chars: Some(40_000),
    },
    Doc {
        path: "docs/ARCHITECTURE.md",
        lines: 300,
        chars: Some(69_000),
    },
    Doc {
        path: "docs/DECISIONS.md",
        lines: 300,
        chars: Some(89_000),
    },
];

/// The longest line of the four documents, in characters: a fact a line, never a page.
pub const MAX_DOC_LINE_CHARS: usize = 1_500;

/// Archived documents: written when a document overflows, never read, no limit.
pub const ARCHIVE_DIR: &str = "docs/archive/";

/// Every workspace crate a composition root may use.
pub const ALL_PRODUCT: [&str; 8] = [
    "openpaint-core",
    "openpaint-practice",
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
    // Practice's generators, metrics and records: pure math, beside core (G9).
    ("crates/practice", &[]),
    // Pure pixel geometry (flood fill): no GPU, no threads, no file system.
    ("crates/raster", &["openpaint-core"]),
    ("crates/app", &["openpaint-core", "openpaint-practice"]),
    (
        "crates/store",
        &["openpaint-core", "openpaint-practice", "openpaint-app"],
    ),
    ("crates/render", &["openpaint-core"]),
    (
        "crates/ui",
        &[
            "openpaint-core",
            "openpaint-practice",
            "openpaint-app",
            "openpaint-render",
        ],
    ),
    (
        "crates/runtime",
        &[
            "openpaint-core",
            "openpaint-practice",
            "openpaint-raster",
            "openpaint-app",
            "openpaint-store",
            "openpaint-render",
            "openpaint-ui",
        ],
    ),
    // Diagnostics and automation: never a dependency of a product crate (DECISIONS D10).
    ("crates/devtools", &ALL_PRODUCT),
    // Contact sheets for the harness and the pack builder (S111); never a product crate's.
    ("crates/sheet", &[]),
    ("apps/harness", &WITH_DEVTOOLS),
    ("apps/android", &ALL_PRODUCT),
    // The practice-pack builder, a workspace of its own (S111): the pack format and its
    // computations from the crates the app reads packs with.
    (
        "tools/packs",
        &[
            "openpaint-practice",
            "openpaint-app",
            "openpaint-store",
            "openpaint-sheet",
        ],
    ),
];

/// Workspaces of their own, outside the root one and the gate, that repo-check still holds to
/// the code rules (`tools.rs`).
pub const TOOLS: &[&str] = &["tools/packs"];

/// The harness assembles the product crates, the devtools and the contact sheets.
const WITH_DEVTOOLS: [&str; 10] = [
    "openpaint-core",
    "openpaint-practice",
    "openpaint-raster",
    "openpaint-app",
    "openpaint-store",
    "openpaint-render",
    "openpaint-ui",
    "openpaint-runtime",
    "openpaint-devtools",
    "openpaint-sheet",
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
        "crates/practice",
        &[
            "wgpu",
            "egui",
            "winit",
            "jni",
            "std::fs",
            "std::thread",
            "std::time",
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

/// The interface's sources, whose words live in its text tables (DECISIONS D100).
pub const UI_SOURCES: &str = "crates/ui/src/";
pub const TEXT_TABLES: &str = "crates/ui/src/text/";

/// The renderer may mention the UI framework only inside its adapter directory.
pub const RENDER_DIR: &str = "crates/render/";
pub const RENDER_EGUI_DIR: &str = "crates/render/src/adapter/";

/// Crate roots that may skip `#![forbid(unsafe_code)]`: JNI needs unsafe, and so does the
/// harness's counting global allocator (never shipped; the harness denies unsafe code and
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
