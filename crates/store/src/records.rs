//! Format 1 (DECISIONS D95): the keys and values the library stores. Integers and floats are
//! little-endian inside values; keys are big-endian, so a project's records sort together and a
//! layer's in drawing order. No serialization crate (`codec.rs`).
//!
//! Keyspace `library`: `format` -> the format of every record (u32, 1); `next` -> the next unused
//! project id (u64); `clock` -> the last value of the change counter (u64); `last` -> the project
//! open when the app was left (u64), absent when none was; `editor` -> the toolbox
//! (`editor.rs`), absent until it first changed; `['a'][project u64]` -> the project's catalogue
//! entry (`catalogue.rs`); `['v'][project u64][version u64]` -> when a version was kept
//! (`versions.rs`).
//!
//! Keyspace `projects`:
//! - `[project u64]['h']` -> the head: the canvas's width and height (u32), the next layer id and
//!   the next stroke id (u64), the save id (u64: a new random one with every save, 0 none,
//!   `SaveId`); the layers bottom to top, a count (u32) and per layer its id (u64), name (u32
//!   length + UTF-8), group (u64, 0 none), visible, locked, alpha locked (u8 each), opacity (f32),
//!   blend (u8: 0 normal, 1 multiply, 2 screen, 3 overlay), clipping (u8), its stroke and edit
//!   counts (u32) and its mask (`write_layer_mask`); the groups oldest first, a count and per
//!   group its id, name, visible and opacity; last the active layer (u64) and the target (u8: 0
//!   its paint, 1 its mask);
//! - `[project u64]['s'][slot u64][index u64]` -> a stroke (`strokes.rs`), oldest at index 0;
//! - `[project u64]['e'][slot u64][index u64]` -> a region edit (`edits.rs`): a fill, cleared
//!   pixels, a transform, an adjustment or a merged layer;
//! - `[project u64]['t']` -> the gallery picture (`catalogue.rs`), absent until first left;
//! - `[project u64]['v'][version u64]` -> a kept version, an OpenPaint file (`versions.rs`).
//!
//! A slot is a layer's id for its paint, `mask_slot(id)` for its mask, and a merged layer's id
//! (a nested record, `nested.rs`) for its content, so all of them are saved, read and removed
//! alike. A save rewrites the head and only the stroke and edit records that changed.

use openpaint_app::{PortError, ProjectId, SaveId};
use openpaint_core::{Artwork, BlendMode, GroupParts, LayerMask, LayerMaskParts, LayerParts};

use crate::codec::{Reader, Writer};

/// The format this build writes, and the only one it reads.
pub const FORMAT: u32 = 1;

pub(crate) const LIBRARY: &str = "library";
pub(crate) const PROJECTS: &str = "projects";
pub(crate) const FORMAT_KEY: &[u8] = b"format";
pub(crate) const LAST: &[u8] = b"last";
pub(crate) const NEXT: &[u8] = b"next";
pub(crate) const CLOCK: &[u8] = b"clock";
/// The first byte of every catalogue entry's key.
pub(crate) const ENTRY: &[u8] = b"a";

const HEAD: u8 = b'h';
const STROKE: u8 = b's';
const THUMBNAIL: u8 = b't';
/// Where the save id lies in a head: after the canvas's size and the two id counters.
const SAVE_ID_AT: usize = 4 + 4 + 8 + 8;
/// Bytes of a layer entry in the head without its name and its mask's counts.
const LAYER_BYTES: usize = 8 + 4 + 8 + 1 + 1 + 1 + 4 + 1 + 1 + 4 + 4 + 1;
/// Bytes of a group entry in the head without its name.
const GROUP_BYTES: usize = 8 + 4 + 1 + 4;
/// A mask's records sit under its layer's id with this bit set (layer ids stay below 2^53).
const MASK_SLOT: u64 = 1 << 63;

/// The slot a layer's (or a merged layer's) mask records are keyed by.
pub(crate) const fn mask_slot(layer: u64) -> u64 {
    layer | MASK_SLOT
}

/// The prefix of every record of `project` in `projects`.
pub(crate) fn project_prefix(project: ProjectId) -> Vec<u8> {
    project.get().to_be_bytes().to_vec()
}

fn project_key(project: ProjectId, kind: u8) -> Vec<u8> {
    let mut key = project_prefix(project);
    key.push(kind);
    key
}

