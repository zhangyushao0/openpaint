//! Schema 9: the keys and values the library stores (DECISIONS D21, D25, D28, D29, D36, D38,
//! D43, D45, D46).
//!
//! Keyspace `library`: `schema` -> the schema of every record (u32 LE); `next` -> the next
//! unused project id (u64 LE); `clock` -> the last value of the change counter (u64 LE); `last`
//! -> the project open when the app was left (u64 LE), absent when none was; `['a'][project
//! u64]` -> the project's catalogue entry (`catalogue.rs`): title and when it last changed;
//! `editor` -> the toolbox (`editor.rs`), absent until the user first changed it.
//!
//! Keyspace `projects`, keys big-endian so a project's records sort together:
//! - `[project u64]['h']` -> the head: schema (u32), canvas width and height (u32), active
//!   layer, next layer id and next stroke id (u64), then the layers bottom to top, each id
//!   (u64), name (u32 length + UTF-8), visible (u8), opacity (f32), locked (u8), alpha locked
//!   (u8), its stroke count (u32) and its edit count (u32);
//! - `[project u64]['s'][layer u64][index u64]` -> one stroke, oldest at index 0 (`strokes.rs`):
//!   id (u64), size, opacity, red, green, blue (f32), blend (u8: 0 paint over, 1 paint atop
//!   under Alpha Lock, 2 erase, 3 smudge, 4 smudge under Alpha Lock), its brush recipe (softness (f32); the curves size over
//!   pressure, size over speed, opacity over pressure, opacity over speed, each a point count
//!   (u32) and per point x, y (f32); grain strength (f32) and scale (u32); the deposit (u8: 0
//!   cover, 1 accumulate), when accumulating its tip (u8: 0 flat, 1 bell), flow and maximum
//!   density (f32) and paper (u8: 0 none, 1 sketch), on paper its contact curve, scale, depth
//!   and dust (f32)), sample count (u32), then per sample x, y, pressure, tilt, azimuth (f32)
//!   and time (u32 ms), then its clip (u8: 0 none, 1 a mask as a fill stores it: rectangle,
//!   run count and runs, `fills.rs`);
//! - `[project u64]['f'][layer u64][index u64]` -> one fill or transform, oldest at index 0
//!   (`fills.rs`);
//! - `[project u64]['t']` -> the gallery thumbnail (`catalogue.rs`), derived from the artwork
//!   when it was last left; absent until then.
//!
//! Integers and floats are little-endian inside values. A save rewrites the head and only the
//! stroke and fill records that changed. Schema 1 (S7) had no `schema`, `clock`, catalogue
//! entries or thumbnails, and `last` named the project saved last; schema 2 (S10) had no recipe
//! in its strokes and no `editor`; schema 3 (S11) had no alpha lock, no blend and a toolbox of
//! one tool without colours; schema 4 (S12) had no fills; schema 5 (S15) had no deposit in its
//! recipes and no StreamLine in its toolbox; schema 6 (S16) had no clip in its strokes; schema
//! 7 (S18) had fills only, without a kind; schema 8 (S20) had no smudge in its strokes or its
//! toolbox; `migrate.rs`, `migrate_6.rs`, `migrate_7.rs`, `migrate_8.rs` and `migrate_9.rs`
//! upgrade them.

use openpaint_app::{PortError, ProjectId};
use openpaint_core::{Artwork, LayerParts};

use crate::codec::{Reader, Writer};
pub(crate) use crate::strokes::{
    STROKE_STYLE_BYTES, decode_stroke, encode_stroke, read_curve, write_recipe,
};

/// The schema this build writes and the newest it reads.
pub const SCHEMA: u32 = 9;

pub(crate) const LIBRARY: &str = "library";
pub(crate) const PROJECTS: &str = "projects";
pub(crate) const SCHEMA_KEY: &[u8] = b"schema";
pub(crate) const LAST: &[u8] = b"last";
pub(crate) const NEXT: &[u8] = b"next";
pub(crate) const CLOCK: &[u8] = b"clock";
/// The first byte of every catalogue entry's key.
pub(crate) const ENTRY: &[u8] = b"a";

const HEAD: u8 = b'h';
const STROKE: u8 = b's';
const THUMBNAIL: u8 = b't';
/// Bytes of a layer entry in the head without its name.
const LAYER_BYTES: usize = 8 + 4 + 1 + 4 + 1 + 1 + 4 + 4;

/// The prefix of every record of `project` in `projects`.
pub(crate) fn project_prefix(project: ProjectId) -> Vec<u8> {
    project.get().to_be_bytes().to_vec()
}

pub(crate) fn head_key(project: ProjectId) -> Vec<u8> {
    let mut key = project_prefix(project);
    key.push(HEAD);
    key
}

pub(crate) fn thumbnail_key(project: ProjectId) -> Vec<u8> {
    let mut key = project_prefix(project);
    key.push(THUMBNAIL);
    key
}

