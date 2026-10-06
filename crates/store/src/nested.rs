//! Nested records (schema 15, DECISIONS D66): a layer merged down into another, an edit record
//! of kind 4 keyed like fills. After the kind: after (u32: the strokes under it), the merged
//! layer's id (u64), opacity (f32), blend (u8, as the head's), clipped (u8), its stroke and edit
//! counts (u32), then its mask as the head stores a layer's (`write_layer_mask`). Its strokes and
//! edits are records under its id, its mask's under `mask_slot(id)`, as a layer's are, so a
//! merge rewrites none of them: they stay where the layer kept them.

use openpaint_app::PortError;
use openpaint_core::{NestedEdit, NestedParts};

use crate::codec::{Reader, Writer};
use crate::records::{blend_byte, read_blend, read_layer_mask, write_layer_mask};

pub(crate) const KIND_NESTED: u8 = 4;

/// How many records a nested record's content holds: its strokes and edits, its mask's.
pub(crate) type Counts = [usize; 4];

pub(crate) fn encode_nested(nested: &NestedEdit) -> Vec<u8> {
    let layer = nested.layer();
    let mut writer = Writer::with_capacity(40);
    writer.u8(KIND_NESTED);
    writer.len(nested.after());
    writer.u64(layer.id().get());
    writer.f32(layer.opacity());
    writer.u8(blend_byte(layer.blend()));
    writer.bool(layer.clipped());
    writer.len(layer.strokes().len());
    writer.len(layer.edits().len());
    write_layer_mask(&mut writer, layer.mask());
    writer.finish()
}

/// A nested record after its kind, its content still empty, and how many records it holds.
pub(crate) fn read_nested(reader: &mut Reader<'_>) -> Result<(NestedParts, Counts), PortError> {
    let after = u64::from(reader.u32()?);
    let id = reader.u64()?;
    let opacity = reader.f32()?;
    let blend = read_blend(reader)?;
    let clipped = reader.bool()?;
    let (strokes, edits) = (reader.u32()? as usize, reader.u32()? as usize);
    let (mask, [mask_strokes, mask_edits]) = read_layer_mask(reader)?;
    let parts = NestedParts {
        after,
        id,
        opacity,
        blend,
        clipped,
        strokes: Vec::new(),
        edits: Vec::new(),
        mask,
    };
    Ok((parts, [strokes, edits, mask_strokes, mask_edits]))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use openpaint_core::{
        Artwork, ArtworkCommand, BrushRecipe, CanvasPoint, CanvasSize, Color, EditParts,
        LayerCommand, LayerId, MaskCommand, StrokeSample, StrokeStyle,
    };

    use crate::fills::{decode_record, encode_edit};

    fn draw(artwork: &Artwork) -> Artwork {
        let recipe = Arc::new(BrushRecipe::plain());
        let style = StrokeStyle::new(recipe, 4.0, 1.0, Color::BLACK).expect("style");
        let sample = |x| {
            let point = CanvasPoint::new(x, 10.0).expect("point");
            StrokeSample::new(point, 0.5, 0.0, 0.0, 0).expect("sample")
        };
        let mut session = artwork.begin_stroke(style, sample(2.0)).expect("begin");
        session.extend(&[sample(9.0)]).expect("extend");
        session.finish().artwork
    }

    #[test]
    fn a_nested_record_holds_how_it_composited_and_its_counts_and_damage_is_corrupt() {
        let layer = |a: &Artwork, c| a.apply(ArtworkCommand::Layer(c)).expect("command").artwork;
        let blank = Artwork::new(CanvasSize::new(40, 30).expect("size"));
        let upper = draw(&layer(&blank, LayerCommand::Create));
        let masked = upper.apply(ArtworkCommand::Mask(MaskCommand::Add(LayerId::new(2))));
        let masked = draw(&draw(&masked.expect("mask").artwork));
        let masked = layer(&masked, LayerCommand::Select(LayerId::new(2)));
        let merged = layer(&masked, LayerCommand::MergeDown(LayerId::new(2)));
        let bytes = encode_edit(&merged.layers()[0].edits()[0]);
        assert_eq!(bytes.len(), 1 + 4 + 8 + 4 + 1 + 1 + 4 + 4 + 1 + 8);
        let (parts, counts) = decode_record(&bytes).expect("decodes");
        assert_eq!(counts, Some([1, 0, 2, 0]));
        let EditParts::Nested(mut expected) = merged.to_parts().layers[0].edits[0].clone() else {
            panic!("a nested record");
        };
        expected.strokes.clear();
        if let Some(mask) = expected.mask.as_mut() {
            mask.strokes.clear();
        }
        assert_eq!(parts, EditParts::Nested(expected));
        assert!(decode_record(&bytes[..bytes.len() - 1]).is_err(), "cut");
        assert!(
            decode_record(&[bytes.clone(), vec![0]].concat()).is_err(),
            "longer"
        );
    }
}