pub(crate) fn head_key(project: ProjectId) -> Vec<u8> {
    project_key(project, HEAD)
}

pub(crate) fn thumbnail_key(project: ProjectId) -> Vec<u8> {
    project_key(project, THUMBNAIL)
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
    project_key(project, STROKE)
}

/// The prefix of the stroke records of one slot, in drawing order.
pub(crate) fn slot_strokes_prefix(project: ProjectId, slot: u64) -> Vec<u8> {
    let mut key = strokes_prefix(project);
    key.extend_from_slice(&slot.to_be_bytes());
    key
}

pub(crate) fn stroke_key(project: ProjectId, slot: u64, index: usize) -> Vec<u8> {
    let mut key = slot_strokes_prefix(project, slot);
    key.extend_from_slice(&(index as u64).to_be_bytes());
    key
}

/// The index at the end of a stroke or edit key.
pub(crate) fn record_index(key: &[u8]) -> Option<u64> {
    let (_, index) = key.split_last_chunk::<8>()?;
    Some(u64::from_be_bytes(*index))
}

pub(crate) fn encode_id(id: u64) -> Vec<u8> {
    id.to_le_bytes().to_vec()
}

pub(crate) fn decode_id(bytes: &[u8], what: &'static str) -> Result<u64, PortError> {
    let mut reader = Reader::new(bytes, what);
    let id = reader.u64()?;
    reader.end()?;
    Ok(id)
}

pub(crate) fn encode_format() -> Vec<u8> {
    FORMAT.to_le_bytes().to_vec()
}

/// The format a library's `format` record names; a record of another length names none.
pub(crate) fn decode_format(bytes: &[u8]) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.try_into().ok()?))
}

/// The head of a stored artwork: everything but its strokes and edits, plus how many each slot
/// holds.
#[derive(Debug)]
pub(crate) struct Head {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) next_layer_id: u64,
    pub(crate) next_stroke_id: u64,
    /// The save that wrote it; `None` for a head written by none (created, imported).
    pub(crate) saved: Option<SaveId>,
    /// Bottom to top, strokes and edits still empty (a mask's too), with how many strokes and
    /// edits each has and its mask has (0 without one).
    pub(crate) layers: Vec<(LayerParts, usize, usize, [usize; 2])>,
    pub(crate) groups: Vec<GroupParts>,
    pub(crate) active: u64,
    pub(crate) on_mask: bool,
}

/// A blend mode's byte.
pub(crate) fn blend_byte(blend: BlendMode) -> u8 {
    match blend {
        BlendMode::Normal => 0,
        BlendMode::Multiply => 1,
        BlendMode::Screen => 2,
        BlendMode::Overlay => 3,
    }
}

pub(crate) fn read_blend(reader: &mut Reader<'_>) -> Result<BlendMode, PortError> {
    match reader.u8()? {
        0 => Ok(BlendMode::Normal),
        1 => Ok(BlendMode::Multiply),
        2 => Ok(BlendMode::Screen),
        3 => Ok(BlendMode::Overlay),
        _ => Err(reader.corrupt()),
    }
}

/// The head of `artwork` written by the save `saved` (`None`: by no save, 0).
pub(crate) fn encode_head(artwork: &Artwork, saved: Option<SaveId>) -> Vec<u8> {
    let layers = artwork.layers().len() * (LAYER_BYTES + 24);
    let groups = artwork.groups().len() * (GROUP_BYTES + 16);
    let mut writer = Writer::with_capacity(48 + layers + groups);
    let canvas = artwork.canvas();
    writer.u32(canvas.width());
    writer.u32(canvas.height());
    writer.u64(artwork.next_layer_id());
    writer.u64(artwork.next_stroke_id());
    writer.u64(saved.map_or(0, SaveId::get));
    writer.len(artwork.layers().len());
    for layer in artwork.layers() {
        let props = layer.props();
        writer.u64(layer.id().get());
        writer.str(layer.name().as_str());
        writer.u64(props.group().map_or(0, |group| group.get()));
        writer.bool(props.visible());
        writer.bool(props.locked());
        writer.bool(props.alpha_locked());
        writer.f32(props.opacity());
        writer.u8(blend_byte(props.blend()));
        writer.bool(props.clipping());
        writer.len(layer.strokes().len());
        writer.len(layer.edits().len());
        write_layer_mask(&mut writer, layer.mask());
    }
    writer.len(artwork.groups().len());
    for group in artwork.groups() {
        writer.u64(group.id().get());
        writer.str(group.name().as_str());
        writer.bool(group.visible());
        writer.f32(group.opacity());
    }
    writer.u64(artwork.active().get());
    writer.bool(artwork.on_mask());
    writer.finish()
}