/// The key of `project`'s catalogue entry in `library`.
pub(crate) fn entry_key(project: ProjectId) -> Vec<u8> {
    let mut key = ENTRY.to_vec();
    key.extend_from_slice(&project.get().to_be_bytes());
    key
}

/// The project a catalogue entry's key names.
pub(crate) fn entry_project(key: &[u8]) -> Option<ProjectId> {
    let id = key.strip_prefix(ENTRY)?;
    Some(ProjectId::new(u64::from_be_bytes(id.try_into().ok()?)))
}

/// The prefix of every stroke record of `project`.
pub(crate) fn strokes_prefix(project: ProjectId) -> Vec<u8> {
    let mut key = project_prefix(project);
    key.push(STROKE);
    key
}

/// The prefix of the stroke records of one layer, in drawing order.
pub(crate) fn layer_prefix(project: ProjectId, layer: u64) -> Vec<u8> {
    let mut key = strokes_prefix(project);
    key.extend_from_slice(&layer.to_be_bytes());
    key
}

pub(crate) fn stroke_key(project: ProjectId, layer: u64, index: usize) -> Vec<u8> {
    let mut key = layer_prefix(project, layer);
    key.extend_from_slice(&(index as u64).to_be_bytes());
    key
}

/// The index at the end of a stroke key under a layer prefix.
pub(crate) fn stroke_index(key: &[u8]) -> Option<u64> {
    let (_, index) = key.split_last_chunk::<8>()?;
    Some(u64::from_be_bytes(*index))
}

pub(crate) fn encode_id(id: u64) -> Vec<u8> {
    id.to_le_bytes().to_vec()
}

pub(crate) fn encode_schema(schema: u32) -> Vec<u8> {
    schema.to_le_bytes().to_vec()
}

pub(crate) fn decode_schema(bytes: &[u8]) -> Result<u32, PortError> {
    let mut reader = Reader::new(bytes, "library schema");
    let schema = reader.u32()?;
    reader.end()?;
    Ok(schema)
}

pub(crate) fn decode_id(bytes: &[u8], what: &'static str) -> Result<u64, PortError> {
    let mut reader = Reader::new(bytes, what);
    let id = reader.u64()?;
    reader.end()?;
    Ok(id)
}

/// The head of a stored artwork: everything but the strokes, plus each layer's stroke count.
#[derive(Debug)]
pub(crate) struct Head {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) active: u64,
    pub(crate) next_layer_id: u64,
    pub(crate) next_stroke_id: u64,
    /// Bottom to top, strokes and fills still empty, with how many strokes and fills each has.
    pub(crate) layers: Vec<(LayerParts, usize, usize)>,
}

pub(crate) fn encode_head(artwork: &Artwork) -> Vec<u8> {
    let mut writer = Writer::with_capacity(48 + artwork.layers().len() * (LAYER_BYTES + 16));
    writer.u32(SCHEMA);
    let canvas = artwork.canvas();
    writer.u32(canvas.width());
    writer.u32(canvas.height());
    writer.u64(artwork.active().get());
    writer.u64(artwork.next_layer_id());
    writer.u64(artwork.next_stroke_id());
    writer.len(artwork.layers().len());
    for layer in artwork.layers() {
        writer.u64(layer.id().get());
        writer.str(layer.name().as_str());
        writer.bool(layer.props().visible());
        writer.f32(layer.props().opacity());
        writer.bool(layer.props().locked());
        writer.bool(layer.props().alpha_locked());
        writer.len(layer.strokes().len());
        writer.len(layer.edits().len());
    }
    writer.finish()
}

pub(crate) fn decode_head(bytes: &[u8]) -> Result<Head, PortError> {
    let mut reader = Reader::new(bytes, "artwork head");
    let schema = reader.u32()?;
    if schema > SCHEMA {
        return Err(PortError::NewerSchema {
            found: schema,
            newest: SCHEMA,
        });
    }
    if schema != SCHEMA {
        return Err(reader.corrupt());
    }
    let (width, height) = (reader.u32()?, reader.u32()?);
    let (active, next_layer_id, next_stroke_id) = (reader.u64()?, reader.u64()?, reader.u64()?);
    let count = reader.count(LAYER_BYTES)?;
    let mut layers = Vec::with_capacity(count);
    for _ in 0..count {
        let id = reader.u64()?;
        let name = reader.str()?;
        let (visible, opacity, locked) = (reader.bool()?, reader.f32()?, reader.bool()?);
        let alpha_locked = reader.bool()?;
        let (strokes, fills) = (reader.u32()? as usize, reader.u32()? as usize);
        let layer = LayerParts {
            id,
            name,
            visible,
            opacity,
            locked,
            alpha_locked,
            strokes: Vec::new(),
            edits: Vec::new(),
        };
        layers.push((layer, strokes, fills));
    }
    reader.end()?;
    Ok(Head {
        width,
        height,
        active,
        next_layer_id,
        next_stroke_id,
        layers,
    })
}
