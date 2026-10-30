//! A colour kind's demonstration's little wheel (`practice_demo_marks.rs`, S92): its pick where
//! the demonstration has moved it (round the ring the short way, then across and up or down the
//! square), Compare's original ringed in red.

use egui::Rect;
use openpaint_app::Hsv;
use openpaint_practice::{Demo, Drawn, On};

use crate::components::wheel_picture;
use crate::practice_demo::drawn;
use crate::practice_demo_marks::colour_of;

/// The colour wheel beside a colour's frames, `seconds` after the start, in `rect`.
pub(crate) fn wheel(painter: &egui::Painter, demo: &Demo, seconds: f64, rect: Rect) {
    let mut pick: Option<[f64; 3]> = None;
    let mut ringed = Vec::new();
    for mark in demo.marks.iter().filter(|mark| mark.on == On::Wheel) {
        let shown = drawn(mark, seconds);
        if shown <= 0.0 {
            continue;
        }
        match mark.drawn {
            Drawn::Pick(to) => pick = Some(pick.map_or(to, |from| moved(from, to, shown))),
            Drawn::Ringed(colour) => {
                ringed.push((hsv(colour), colour_of(mark.pen).gamma_multiply(shown)));
            }
            _ => {}
        }
    }
    if let Some(pick) = pick {
        wheel_picture(painter, rect, hsv(pick), &ringed);
    }
}

/// The pick `shown` of the way from `from` to `to`, eased: the hue the short way round.
fn moved(from: [f64; 3], to: [f64; 3], shown: f32) -> [f64; 3] {
    let t = f64::from(shown);
    let t = t * t * (3.0 - 2.0 * t);
    let turn = (to[0] - from[0] + 540.0).rem_euclid(360.0) - 180.0;
    [
        (from[0] + turn * t).rem_euclid(360.0),
        from[1] + (to[1] - from[1]) * t,
        from[2] + (to[2] - from[2]) * t,
    ]
}

/// A hue in degrees, saturation and value as the wheel takes them.
fn hsv([h, s, v]: [f64; 3]) -> Hsv {
    Hsv {
        h: (h / 360.0) as f32,
        s: s.clamp(0.0, 1.0) as f32,
        v: v.clamp(0.0, 1.0) as f32,
    }
}
