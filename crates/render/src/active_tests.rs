use super::*;
use openpaint_core::{BrushRecipe, CanvasPoint, CanvasSize, Color, StrokeSession};

/// Samples along a gentle curve with a new pressure each, so every one is a knot.
fn sample(i: u16) -> StrokeSample {
    let t = f32::from(i);
    let position = CanvasPoint::new(4.0 + 4.0 * t, 10.0 + 0.05 * t * t).expect("point");
    StrokeSample::new(position, 0.5 + 0.01 * t, 0.0, 0.0, 4 * u32::from(i)).expect("sample")
}

fn style(tool: StrokeTool) -> StrokeStyle {
    let recipe = std::sync::Arc::new(BrushRecipe::plain());
    let style = StrokeStyle::new(recipe, 4.0, 1.0, Color::BLACK).expect("style");
    style.with_tool(tool)
}

fn begin(artwork: &Artwork, tool: StrokeTool, first: u16) -> StrokeSession {
    artwork
        .begin_stroke(style(tool), sample(first))
        .expect("begin")
}

fn blank() -> Artwork {
    Artwork::new(CanvasSize::new(128, 64).expect("size"))
}

fn extend(session: &mut StrokeSession, range: Range<u16>) {
    let samples: Vec<StrokeSample> = range.map(sample).collect();
    session.extend(&samples).expect("extend");
}

#[test]
fn a_growing_stroke_keeps_its_settled_spans_and_redraws_its_tail() {
    let base = blank();
    let mut track = ActiveTrack::default();
    let mut session = begin(&base, StrokeTool::Paint, 0);
    let first = track.advance(&base, Some(session.active()), true);
    assert_eq!((first.reset, first.kept.clone()), (true, 0..0));
    assert!(first.fresh_region.is_some(), "the dot, drawn as the tail");
    let mut settled = 0;
    let mut tail = first.fresh_region;
    for end in [2u16, 3, 5, 8, 9, 14] {
        let done = u16::try_from(session.active().samples().len()).expect("count");
        extend(&mut session, done..end);
        let step = track.advance(&base, Some(session.active()), true);
        let now = usize::from(end).saturating_sub(3);
        assert_eq!(
            (step.reset, step.kept.clone()),
            (false, settled..now),
            "at {end}"
        );
        let region = step.fresh_region.expect("fresh");
        let old = tail.expect("tail");
        assert_eq!(
            region.union(&old),
            region,
            "the tail drawn before is evaluated again"
        );
        settled = now;
        let path = Path::of(session.active().samples());
        tail = span_regions(
            session.active().style(),
            session.active().samples(),
            &path,
            now..path.spans(),
        )
        .reduce(|a, b| a.union(&b));
    }
    let same = track.advance(&base, Some(session.active()), true);
    assert_eq!(
        (same.reset, same.kept, same.fresh_region),
        (false, settled..settled, None)
    );
}

#[test]
fn a_cancelled_stroke_is_erased_and_a_committed_one_is_not() {
    let base = blank();
    let mut track = ActiveTrack::default();
    let mut session = begin(&base, StrokeTool::Paint, 0);
    extend(&mut session, 1..6);
    let drawn = track.advance(&base, Some(session.active()), true);
    let cancelled = track.advance(&session.clone().cancel(), None, true);
    assert_eq!(cancelled.redraw, drawn.fresh_region);
    track.advance(&base, Some(session.active()), true);
    let committed = track.advance(&session.finish().artwork, None, true);
    assert_eq!(
        committed.redraw, None,
        "the last frame showed the finished stroke"
    );
}

#[test]
fn samples_committed_before_their_evaluation_are_redrawn() {
    let base = blank();
    let mut track = ActiveTrack::default();
    let mut session = begin(&base, StrokeTool::Paint, 0);
    extend(&mut session, 1..6);
    track.advance(&base, Some(session.active()), true);
    let drawn = session.active().samples().to_vec();
    extend(&mut session, 6..8);
    let style = session.active().style().clone();
    let all = session.active().samples().to_vec();
    let committed = track.advance(&session.finish().artwork, None, true);
    let expected = union(
        stroke_region(&style, &drawn, 6),
        stroke_region(&style, &all, 6),
    );
    assert_eq!(committed.redraw, expected);
    assert!(expected.is_some());
}

#[test]
fn a_stroke_whose_end_moved_as_it_lifted_redraws_the_old_end_and_the_new() {
    let base = blank();
    let mut track = ActiveTrack::default();
    let mut session = begin(&base, StrokeTool::Paint, 0);
    extend(&mut session, 1..8);
    track.advance(&base, Some(session.active()), true);
    let style = session.active().style().clone();
    let drawn = session.active().samples().to_vec();
    session.revise(5, &[sample(9), sample(12)]).expect("revise");
    let moved = session.active().samples().to_vec();
    let committed = track.advance(&session.finish().artwork, None, true);
    let old = stroke_region(&style, &drawn, 5).expect("old end");
    let new = stroke_region(&style, &moved, 5).expect("new end");
    assert_eq!(committed.redraw, Some(old.union(&new)));
}

#[test]
fn another_stroke_starts_over_and_erases_the_first() {
    let base = blank();
    let mut track = ActiveTrack::default();
    let first = begin(&base, StrokeTool::Paint, 0);
    let drawn = track.advance(&base, Some(first.active()), true);
    let second = begin(&base, StrokeTool::Paint, 20);
    let next = track.advance(&base, Some(second.active()), true);
    assert!(next.reset);
    assert_eq!(next.redraw, drawn.fresh_region);
    let hidden = track.advance(&base, Some(second.active()), false);
    assert_eq!((hidden.kept, hidden.fresh_region), (0..0, None));
    assert!(
        track
            .advance(&base, Some(second.active()), true)
            .fresh_region
            .is_some()
    );
}

#[test]
fn a_smudge_trails_its_tail_until_committed() {
    let base = blank();
    let mut track = ActiveTrack::default();
    let mut session = begin(&base, StrokeTool::Smudge, 0);
    extend(&mut session, 1..10);
    let step = track.advance(&base, Some(session.active()), true);
    let samples = session.active().samples().to_vec();
    let path = Path::of(&samples);
    let style = session.active().style().clone();
    let settled =
        span_regions(&style, &samples, &path, 0..path.settled()).reduce(|a, b| a.union(&b));
    assert_eq!(step.fresh_region, settled, "no tail drawn");
    let tail = span_regions(&style, &samples, &path, path.settled()..path.spans())
        .reduce(|a, b| a.union(&b));
    let committed = track.advance(&session.finish().artwork, None, true);
    assert_eq!(committed.redraw, tail, "its tail shows once committed");
}
