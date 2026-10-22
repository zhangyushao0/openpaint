//! Stroke records (format 1, `records.rs` has the keys): its id (u64); its blend (u8: 0 paint
//! over, 1 paint atop under Alpha Lock, 2 erase, 3 smudge, 4 smudge under Alpha Lock); its size,
//! opacity and colour's red, green and blue (f32); the seed of its stamps' jitter (u32); its brush
//! recipe (`write_recipe`); its samples, a count (u32) and per sample x, y, pressure, tilt and
//! azimuth (f32) and the time since the stroke began (u32 ms); last its clip, the selection it was
//! drawn inside (`write_optional_mask`, `edits.rs`).
//!
//! A recipe, stage by stage: softness (f32); size over pressure, speed and tilt, then opacity over
//! pressure, speed and tilt (curves: a point count (u32) and per point x, y (f32); tilt in
//! degrees); grain strength (f32) and scale (u32); the deposit (u8: 0 cover, 1 accumulate; when
//! accumulating its tip (u8: 0 flat, 1 bell), flow and maximum density (f32) and paper (u8: 0
//! none, 1 sketch; on paper its contact curve, scale, depth and dust, f32)); the taper's start,
//! end and tip (f32); the shape (`shapes.rs`). The toolbox stores its brushes' recipes alike.

use openpaint_app::PortError;
use openpaint_core::{
    AccumulationParts, PaperParts, PaperTexture, RecipeParts, RegionMask, SampleParts, Stroke,
    StrokeBlend, StrokeParts, Tip,
};

use crate::codec::{Reader, Writer};
use crate::edits::{read_optional_mask, write_optional_mask};
use crate::shapes::{read_shape, write_shape};

/// Bytes of one encoded sample.
const SAMPLE_BYTES: usize = 24;
/// Bytes of one curve point.
const POINT_BYTES: usize = 8;

pub(crate) fn encode_stroke(stroke: &Stroke) -> Vec<u8> {
    let samples = stroke.samples();
    let mut writer = Writer::with_capacity(160 + samples.len() * SAMPLE_BYTES);
    writer.u64(stroke.id().get());
    writer.u8(match stroke.blend() {
        StrokeBlend::Over => 0,
        StrokeBlend::Atop => 1,
        StrokeBlend::Erase => 2,
        StrokeBlend::Smudge => 3,
        StrokeBlend::SmudgeLocked => 4,
    });
    let style = stroke.style();
    let color = style.color();
    for value in [
        style.size(),
        style.opacity(),
        color.r(),
        color.g(),
        color.b(),
    ] {
        writer.f32(value);
    }
    writer.u32(style.seed());
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
    let clip = stroke.clip().map(RegionMask::to_parts);
    write_optional_mask(&mut writer, clip.as_ref());
    writer.finish()
}

pub(crate) fn decode_stroke(bytes: &[u8]) -> Result<StrokeParts, PortError> {
    let mut reader = Reader::new(bytes, "stroke");
    let id = reader.u64()?;
    let blend = match reader.u8()? {
        0 => StrokeBlend::Over,
        1 => StrokeBlend::Atop,
        2 => StrokeBlend::Erase,
        3 => StrokeBlend::Smudge,
        4 => StrokeBlend::SmudgeLocked,
        _ => return Err(reader.corrupt()),
    };
    let (size, opacity) = (reader.f32()?, reader.f32()?);
    let color = [reader.f32()?, reader.f32()?, reader.f32()?];
    let seed = reader.u32()?;
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
    let clip = read_optional_mask(&mut reader)?;
    reader.end()?;
    Ok(StrokeParts {
        id,
        blend,
        recipe,
        size,
        opacity,
        color,
        samples,
        clip,
        seed,
    })
}

/// A recipe, stage by stage (see the module).
pub(crate) fn write_recipe(writer: &mut Writer, recipe: &RecipeParts) {
    writer.f32(recipe.softness);
    for curve in [
        &recipe.size_pressure,
        &recipe.size_speed,
        &recipe.size_tilt,
        &recipe.opacity_pressure,
        &recipe.opacity_speed,
        &recipe.opacity_tilt,
    ] {
        write_curve(writer, curve);
    }
    writer.f32(recipe.grain_strength);
    writer.u32(recipe.grain_scale);
    write_deposit(writer, recipe.deposit.as_ref());
    for value in recipe.taper {
        writer.f32(value);
    }
    write_shape(writer, &recipe.shape);
}

/// A recipe as `write_recipe` stores it.
pub(crate) fn read_recipe(reader: &mut Reader<'_>) -> Result<RecipeParts, PortError> {
    let softness = reader.f32()?;
    let (size_pressure, size_speed) = (read_curve(reader)?, read_curve(reader)?);
    let size_tilt = read_curve(reader)?;
    let (opacity_pressure, opacity_speed) = (read_curve(reader)?, read_curve(reader)?);
    let opacity_tilt = read_curve(reader)?;
    let (grain_strength, grain_scale) = (reader.f32()?, reader.u32()?);
    let deposit = read_deposit(reader)?;
    let taper = [reader.f32()?, reader.f32()?, reader.f32()?];
    Ok(RecipeParts {
        softness,
        size_pressure,
        size_speed,
        size_tilt,
        opacity_pressure,
        opacity_speed,
        opacity_tilt,
        grain_strength,
        grain_scale,
        deposit,
        taper,
        shape: read_shape(reader)?,
    })
}

pub(crate) fn write_curve(writer: &mut Writer, curve: &[[f32; 2]]) {
    writer.len(curve.len());
    for [x, y] in curve {
        writer.f32(*x);
        writer.f32(*y);
    }
}

pub(crate) fn read_curve(reader: &mut Reader<'_>) -> Result<Vec<[f32; 2]>, PortError> {
    let count = reader.count(POINT_BYTES)?;
    (0..count)
        .map(|_| Ok([reader.f32()?, reader.f32()?]))
        .collect()
}

fn write_deposit(writer: &mut Writer, deposit: Option<&AccumulationParts>) {
    let Some(deposit) = deposit else {
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
