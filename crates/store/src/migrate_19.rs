//! Schema 18 -> 19 (S44, DECISIONS D71): every stroke's recipe gains its size and opacity over
//! tilt and its taper, right after its deposit (`strokes.rs`). Every old stroke gets the curves
//! flat at 1 and no taper, every old byte kept around them, so it draws exactly as it did; heads
//! are only marked 19.

use openpaint_app::ProjectId;

use crate::codec::{Reader, Writer};
use crate::migrate::{Upgrade, remarked};
use crate::records::{SCHEMA_KEY, encode_schema, head_key};
use crate::strokes::{UNTILTED, deposit_end, read_clip, read_stroke_6, write_curve};

/// Schema 18 -> 19, from every project's head and every stroke record as schema 18 stored them
/// (layers, masks and nested records alike). A head not marked 18, or a record that is not a
/// whole schema-18 stroke, is left as it is, so opening its project reports what is wrong.
pub(crate) fn upgrade_18_to_19(
    heads: &[(ProjectId, Vec<u8>)],
    strokes: &[(Vec<u8>, Vec<u8>)],
) -> Upgrade {
    let mut upgrade = Upgrade::default();
    for (id, head) in heads {
        if let Some(head) = remarked(head, 18, 19) {
            upgrade.projects.push((head_key(*id), head));
        }
    }
    for (key, value) in strokes {
        if let Some(stroke) = stroke_18_to_19(value) {
            upgrade.projects.push((key.clone(), stroke));
        }
    }
    upgrade
        .library
        .push((SCHEMA_KEY.to_vec(), encode_schema(19)));
    upgrade
}

/// A whole schema-18 stroke with flat tilt curves and no taper after its deposit.
fn stroke_18_to_19(stroke: &[u8]) -> Option<Vec<u8>> {
    let mut reader = Reader::new(stroke, "stroke");
    read_stroke_6(&mut reader).ok()?;
    read_clip(&mut reader).ok()?;
    reader.end().ok()?;
    let at = deposit_end(stroke)?;
    let mut writer = Writer::with_capacity(stroke.len() + 36);
    writer.bytes(&stroke[..at]);
    // Size and opacity over tilt flat at 1, then no taper: frozen here, apart from any brush.
    for _ in 0..2 {
        write_curve(&mut writer, &UNTILTED);
    }
    for _ in 0..3 {
        writer.f32(0.0);
    }
    writer.bytes(&stroke[at..]);
    Some(writer.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::records::{decode_stroke, write_recipe};
    use openpaint_core::{BrushRecipe, Curve, MaskParts, Taper};

    /// A schema-18 stroke of the plain round brush, accumulating under a bell: id 9, size 12,
    /// opacity 0.5, black, erasing, three samples tilted, a clip.
    fn stroke_18() -> Vec<u8> {
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
    fn strokes_gain_flat_tilt_and_no_taper_and_keep_every_byte() {
        let old = stroke_18();
        let heads = [
            (
                ProjectId::new(1),
                [&18u32.to_le_bytes()[..], &[4, 5]].concat(),
            ),
            (ProjectId::new(2), vec![17, 0, 0, 0, 7]),
        ];
        let strokes = [
            (b"key".to_vec(), old.clone()),
            (b"cut".to_vec(), old[..old.len() - 1].to_vec()),
            (b"long".to_vec(), [&old[..], &[0]].concat()),
        ];
        let upgrade = upgrade_18_to_19(&heads, &strokes);
        let marked = [&19u32.to_le_bytes()[..], &[4, 5]].concat();
        assert_eq!(
            upgrade.projects.len(),
            2,
            "a foreign head, a cut and a long stroke stay"
        );
        assert_eq!(upgrade.projects[0], (head_key(ProjectId::new(1)), marked));
        let (key, new) = &upgrade.projects[1];
        assert_eq!(key, b"key");
        let at = deposit_end(&old).expect("a recipe");
        assert_eq!(
            new.len(),
            old.len() + 36,
            "two one-point curves and three floats"
        );
        assert_eq!(new[..at], old[..at], "every byte up to the deposit");
        assert_eq!(new[at + 36..], old[at..], "the samples and the clip");
        let stroke = decode_stroke(new).expect("a schema-19 stroke");
        let recipe = BrushRecipe::from_parts(&stroke.recipe).expect("valid");
        assert_eq!(recipe.taper(), Taper::NONE);
        assert_eq!(
            *recipe.size().tilt(),
            Curve::new(&[[0.0, 1.0]]).expect("flat")
        );
        assert_eq!(*recipe.opacity().tilt(), *recipe.size().tilt());
        assert_eq!((stroke.id, stroke.samples.len()), (9, 3));
        assert!(stroke.clip.is_some());
        assert_eq!(upgrade.library, [(SCHEMA_KEY.to_vec(), encode_schema(19))]);
    }
}
