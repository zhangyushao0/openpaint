//! Smudge (DECISIONS D46, after draweave's ADR-0049): a smudge stroke carries its own layer's
//! colour along its path. Its contacts (`dabs.rs`, one at most a pixel apart) each move a share
//! `w` of the colour a contact's step back to where it is: `F <- (1 - w) F(p) + w F(p - d)`.
//! A pixel's result then depends on its neighbours, so a smudge is no per-pixel op of its own:
//! each one is evaluated once, in order, into a residual on canvas-pixel nodes, `after = A x
//! before + R` (`residual.rs`, tiles allocated as its contacts reach them), from the layer's
//! pixels just before it (`material.rs`, the single evaluator on the layer's earlier ops). The
//! evaluator then applies that residual where the stroke lies (opcode `Smudge`).
//!
//! The in-progress smudge extends its residual by the contacts its settled spans add, with the
//! very contacts and tiles a cold evaluation of the same samples applies, so both are bit for
//! bit equal; on commit the residual moves into the target's arena (`arena.rs`) unchanged. A
//! residual is derived and owned by its canvas target (D17); it is rebuilt from the artwork
//! whenever it is missing (another target, a reopened artwork, a redo after it was dropped).
//!
//! The residuals share their arena with the filters' results (S33, `filter.rs`), prepared with
//! them in scene order, as each may read the other's pixels.

mod arena;
mod dabs;
mod engine;
mod kernels;
mod material;
mod residual;

pub(crate) use arena::{Arena, Keep, Key};
pub(crate) use dabs::{smudge_reach, whole_reach};
pub(crate) use engine::{Smudges, Wanted, reached};
pub(crate) use kernels::{SHADER as SMUDGE_SHADER, SmudgePipeline};
pub(crate) use material::Material;

/// Side of a residual tile in nodes (one node per canvas pixel).
pub(crate) const RESIDUAL_TILE: u32 = 32;
/// Nodes in one residual tile.
pub(crate) const TILE_NODES: u32 = RESIDUAL_TILE * RESIDUAL_TILE;
/// Words of one node: the share kept `A`, then the colour carried in `R` (premultiplied RGBA).
pub(crate) const NODE_WORDS: u32 = 5;
/// Words before a residual's tile table: its first tile's column and row, and the table's width
/// and height in tiles.
pub(crate) const RESIDUAL_HEADER: u32 = 4;
