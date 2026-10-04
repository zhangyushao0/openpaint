//! Which stroke records a save writes and removes. A layer's strokes are stored by position, so
//! between the stored version and the new one only the strokes after their common prefix
//! change: drawing appends one, undo drops the last, a branch replaces the tail.

use openpaint_core::{Artwork, RegionEdit, Stroke, StrokeList};

use crate::records::mask_slot;

/// One stroke or fill record by layer id and position.
pub(crate) type Slot = (u64, usize);

/// The stroke and fill records to write and the ones to remove; never the same slot in both.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct Delta<'a> {
    pub(crate) write: Vec<(Slot, &'a Stroke)>,
    pub(crate) remove: Vec<Slot>,
    pub(crate) write_fills: Vec<(Slot, &'a RegionEdit)>,
    pub(crate) remove_fills: Vec<Slot>,
}

/// What one slot holds: a layer's paint under its id, or its mask under `mask_slot`.
pub(crate) type Content<'a> = (u64, &'a StrokeList, &'a [RegionEdit]);

/// Every slot of `artwork`: each layer's paint, then its mask's if it has one.
pub(crate) fn contents(artwork: &Artwork) -> Vec<Content<'_>> {
    let mut contents = Vec::with_capacity(artwork.layers().len());
    for layer in artwork.layers() {
        let id = layer.id().get();
        contents.push((id, layer.strokes(), layer.edits()));
        if let Some(mask) = layer.mask() {
            contents.push((mask_slot(id), mask.strokes(), mask.edits()));
        }
    }
    contents
}

/// The records that turn the stored `base` into `artwork`. Per slot (a layer's paint or its
/// mask) this compares stroke handles (pointer and id, never samples) up to the first
/// difference, then touches only what follows it; slots `base` has and `artwork` lacks (a
/// deleted layer or mask) lose all their records.
pub(crate) fn delta<'a>(base: &Artwork, artwork: &'a Artwork) -> Delta<'a> {
    let mut delta = Delta::default();
    if base.same_version(artwork) {
        return delta;
    }
    let old = contents(base);
    let new = contents(artwork);
    let find = |slots: &[Content<'_>], id: u64| slots.iter().position(|(slot, ..)| *slot == id);
    for &(id, strokes, edits) in &new {
        let before = find(&old, id).map(|at| old[at]);
        let old_strokes = before.map(|(_, strokes, _)| strokes);
        let kept = old_strokes.map_or(0, |old| common_prefix(old, strokes));
        for (offset, stroke) in strokes.since(kept).enumerate() {
            delta.write.push(((id, kept + offset), stroke));
        }
        let old_len = old_strokes.map_or(0, StrokeList::len);
        delta
            .remove
            .extend((strokes.len().max(kept)..old_len).map(|index| (id, index)));
        let old_fills = before.map_or(&[][..], |(_, _, edits)| edits);
        let kept = old_fills
            .iter()
            .zip(edits)
            .take_while(|(old, new)| old.same(new))
            .count();
        for (index, fill) in edits.iter().enumerate().skip(kept) {
            delta.write_fills.push(((id, index), fill));
        }
        let removed = edits.len().max(kept)..old_fills.len();
        delta.remove_fills.extend(removed.map(|index| (id, index)));
    }
    for &(id, strokes, edits) in &old {
        if find(&new, id).is_none() {
            delta
                .remove
                .extend((0..strokes.len()).map(|index| (id, index)));
            delta
                .remove_fills
                .extend((0..edits.len()).map(|index| (id, index)));
        }
    }
    delta
}

/// How many strokes from the start both lists hold as the same stroke.
fn common_prefix(old: &StrokeList, new: &StrokeList) -> usize {
    old.iter()
        .zip(new.iter())
        .take_while(|(old, new)| same(old, new))
        .count()
}

/// One stroke seen from two versions: same id, same style, one shared sample array.
fn same(old: &Stroke, new: &Stroke) -> bool {
    old.id() == new.id() && old.style() == new.style() && old.shares_samples(new)
}

