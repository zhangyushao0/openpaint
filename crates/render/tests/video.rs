//! GPU tests of the replay's video frames (S163): frames drawn one after another on the video's
//! own canvas (strokes drawn along their samples, committed, a fill, the playhead moved back and
//! on) equal each frame drawn cold; the scale pass equals its CPU reference on any GPU, a video of
//! the canvas's size is the canvas; the end card lays the picture on the desk above the band.

use std::sync::{Arc, OnceLock};

use openpaint_core::{
    ActiveStroke, Artwork, ArtworkCommand, BrushRecipe, CanvasPoint, CanvasSize, Color,
    LayerCommand, RegionMask, StrokeSample, StrokeStyle,
};
use openpaint_render::{CanvasTarget, Core, FrameRequest, RgbaImage, VideoTarget, video_reference};

const CANVAS: [u32; 2] = [320, 200];
/// Five canvas pixels to three video pixels: weights that split pixels.
const VIDEO: [u32; 2] = [192, 120];
const DESK: [u8; 4] = [233, 230, 224, 255];

/// This binary's one GPU device (one device per test binary, ARCHITECTURE section 8).
fn core() -> &'static Core {
    static CORE: OnceLock<Core> = OnceLock::new();
    CORE.get_or_init(|| Core::headless().expect("a GPU adapter is required for render tests"))
}

fn sample(x: f32, y: f32, pressure: f32, time_ms: u32) -> StrokeSample {
    let position = CanvasPoint::new(x, y).expect("canvas point");
    StrokeSample::new(position, pressure, 0.0, 0.0, time_ms).expect("sample")
}

fn style(size: f32, opacity: f32, [r, g, b]: [f32; 3]) -> StrokeStyle {
    let color = Color::rgb(r, g, b).expect("colour");
    StrokeStyle::new(Arc::new(BrushRecipe::plain()), size, opacity, color).expect("style")
}

/// A wave across the canvas, translucent, its pressure rising and falling.
fn wave() -> (StrokeStyle, Vec<StrokeSample>) {
    let samples = (0..48u16)
        .map(|i| {
            let t = f32::from(i) / 47.0;
            let x = 30.0 + 260.0 * t;
            let y = 100.0 + 55.0 * (std::f32::consts::TAU * t).sin();
            sample(
                x,
                y,
                0.1 + 0.9 * (std::f32::consts::PI * t).sin(),
                u32::from(i) * 8,
            )
        })
        .collect();
    (style(36.0, 0.8, [0.12, 0.25, 0.65]), samples)
}

/// A diagonal crossing the wave.
fn diagonal() -> (StrokeStyle, Vec<StrokeSample>) {
    let samples = (0..40u16)
        .map(|i| {
            let t = f32::from(i) / 39.0;
            sample(
                60.0 + 200.0 * t,
                30.0 + 140.0 * t,
                0.3 + 0.7 * t,
                u32::from(i) * 8,
            )
        })
        .collect();
    (style(20.0, 0.6, [0.85, 0.2, 0.1]), samples)
}

fn base() -> Artwork {
    let blank = Artwork::new(CanvasSize::new(CANVAS[0], CANVAS[1]).expect("size"));
    let created = ArtworkCommand::Layer(LayerCommand::Create);
    blank.apply(created).expect("layer").artwork
}

/// The moments of a stroke drawn over `artwork` as the replay shows them: its first samples
/// growing, then the artwork with it committed.
fn drawn(
    artwork: &Artwork,
    (style, samples): &(StrokeStyle, Vec<StrokeSample>),
) -> (Vec<(Artwork, ActiveStroke)>, Artwork) {
    let mut session = artwork
        .begin_stroke(style.clone(), samples[0])
        .expect("begin");
    let mut moments = vec![(artwork.clone(), session.active().clone())];
    let mut done = 1;
    for end in [7, 19, 33, samples.len()] {
        session.extend(&samples[done..end]).expect("extend");
        moments.push((artwork.clone(), session.active().clone()));
        done = end;
    }
    (moments, session.finish().artwork)
}

/// A replay's frames: the wave drawn, the diagonal drawn, a fill, the playhead back to after the
/// wave, then on to the end.
fn frames() -> Vec<(Artwork, Option<ActiveStroke>)> {
    let blank = base();
    let (wave_moments, waved) = drawn(&blank, &wave());
    let (diagonal_moments, crossed) = drawn(&waved, &diagonal());
    let mask = RegionMask::from_pixels(
        blank.canvas(),
        &(0..CANVAS[0] * CANVAS[1])
            .map(|at| (40..120).contains(&(at % CANVAS[0])) && at / CANVAS[0] < 60)
            .collect::<Vec<_>>(),
    )
    .expect("mask");
    let colour = Color::rgb(0.95, 0.75, 0.2).expect("colour");
    let filled = crossed.fill_region(mask, colour).expect("fill").artwork;
    let mut frames = vec![(blank, None)];
    frames.extend(wave_moments.into_iter().map(|(a, s)| (a, Some(s))));
    frames.push((waved.clone(), None));
    frames.extend(diagonal_moments.into_iter().map(|(a, s)| (a, Some(s))));
    frames.extend([(crossed, None), (filled.clone(), None), (waved, None)]);
    frames.push((filled, None));
    frames
}