/// `head` as an OpenPaint file holds it: its save id 0, every other byte as it was. A save id
/// names a save of one library (the display cache's key), never a file's content.
pub(crate) fn unsaved(mut head: Vec<u8>) -> Vec<u8> {
    if let Some(saved) = head.get_mut(SAVE_ID_AT..SAVE_ID_AT + 8) {
        saved.fill(0);
    }
    head
}

/// A mask as the head (and a nested record) stores it: none (u8 0), or flags (u8: 1, plus 2
/// enabled, plus 4 inverted) followed by its stroke and edit counts (u32).
pub(crate) fn write_layer_mask(writer: &mut Writer, mask: Option<&LayerMask>) {
    match mask {
        Some(mask) => {
            let flags = 1 | u8::from(mask.enabled()) << 1 | u8::from(mask.inverted()) << 2;
            writer.u8(flags);
            writer.len(mask.strokes().len());
            writer.len(mask.edits().len());
        }
        None => writer.u8(0),
    }
}

/// A mask as `write_layer_mask` stores it, and its stroke and edit counts.
pub(crate) fn read_layer_mask(
    reader: &mut Reader<'_>,
) -> Result<(Option<LayerMaskParts>, [usize; 2]), PortError> {
    let flags = reader.u8()?;
    if flags == 0 {
        return Ok((None, [0; 2]));
    }
    if flags & 1 == 0 || flags > 7 {
        return Err(reader.corrupt());
    }
    let counts = [reader.u32()? as usize, reader.u32()? as usize];
    let mask = LayerMaskParts {
        enabled: flags & 2 != 0,
        inverted: flags & 4 != 0,
        strokes: Vec::new(),
        edits: Vec::new(),
    };
    Ok((Some(mask), counts))
}

pub(crate) fn decode_head(bytes: &[u8]) -> Result<Head, PortError> {
    let mut reader = Reader::new(bytes, "artwork head");
    let (width, height) = (reader.u32()?, reader.u32()?);
    let (next_layer_id, next_stroke_id) = (reader.u64()?, reader.u64()?);
    let saved = SaveId::new(reader.u64()?);
    let count = reader.count(LAYER_BYTES)?;
    let mut layers = Vec::with_capacity(count);
    for _ in 0..count {
        layers.push(read_layer(&mut reader)?);
    }
    let groups = read_groups(&mut reader)?;
    let (active, on_mask) = (reader.u64()?, reader.bool()?);
    reader.end()?;
    Ok(Head {
        width,
        height,
        next_layer_id,
        next_stroke_id,
        saved,
        layers,
        groups,
        active,
        on_mask,
    })
}

fn read_layer(
    reader: &mut Reader<'_>,
) -> Result<(LayerParts, usize, usize, [usize; 2]), PortError> {
    let (id, name, group) = (reader.u64()?, reader.str()?, reader.u64()?);
    let (visible, locked, alpha_locked) = (reader.bool()?, reader.bool()?, reader.bool()?);
    let (opacity, blend, clipping) = (reader.f32()?, read_blend(reader)?, reader.bool()?);
    let (strokes, edits) = (reader.u32()? as usize, reader.u32()? as usize);
    let (mask, masked) = read_layer_mask(reader)?;
    let layer = LayerParts {
        id,
        name,
        visible,
        opacity,
        locked,
        alpha_locked,
        clipping,
        blend,
        group: (group != 0).then_some(group),
        strokes: Vec::new(),
        edits: Vec::new(),
        mask,
    };
    Ok((layer, strokes, edits, masked))
}

fn read_groups(reader: &mut Reader<'_>) -> Result<Vec<GroupParts>, PortError> {
    let count = reader.count(GROUP_BYTES)?;
    let mut groups = Vec::with_capacity(count);
    for _ in 0..count {
        let (id, name) = (reader.u64()?, reader.str()?);
        let (visible, opacity) = (reader.bool()?, reader.f32()?);
        groups.push(GroupParts {
            id,
            name,
            visible,
            opacity,
        });
    }
    Ok(groups)
}