#[cfg(test)]
mod tests {
    use openpaint_core::{
        ArtworkCommand, CanvasPoint, CanvasSize, Color, LayerCommand, StrokeSample, StrokeStyle,
    };

    use super::*;

    fn blank() -> Artwork {
        Artwork::new(CanvasSize::new(64, 48).expect("size"))
    }

    fn draw(artwork: &Artwork, x: f32) -> Artwork {
        let recipe = std::sync::Arc::new(openpaint_core::BrushRecipe::plain());
        let style = StrokeStyle::new(recipe, 4.0, 1.0, Color::BLACK).expect("style");
        let sample = |x| {
            let point = CanvasPoint::new(x, 10.0).expect("point");
            StrokeSample::new(point, 0.5, 0.0, 0.0, 0).expect("sample")
        };
        let mut session = artwork.begin_stroke(style, sample(x)).expect("begin");
        session.extend(&[sample(x + 5.0)]).expect("extend");
        session.finish().artwork
    }

    fn slots(delta: &Delta<'_>) -> (Vec<Slot>, Vec<Slot>) {
        let written = delta.write.iter().map(|(slot, _)| *slot).collect();
        (written, delta.remove.clone())
    }

    #[test]
    fn drawing_writes_only_the_new_stroke_and_undo_removes_it() {
        let one = draw(&blank(), 1.0);
        let two = draw(&one, 2.0);
        assert_eq!(slots(&delta(&one, &two)), (vec![(1, 1)], vec![]));
        assert_eq!(slots(&delta(&two, &one)), (vec![], vec![(1, 1)]));
        assert_eq!(slots(&delta(&two, &two)), (vec![], vec![]));
    }

    #[test]
    fn a_branch_rewrites_the_tail_from_the_first_difference() {
        let one = draw(&blank(), 1.0);
        let two = draw(&one, 2.0);
        let three = draw(&two, 3.0);
        // Undo twice, then draw: the new stroke reuses position 1 with the id undone there.
        let branch = draw(&one, 9.0);
        assert_eq!(slots(&delta(&three, &branch)), (vec![(1, 1)], vec![(1, 2)]));
    }

    #[test]
    fn a_fill_writes_its_record_once_and_undo_removes_it() {
        let one = draw(&blank(), 1.0);
        let canvas = one.canvas();
        let mut inside = vec![false; 64 * 48];
        inside[100] = true;
        let mask = openpaint_core::RegionMask::from_pixels(canvas, &inside).expect("mask");
        let filled = one.fill_region(mask, Color::WHITE).expect("fill").artwork;
        let fills = |delta: &Delta<'_>| {
            let written: Vec<Slot> = delta.write_fills.iter().map(|(slot, _)| *slot).collect();
            (written, delta.remove_fills.clone())
        };
        assert_eq!(fills(&delta(&one, &filled)), (vec![(1, 0)], vec![]));
        assert_eq!(
            slots(&delta(&one, &filled)),
            (vec![], vec![]),
            "no stroke record"
        );
        let later = draw(&filled, 9.0);
        assert_eq!(fills(&delta(&filled, &later)), (vec![], vec![]));
        assert_eq!(fills(&delta(&later, &one)), (vec![], vec![(1, 0)]));
    }

    #[test]
    fn a_deleted_layer_loses_its_records_and_a_new_one_writes_all() {
        let drawn = draw(&draw(&blank(), 1.0), 2.0);
        let create = ArtworkCommand::Layer(LayerCommand::Create);
        let layered = drawn.apply(create).expect("create").artwork;
        let on_top = draw(&layered, 5.0);
        assert_eq!(slots(&delta(&blank(), &on_top)).0.len(), 3);
        let first = on_top.layers()[0].id();
        let delete = ArtworkCommand::Layer(LayerCommand::Delete(first));
        let deleted = on_top.apply(delete).expect("delete").artwork;
        assert_eq!(
            slots(&delta(&on_top, &deleted)),
            (vec![], vec![(1, 0), (1, 1)])
        );
    }
}
