//! Schema 19 -> 20 (S45, DECISIONS D72): every stroke's recipe gains its shape right after its
//! taper, followed by the seed the stroke's stamps are jittered by (`strokes.rs`, `shapes.rs`).
//! Every old stroke gets the continuous round capsule and seed 0, every old byte kept around
//! them, so it draws exactly as it did; heads are only marked 20.

use openpaint_app::ProjectId;

use crate::codec::{Reader, Writer};
use crate::migrate::{Upgrade, remarked};
use crate::records::{SCHEMA_KEY, encode_schema, head_key};
use crate::strokes::{read_clip, read_stroke_19, taper_end};

/// Bytes the step puts into every stroke: stamp, three floats, rotation, three floats, seed.
const SHAPED_BYTES: usize = 1 + 3 * 4 + 1 + 3 * 4 + 4;

/// Schema 19 -> 20, from every project's head and every stroke record as schema 19 stored them
/// (layers, masks and nested records alike). A head not marked 19, or a record that is not a
/// whole schema-19 stroke, is left as it is, so opening its project reports what is wrong.
pub(crate) fn upgrade_19_to_20(
    heads: &[(ProjectId, Vec<u8>)],
    strokes: &[(Vec<u8>, Vec<u8>)],
) -> Upgrade {
    let mut upgrade = Upgrade::default();
    for (id, head) in heads {
        if let Some(head) = remarked(head, 19, 20) {
            upgrade.projects.push((head_key(*id), head));
        }
    }
    for (key, value) in strokes {
        if let Some(stroke) = stroke_19_to_20(value) {
            upgrade.projects.push((key.clone(), stroke));
        }
    }
    upgrade
        .library
        .push((SCHEMA_KEY.to_vec(), encode_schema(20)));
    upgrade
}

/// A whole schema-19 stroke with the continuous shape and seed 0 after its taper.
fn stroke_19_to_20(stroke: &[u8]) -> Option<Vec<u8>> {
    let mut reader = Reader::new(stroke, "stroke");
    read_stroke_19(&mut reader).ok()?;
    read_clip(&mut reader).ok()?;
    reader.end().ok()?;
    let at = taper_end(stroke)?;
    let mut writer = Writer::with_capacity(stroke.len() + SHAPED_BYTES);
    writer.bytes(&stroke[..at]);
    // A disc of aspect 1, no spacing (continuous) and no scatter, fixed rotation, no angle and
    // no jitter, then seed 0: frozen here, apart from any brush.
    writer.u8(0);
    for value in [1.0, 0.0, 0.0] {
        writer.f32(value);
    }
    writer.u8(0);
    for _ in 0..3 {
        writer.f32(0.0);
    }
    writer.u32(0);
    writer.bytes(&stroke[at..]);
    Some(writer.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::records::{decode_stroke, write_recipe};
    use crate::strokes::write_curve;
    use openpaint_core::{BrushRecipe, MaskParts, Shape, ShapeParts, Taper};

    /// A schema-19 stroke of the plain round brush, accumulating under a bell, widening with
    /// tilt and tapered: id 9, size 12, opacity 0.5, black, erasing, three samples tilted, a
    /// clip.
    fn stroke_19() -> Vec<u8> {
        let mut writer = Writer::default();
        writer.u64(9);
        for value in [12.0f32, 0.5, 0.0, 0.0, 0.0] {
            writer.f32(value);
        }
        writer.u8(2);
        write_recipe(&mut writer, &BrushRecipe::plain().to_parts());
        // Accumulating, a bell tip, flow 1.25, max density 1, no paper.
        writer.u8(1);
        writer.u8(1);
        writer.f32(1.25);
        writer.f32(1.0);
        writer.u8(0);
        // Size over tilt up to 2, opacity over tilt flat, a taper of 1.5 and 2 to a quarter.
        write_curve(&mut writer, &[[0.0, 1.0], [60.0, 2.0]]);
        write_curve(&mut writer, &[[0.0, 1.0]]);
        for value in [1.5f32, 2.0, 0.25] {
            writer.f32(value);
        }
        writer.u32(3);
        for (x, time) in [(3.0f32, 0u32), (9.0, 8), (15.0, 16)] {
            for value in [x, 4.0, 0.25, 1.2, -1.0] {
                writer.f32(value);
            }
            writer.u32(time);
        }
        writer.u8(1);
        let clip = MaskParts {
            origin: [2, 1],
            size: [4, 2],
            runs: vec![3, 5],
        };
        crate::fills::write_mask(&mut writer, &clip);
        writer.finish()
    }

    #[test]
    fn strokes_gain_the_continuous_shape_and_seed_0_and_keep_every_byte() {
        let old = stroke_19();
        let heads = [
            (
                ProjectId::new(1),
                [&19u32.to_le_bytes()[..], &[4, 5]].concat(),
            ),
            (ProjectId::new(2), vec![18, 0, 0, 0, 7]),
        ];
        let strokes = [
            (b"key".to_vec(), old.clone()),
            (b"cut".to_vec(), old[..old.len() - 1].to_vec()),
            (b"long".to_vec(), [&old[..], &[0]].concat()),
        ];
        let upgrade = upgrade_19_to_20(&heads, &strokes);
        let marked = [&20u32.to_le_bytes()[..], &[4, 5]].concat();
        assert_eq!(
            upgrade.projects.len(),
            2,
            "a foreign head, a cut and a long stroke stay"
        );
        assert_eq!(upgrade.projects[0], (head_key(ProjectId::new(1)), marked));
        let (key, new) = &upgrade.projects[1];
        assert_eq!(key, b"key");
        let at = taper_end(&old).expect("a recipe");
        assert_eq!(new.len(), old.len() + 30, "the shape and the seed");
        assert_eq!(new[..at], old[..at], "every byte up to the taper");
        let shaped = [
            &[0][..],
            &1f32.to_le_bytes(),
            &[0; 8],
            &[0],
            &[0; 12],
            &[0; 4],
        ]
        .concat();
        assert_eq!(new[at..at + 30], shaped, "a continuous disc, seed 0");
        assert_eq!(new[at + 30..], old[at..], "the samples and the clip");
        let stroke = decode_stroke(new).expect("a schema-20 stroke");
        assert_eq!(
            (stroke.recipe.shape, stroke.seed),
            (ShapeParts::CONTINUOUS, 0)
        );
        let recipe = BrushRecipe::from_parts(&stroke.recipe).expect("valid");
        assert_eq!(*recipe.shape(), Shape::CONTINUOUS);
        assert_eq!(recipe.taper(), Taper::new(1.5, 2.0, 0.25).expect("taper"));
        assert_eq!(recipe.size().tilt().points(), [[0.0, 1.0], [60.0, 2.0]]);
        assert_eq!((stroke.id, stroke.samples.len()), (9, 3));
        assert!(stroke.clip.is_some());
        assert_eq!(upgrade.library, [(SCHEMA_KEY.to_vec(), encode_schema(20))]);
    }
}
