//! Which stroke records a save writes and removes. A layer's strokes are stored by position, so
//! between the stored version and the new one only the strokes after their common prefix
//! change: drawing appends one, undo drops the last, a branch replaces the tail.

use openpaint_core::{Artwork, Stroke, StrokeList};

/// One stroke record by layer id and position.
pub(crate) type Slot = (u64, usize);

/// The stroke records to write and the ones to remove; never the same slot in both.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct Delta<'a> {
    pub(crate) write: Vec<(Slot, &'a Stroke)>,
    pub(crate) remove: Vec<Slot>,
}

/// The records that turn the stored `base` into `artwork`. Per layer this compares stroke
/// handles (pointer and id, never samples) up to the first difference, then touches only what
/// follows it; layers `base` has and `artwork` lacks lose all their records.
pub(crate) fn delta<'a>(base: &Artwork, artwork: &'a Artwork) -> Delta<'a> {
    let mut delta = Delta::default();
    if base.same_version(artwork) {
        return delta;
    }
    for layer in artwork.layers() {
        let id = layer.id().get();
        let new = layer.strokes();
        let old = base.layer(layer.id()).map(|old| old.strokes());
        let kept = old.map_or(0, |old| common_prefix(old, new));
        for (offset, stroke) in new.since(kept).enumerate() {
            delta.write.push(((id, kept + offset), stroke));
        }
        let old_len = old.map_or(0, StrokeList::len);
        delta
            .remove
            .extend((new.len().max(kept)..old_len).map(|index| (id, index)));
    }
    for old in base.layers() {
        if artwork.layer(old.id()).is_none() {
            let id = old.id().get();
            delta
                .remove
                .extend((0..old.strokes().len()).map(|index| (id, index)));
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
        let style = StrokeStyle::new(4.0, 1.0, Color::BLACK).expect("style");
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
