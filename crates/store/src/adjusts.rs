//! Adjustment records (schema 12, DECISIONS D54; blur and sharpening since schema 14, D59): an
//! edit record of kind 2, keyed like fills.
//! After the kind: after (u32: the strokes under it), its clip (u8: 0 none, 1 a mask as fills
//! store it), inverted (u8: made on an inverted mask), then what it does (u8: 0 hue,
//! saturation and brightness as three f32; 1 curves: composite, red, green and blue, each a
//! point count (u32) and its points as x, y f32 pairs; 2 a Gaussian blur: its radius, f32; 3
//! sharpening: its amount, f32).

use openpaint_app::PortError;
use openpaint_core::{AdjustEdit, AdjustParts, AdjustmentParts};

use crate::codec::{Reader, Writer};
use crate::fills::{read_mask, write_mask};
use crate::strokes::{read_curve, write_curve};

pub(crate) const KIND_ADJUST: u8 = 2;
const HSB: u8 = 0;
const CURVES: u8 = 1;
const BLUR: u8 = 2;
const SHARPEN: u8 = 3;

pub(crate) fn encode_adjust(adjust: &AdjustEdit) -> Vec<u8> {
    let parts = adjust.to_parts();
    let mut writer = Writer::with_capacity(64);
    writer.u8(KIND_ADJUST);
    writer.len(adjust.after());
    match &parts.clip {
        Some(clip) => {
            writer.u8(1);
            write_mask(&mut writer, clip);
        }
        None => writer.u8(0),
    }
    writer.bool(parts.inverted);
    match &parts.adjustment {
        AdjustmentParts::Hsb(values) => {
            writer.u8(HSB);
            for value in values.iter().copied() {
                writer.f32(value);
            }
        }
        AdjustmentParts::Curves(curves) => {
            writer.u8(CURVES);
            for curve in curves.iter() {
                write_curve(&mut writer, curve.as_slice());
            }
        }
        AdjustmentParts::Blur(radius) => {
            writer.u8(BLUR);
            writer.f32(*radius);
        }
        AdjustmentParts::Sharpen(amount) => {
            writer.u8(SHARPEN);
            writer.f32(*amount);
        }
    }
    writer.finish()
}

/// An adjustment after its kind.
pub(crate) fn read_adjust(reader: &mut Reader<'_>) -> Result<AdjustParts, PortError> {
    let after = u64::from(reader.u32()?);
    let clip = match reader.u8()? {
        0 => None,
        1 => Some(read_mask(reader)?),
        _ => return Err(reader.corrupt()),
    };
    let inverted = reader.bool()?;
    let adjustment = match reader.u8()? {
        HSB => AdjustmentParts::Hsb([reader.f32()?, reader.f32()?, reader.f32()?]),
        CURVES => AdjustmentParts::Curves([
            read_curve(reader)?,
            read_curve(reader)?,
            read_curve(reader)?,
            read_curve(reader)?,
        ]),
        BLUR => AdjustmentParts::Blur(reader.f32()?),
        SHARPEN => AdjustmentParts::Sharpen(reader.f32()?),
        _ => return Err(reader.corrupt()),
    };
    Ok(AdjustParts {
        after,
        clip,
        inverted,
        adjustment,
    })
}

#[cfg(test)]
mod tests {
    use openpaint_core::{
        Adjustment, Artwork, Blur, CanvasSize, Curves, Hsb, RegionMask, Sharpen, ToneChannel,
        ToneCurve,
    };

    use crate::fills::{decode_edit, encode_edit};

    #[test]
    fn adjustment_records_round_trip_and_damage_is_corrupt() {
        let canvas = CanvasSize::new(40, 30).expect("size");
        let lifted = ToneCurve::new(&[[0.0, 0.1], [0.5, 0.8], [1.0, 1.0]]).expect("curve");
        let curves = Adjustment::Curves(Curves::default().with(ToneChannel::Green, lifted));
        let clip = RegionMask::polygon(canvas, &[[3.0, 4.0], [15.0, 4.0], [9.0, 13.0]]);
        for (adjustment, clip) in [
            (Adjustment::Hsb(Hsb::new(0.2, 0.7, 0.4)), None),
            (curves, clip.clone()),
            (Adjustment::Blur(Blur::new(6.5)), clip),
            (Adjustment::Sharpen(Sharpen::new(0.75)), None),
        ] {
            let artwork = Artwork::new(canvas)
                .adjust_region(clip, adjustment)
                .expect("adjusted")
                .artwork;
            let bytes = encode_edit(&artwork.layers()[0].edits()[0]);
            assert_eq!(bytes[0], super::KIND_ADJUST);
            let parts = decode_edit(&bytes).expect("decodes");
            assert_eq!(parts, artwork.to_parts().layers[0].edits[0]);
            assert!(decode_edit(&bytes[..bytes.len() - 1]).is_err(), "cut");
            assert!(decode_edit(&[bytes, vec![0]].concat()).is_err(), "longer");
        }
    }
}
