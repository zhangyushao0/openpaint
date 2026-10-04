//! Stroke records (schema 9; the layout is in `records.rs`): style, blend, recipe, samples and,
//! since schema 7, the selection the stroke was drawn inside (DECISIONS D43). Since schema 9
//! the blend may be a smudge (3) or a smudge under Alpha Lock (4) (D46).

use openpaint_app::PortError;
use openpaint_core::{
    AccumulationParts, PaperParts, PaperTexture, RecipeParts, SampleParts, Stroke, StrokeBlend,
    StrokeParts, Tip,
};

use crate::codec::{Reader, Writer};
use crate::fills::{read_mask, write_mask};

/// Bytes of one encoded sample.
const SAMPLE_BYTES: usize = 24;
/// Bytes of a stroke record before its blend: id, size, opacity and colour.
pub(crate) const STROKE_STYLE_BYTES: usize = 8 + 5 * 4;
/// Bytes of one curve point.
const POINT_BYTES: usize = 8;

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
        StrokeBlend::Smudge => 3,
        StrokeBlend::SmudgeLocked => 4,
    });
    write_whole_recipe(&mut writer, &style.recipe().to_parts());
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
    match stroke.clip() {
        None => writer.u8(0),
        Some(clip) => {
            writer.u8(1);
            write_mask(&mut writer, &clip.to_parts());
        }
    }
    writer.finish()
}

pub(crate) fn decode_stroke(bytes: &[u8]) -> Result<StrokeParts, PortError> {
    let mut reader = Reader::new(bytes, "stroke");
    let mut stroke = read_stroke_6(&mut reader)?;
    stroke.clip = match reader.u8()? {
        0 => None,
        1 => Some(read_mask(&mut reader)?),
        _ => return Err(reader.corrupt()),
    };
    reader.end()?;
    Ok(stroke)
}

/// A stroke as schema 6 stored all of it, which schema 7 follows with the clip.
pub(crate) fn read_stroke_6(reader: &mut Reader<'_>) -> Result<StrokeParts, PortError> {
    let id = reader.u64()?;
    let (size, opacity) = (reader.f32()?, reader.f32()?);
    let color = [reader.f32()?, reader.f32()?, reader.f32()?];
    let blend = match reader.u8()? {
        0 => StrokeBlend::Over,
        1 => StrokeBlend::Atop,
        2 => StrokeBlend::Erase,
        // Since schema 9 (D46): a smudge, then one under Alpha Lock.
        3 => StrokeBlend::Smudge,
        4 => StrokeBlend::SmudgeLocked,
        _ => return Err(reader.corrupt()),
    };
    let recipe = read_recipe(reader)?;
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
    Ok(StrokeParts {
        id,
        blend,
        recipe,
        size,
        opacity,
        color,
        samples,
        clip: None,
    })
}

/// A recipe as a stroke record holds it, after the blend: its stages up to the grain (schema
/// 3's recipe), then its deposit.
fn write_whole_recipe(writer: &mut Writer, recipe: &RecipeParts) {
    write_recipe(writer, recipe);
    let Some(deposit) = &recipe.deposit else {
        writer.u8(0);
        return;
    };
    writer.u8(1);
    writer.u8(match deposit.tip {
        Tip::Flat => 0,
        Tip::Bell => 1,
    });
    writer.f32(deposit.flow);
    writer.f32(deposit.max_density);
    let Some(paper) = &deposit.paper else {
        writer.u8(0);
        return;
    };
    writer.u8(match paper.texture {
        PaperTexture::Sketch => 1,
    });
    write_curve(writer, &paper.contact);
    for value in [paper.scale, paper.depth, paper.dust] {
        writer.f32(value);
    }
}

pub(crate) fn write_curve(writer: &mut Writer, curve: &[[f32; 2]]) {
    writer.len(curve.len());
    for [x, y] in curve {
        writer.f32(*x);
        writer.f32(*y);
    }
}

/// A recipe's stages up to its grain, as schemas 3 to 5 stored the whole recipe.
pub(crate) fn write_recipe(writer: &mut Writer, recipe: &RecipeParts) {
    writer.f32(recipe.softness);
    for curve in [
        &recipe.size_pressure,
        &recipe.size_speed,
        &recipe.opacity_pressure,
        &recipe.opacity_speed,
    ] {
        write_curve(writer, curve);
    }
    writer.f32(recipe.grain_strength);
    writer.u32(recipe.grain_scale);
}

pub(crate) fn read_curve(reader: &mut Reader<'_>) -> Result<Vec<[f32; 2]>, PortError> {
    let count = reader.count(POINT_BYTES)?;
    (0..count)
        .map(|_| Ok([reader.f32()?, reader.f32()?]))
        .collect()
}

fn read_recipe(reader: &mut Reader<'_>) -> Result<RecipeParts, PortError> {
    let softness = reader.f32()?;
    let (size_pressure, size_speed) = (read_curve(reader)?, read_curve(reader)?);
    let (opacity_pressure, opacity_speed) = (read_curve(reader)?, read_curve(reader)?);
    let (grain_strength, grain_scale) = (reader.f32()?, reader.u32()?);
    Ok(RecipeParts {
        softness,
        size_pressure,
        size_speed,
        opacity_pressure,
        opacity_speed,
        grain_strength,
        grain_scale,
        deposit: read_deposit(reader)?,
    })
}

fn read_deposit(reader: &mut Reader<'_>) -> Result<Option<AccumulationParts>, PortError> {
    match reader.u8()? {
        0 => return Ok(None),
        1 => {}
        _ => return Err(reader.corrupt()),
    }
    let tip = match reader.u8()? {
        0 => Tip::Flat,
        1 => Tip::Bell,
        _ => return Err(reader.corrupt()),
    };
    let (flow, max_density) = (reader.f32()?, reader.f32()?);
    let paper = match reader.u8()? {
        0 => None,
        1 => Some(PaperParts {
            texture: PaperTexture::Sketch,
            contact: read_curve(reader)?,
            scale: reader.f32()?,
            depth: reader.f32()?,
            dust: reader.f32()?,
        }),
        _ => return Err(reader.corrupt()),
    };
    Ok(Some(AccumulationParts {
        tip,
        flow,
        max_density,
        paper,
    }))
}