fn request<'a>(artwork: &'a Artwork, active: Option<&'a ActiveStroke>) -> FrameRequest<'a> {
    FrameRequest {
        artwork,
        active,
        background: Color::WHITE,
    }
}

/// A frame drawn on a video target of its own.
fn cold(artwork: &Artwork, active: Option<&ActiveStroke>, size: [u32; 2]) -> RgbaImage {
    let mut target = VideoTarget::new(core(), CANVAS, size).expect("target");
    target
        .frame(core(), &request(artwork, active))
        .expect("cold frame")
}

#[test]
fn frames_drawn_one_after_another_equal_each_drawn_cold() {
    let core = core();
    let mut target = VideoTarget::new(core, CANVAS, VIDEO).expect("target");
    assert_eq!(target.size(), VIDEO);
    for (index, (artwork, active)) in frames().iter().enumerate() {
        let frame = target
            .frame(core, &request(artwork, active.as_ref()))
            .expect("frame");
        assert_eq!(frame.size(), VIDEO);
        let alone = cold(artwork, active.as_ref(), VIDEO);
        assert!(
            frame == alone,
            "frame {index} differs from its cold drawing"
        );
    }
}

#[test]
fn the_scale_pass_is_its_reference_and_the_canvas_size_the_canvas() {
    let core = core();
    let frames = frames();
    let (artwork, _) = frames.last().expect("frames");
    let canvas = CanvasTarget::whole(core, &request(artwork, None), false).expect("whole");
    for size in [CANVAS, VIDEO, [97, 61], [320, 1], [1, 1]] {
        let frame = cold(artwork, None, size);
        assert!(frame == video_reference(&canvas, size), "at {size:?}");
    }
    assert!(
        cold(artwork, None, CANVAS) == canvas,
        "the canvas's size is the canvas"
    );
}

#[test]
fn the_card_lays_the_last_picture_on_the_desk_above_the_caption() {
    let core = core();
    let frames = frames();
    let (artwork, _) = frames.last().expect("frames");
    let mut target = VideoTarget::new(core, CANVAS, VIDEO).expect("target");
    let band = [VIDEO[0], 14];
    let caption: Vec<u8> = (0..band[0] * band[1])
        .flat_map(|at| [(at % 251) as u8, 40, (at / band[0]) as u8 * 9, 255])
        .collect();
    let caption = RgbaImage::from_rgba(band, caption).expect("caption");
    let refused = target.card(core, (DESK, &caption));
    assert!(refused.is_err(), "no card before a frame");
    target.frame(core, &request(artwork, None)).expect("frame");
    let narrow = RgbaImage::from_rgba([10, 14], vec![0; 10 * 14 * 4]).expect("narrow");
    assert!(
        target.card(core, (DESK, &narrow)).is_err(),
        "a band not across"
    );
    let card = target.card(core, (DESK, &caption)).expect("card");
    assert_eq!(card.size(), VIDEO);
    let pixel = |[x, y]: [u32; 2]| -> [u8; 4] {
        let at = ((y * VIDEO[0] + x) * 4) as usize;
        card.pixels()[at..at + 4].try_into().expect("pixel")
    };
    for corner in [[0, 0], [VIDEO[0] - 1, 0]] {
        assert_eq!(pixel(corner), DESK);
    }
    let rows = (VIDEO[1] - band[1]) as usize * VIDEO[0] as usize * 4;
    assert!(
        card.pixels()[rows..] == *caption.pixels(),
        "the band along the bottom"
    );
    // 120 px high: 40 points of margin are 5 px; the canvas fitted within 182 x 96 is 154 x 96,
    // centred above the band.
    let (at, size) = ([19, 5], [154, 96]);
    let canvas = CanvasTarget::whole(core, &request(artwork, None), false).expect("whole");
    let picture = video_reference(&canvas, size);
    for y in 0..size[1] {
        let from = (((at[1] + y) * VIDEO[0] + at[0]) * 4) as usize;
        let row = (size[0] * 4) as usize;
        let expected = &picture.pixels()[(y as usize * row)..(y as usize + 1) * row];
        assert!(
            card.pixels()[from..from + row] == *expected,
            "picture row {y}"
        );
    }
    let under = pixel([at[0] + size[0] / 2, at[1] + size[1]]);
    assert!(
        under[0] < DESK[0],
        "the shadow under the picture: {under:?}"
    );
}

#[test]
fn a_frame_of_another_canvas_is_refused() {
    let core = core();
    let mut target = VideoTarget::new(core, CANVAS, VIDEO).expect("target");
    let other = Artwork::new(CanvasSize::new(100, 100).expect("size"));
    assert!(target.frame(core, &request(&other, None)).is_err());
    assert!(VideoTarget::new(core, CANVAS, [0, 120]).is_err());
}
