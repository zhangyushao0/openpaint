//! Selections as region masks: a lasso or rectangle rasterised at pixel centres, intersection
//! and inverse, and strokes drawn inside a selection, which keep their clip through storage.

use std::sync::Arc;

use openpaint_core::{
    Artwork, BrushRecipe, CanvasPoint, CanvasSize, Color, RegionMask, StrokeSample, StrokeStyle,
};

fn canvas() -> CanvasSize {
    CanvasSize::new(70, 40).unwrap()
}

fn pixels(mask: &RegionMask) -> Vec<[u32; 2]> {
    let mut all = Vec::new();
    for y in 0..40 {
        for x in 0..70 {
            if mask.contains([x, y]) {
                all.push([x, y]);
            }
        }
    }
    all
}

#[test]
fn a_rectangle_holds_the_pixels_whose_centres_it_covers() {
    let square = [[10.2, 5.0], [20.0, 5.0], [20.0, 9.6], [10.2, 9.6]];
    let mask = RegionMask::polygon(canvas(), &square).unwrap();
    assert_eq!((mask.origin(), mask.size()), ([10, 5], [10, 5]));
    assert_eq!(pixels(&mask).len(), 50);
    assert!(mask.contains([19, 9]) && !mask.contains([20, 9]) && !mask.contains([10, 10]));
    let thin = [[3.0, 3.0], [3.4, 3.0], [3.4, 30.0], [3.0, 30.0]];
    assert_eq!(
        RegionMask::polygon(canvas(), &thin),
        None,
        "no centre inside"
    );
    assert_eq!(RegionMask::polygon(canvas(), &square[..2]), None);
}

#[test]
fn a_lasso_off_the_canvas_keeps_what_lies_on_it_and_crossings_follow_even_odd() {
    let over = [[-20.0, -20.0], [90.0, -20.0], [90.0, 60.0], [-20.0, 60.0]];
    let all = RegionMask::polygon(canvas(), &over).unwrap();
    assert_eq!(all, RegionMask::whole(canvas()));
    // A bow tie: its two triangles in, nothing else.
    let bow = [[0.0, 0.0], [40.0, 40.0], [0.0, 40.0], [40.0, 0.0]];
    let mask = RegionMask::polygon(canvas(), &bow).unwrap();
    assert!(mask.contains([20, 2]) && mask.contains([20, 37]));
    assert!(!mask.contains([2, 20]) && !mask.contains([37, 20]));
}

#[test]
fn intersection_and_inverse_trim_to_their_pixels() {
    let a = RegionMask::polygon(canvas(), &rect(5.0, 5.0, 40.0, 30.0)).unwrap();
    let b = RegionMask::polygon(canvas(), &rect(30.0, 20.0, 60.0, 35.0)).unwrap();
    let both = a.intersect(&b).unwrap();
    assert_eq!((both.origin(), both.size()), ([30, 20], [10, 10]));
    let far = RegionMask::polygon(canvas(), &rect(50.0, 0.0, 60.0, 4.0)).unwrap();
    assert_eq!(a.intersect(&far), None);
    let either = a.union(&far);
    assert_eq!((either.origin(), either.size()), ([5, 0], [55, 30]));
    assert_eq!(pixels(&either).len(), 35 * 25 + 10 * 4);
    let outside = a.inverted(canvas()).unwrap();
    assert_eq!(pixels(&outside).len(), 70 * 40 - 35 * 25);
    assert!(outside.contains([0, 0]) && !outside.contains([5, 5]));
    assert_eq!(outside.inverted(canvas()).unwrap(), a);
    assert_eq!(RegionMask::whole(canvas()).inverted(canvas()), None);
}

fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> [[f32; 2]; 4] {
    [[x0, y0], [x1, y0], [x1, y1], [x0, y1]]
}

fn sample(x: f32, t: u32) -> StrokeSample {
    StrokeSample::new(CanvasPoint::new(x, 12.0).unwrap(), 1.0, 0.0, 0.0, t).unwrap()
}

#[test]
fn a_clipped_stroke_keeps_its_clip_through_storage_and_strokes_share_it_again() {
    let clip = RegionMask::polygon(canvas(), &rect(10.0, 0.0, 30.0, 40.0)).unwrap();
    let style = StrokeStyle::new(Arc::new(BrushRecipe::plain()), 4.0, 1.0, Color::BLACK).unwrap();
    let mut artwork = Artwork::new(canvas());
    for start in [2.0, 6.0] {
        let session = artwork
            .begin_stroke(style.clone(), sample(start, 0))
            .unwrap();
        let mut session = session.clipped(clip.clone()).unwrap();
        assert_eq!(session.active().clip(), Some(&clip));
        session.extend(&[sample(50.0, 8)]).unwrap();
        artwork = session.finish().artwork;
    }
    let strokes: Vec<_> = artwork.layers()[0].strokes().iter().cloned().collect();
    assert!(strokes[0].clip().unwrap().shares(&clip));
    let parts = artwork.to_parts();
    let stored = parts.layers[0].strokes[0].clip.clone().unwrap();
    assert_eq!((stored.origin, stored.size), ([10, 0], [20, 40]));
    let loaded = Artwork::from_storage(parts.clone()).unwrap();
    assert_eq!(loaded, artwork);
    let loaded: Vec<_> = loaded.layers()[0].strokes().iter().cloned().collect();
    let (first, second) = (loaded[0].clip().unwrap(), loaded[1].clip().unwrap());
    assert!(first.shares(second), "one mask in memory again");
    let mut broken = parts;
    broken.layers[0].strokes[1].clip.as_mut().unwrap().origin = [60, 0];
    assert!(
        Artwork::from_storage(broken).is_err(),
        "a clip off the canvas"
    );
    let wide = CanvasSize::new(140, 40).unwrap();
    let off = RegionMask::polygon(wide, &rect(100.0, 0.0, 120.0, 10.0)).unwrap();
    let session = Artwork::new(canvas())
        .begin_stroke(style, sample(1.0, 0))
        .unwrap();
    assert!(session.clipped(off).is_err());
}
