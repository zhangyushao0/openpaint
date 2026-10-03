//! Schema 4: the keys and values the library stores (DECISIONS D21, D25, D28, D29).
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
//!   (u8) and its stroke count (u32);
//! - `[project u64]['s'][layer u64][index u64]` -> one stroke, oldest at index 0: id (u64),
//!   size, opacity, red, green, blue (f32), blend (u8: 0 paint over, 1 paint atop under Alpha
//!   Lock, 2 erase), its brush recipe (softness (f32); the curves size over pressure, size over
//!   speed, opacity over pressure, opacity over speed, each a point count (u32) and per point x,
//!   y (f32); grain strength (f32) and scale (u32)), sample count (u32), then per sample x, y,
//!   pressure, tilt, azimuth (f32) and time (u32 ms);
//! - `[project u64]['t']` -> the gallery thumbnail (`catalogue.rs`), derived from the artwork
//!   when it was last left; absent until then.
//!
//! Integers and floats are little-endian inside values. A save rewrites the head and only the
//! stroke records that changed. Schema 1 (S7) had no `schema`, `clock`, catalogue entries or
//! thumbnails, and `last` named the project saved last; schema 2 (S10) had no recipe in its
//! strokes and no `editor`; schema 3 (S11) had no alpha lock, no blend and a toolbox of one
//! tool without colours; `migrate.rs` upgrades them.

use openpaint_app::{PortError, ProjectId};
use openpaint_core::{
    Artwork, LayerParts, RecipeParts, SampleParts, Stroke, StrokeBlend, StrokeParts,
};

use crate::codec::{Reader, Writer};

/// The schema this build writes and the newest it reads.
pub const SCHEMA: u32 = 4;

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
/// Bytes of one encoded sample.
const SAMPLE_BYTES: usize = 24;
/// Bytes of a stroke record before its blend: id, size, opacity and colour.
pub(crate) const STROKE_STYLE_BYTES: usize = 8 + 5 * 4;
/// Bytes of one curve point.
const POINT_BYTES: usize = 8;
/// Bytes of a layer entry in the head without its name.
const LAYER_BYTES: usize = 8 + 4 + 1 + 4 + 1 + 1 + 4;

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
    /// Bottom to top, strokes still empty, with how many each has.
    pub(crate) layers: Vec<(LayerParts, usize)>,
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
        let strokes = reader.u32()? as usize;
        let layer = LayerParts {
            id,
            name,
            visible,
            opacity,
            locked,
            alpha_locked,
            strokes: Vec::new(),
        };
        layers.push((layer, strokes));
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

pub(crate) fn encode_stroke(stroke: &Stroke) -> Vec<u8> {
    let samples = stroke.samples();
    let mut writer = Writer::with_capacity(32 + samples.len() * SAMPLE_BYTES);
    writer.u64(stroke.id().get());
    let style = stroke.style();
    writer.f32(style.size());
    writer.f32(style.opacity());
    let color = style.color();
    for channel in [color.r(), color.g(), color.b()] {
        writer.f32(channel);
    }
    writer.u8(match stroke.blend() {
        StrokeBlend::Over => 0,
        StrokeBlend::Atop => 1,
        StrokeBlend::Erase => 2,
    });
    write_recipe(&mut writer, &style.recipe().to_parts());
    writer.len(samples.len());
    for sample in samples {
        let position = sample.position();
        for value in [
            position.x(),
            position.y(),
            sample.pressure(),
            sample.tilt(),
            sample.azimuth(),
        ] {
            writer.f32(value);
        }
        writer.u32(sample.time_ms());
    }
    writer.finish()
}

pub(crate) fn decode_stroke(bytes: &[u8]) -> Result<StrokeParts, PortError> {
    let mut reader = Reader::new(bytes, "stroke");
    let id = reader.u64()?;
    let (size, opacity) = (reader.f32()?, reader.f32()?);
    let color = [reader.f32()?, reader.f32()?, reader.f32()?];
    let blend = match reader.u8()? {
        0 => StrokeBlend::Over,
        1 => StrokeBlend::Atop,
        2 => StrokeBlend::Erase,
        _ => return Err(reader.corrupt()),
    };
    let recipe = read_recipe(&mut reader)?;
    let count = reader.count(SAMPLE_BYTES)?;
    let mut samples = Vec::with_capacity(count);
    for _ in 0..count {
        samples.push(SampleParts {
            x: reader.f32()?,
            y: reader.f32()?,
            pressure: reader.f32()?,
            tilt: reader.f32()?,
            azimuth: reader.f32()?,
            time_ms: reader.u32()?,
        });
    }
    reader.end()?;
    Ok(StrokeParts {
        id,
        blend,
        recipe,
        size,
        opacity,
        color,
        samples,
    })
}

/// A recipe as a stroke record holds it, after the colour.
pub(crate) fn write_recipe(writer: &mut Writer, recipe: &RecipeParts) {
    writer.f32(recipe.softness);
    for curve in [
        &recipe.size_pressure,
        &recipe.size_speed,
        &recipe.opacity_pressure,
        &recipe.opacity_speed,
    ] {
        writer.len(curve.len());
        for [x, y] in curve {
            writer.f32(*x);
            writer.f32(*y);
        }
    }
    writer.f32(recipe.grain_strength);
    writer.u32(recipe.grain_scale);
}

fn read_recipe(reader: &mut Reader<'_>) -> Result<RecipeParts, PortError> {
    let softness = reader.f32()?;
    let mut curve = || -> Result<Vec<[f32; 2]>, PortError> {
        let count = reader.count(POINT_BYTES)?;
        (0..count)
            .map(|_| Ok([reader.f32()?, reader.f32()?]))
            .collect()
    };
    let (size_pressure, size_speed) = (curve()?, curve()?);
    let (opacity_pressure, opacity_speed) = (curve()?, curve()?);
    Ok(RecipeParts {
        softness,
        size_pressure,
        size_speed,
        opacity_pressure,
        opacity_speed,
        grain_strength: reader.f32()?,
        grain_scale: reader.u32()?,
    })
}
