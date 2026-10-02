//! Artwork semantics: the only owner of layers, strokes, commands and their validation.
//!
//! Pure by rule: no GPU, no UI, no file system, no threads, no clock (`cargo repo-check`).
//! `lib.rs` only declares modules and re-exports; concepts live in their own files.
#![forbid(unsafe_code)]
