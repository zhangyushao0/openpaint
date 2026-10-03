use egui::{Id, Pos2};
use openpaint_core::Color;

use super::*;
use crate::components::loupe;

/// One pass at `time` seconds, the loupe shown at `contact` or not.
fn pass(ctx: &egui::Context, time: f64, contact: Option<Pos2>) -> Option<f32> {
    let input = egui::RawInput {
        time: Some(time),
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1600.0, 1000.0))),
        ..Default::default()
    };
    let mut output = ctx.run_ui(input, |ui| {
        let ctx = ui.ctx().clone();
        begin(&ctx);
        if let Some(contact) = contact {
            loupe(&ctx, contact, Color::WHITE, Color::WHITE);
        }
        finish(&ctx);
    });
    output.textures_delta.clear();
    state(ctx, |state| {
        let id = Id::new("loupe");
        let entry = state.entries.iter().find(|entry| entry.id == id)?;
        Some(entry.growth.grown())
    })
}

#[test]
fn a_shown_surface_grows_and_one_no_longer_shown_goes_back_and_away() {
    let ctx = egui::Context::default();
    let at = Pos2::new(800.0, 500.0);
    let mut time = 10.0;
    assert_eq!(
        pass(&ctx, time, Some(at)),
        Some(0.0),
        "it starts at its source"
    );
    for _ in 0..60 {
        time += 1.0 / 60.0;
        pass(&ctx, time, Some(at));
    }
    assert_eq!(
        pass(&ctx, time, Some(at)),
        Some(1.0),
        "grown within a second"
    );
    // At rest until now, it starts moving from this frame on.
    for _ in 0..2 {
        time += 1.0 / 60.0;
        pass(&ctx, time, None);
    }
    let closing = pass(&ctx, time, None).expect("still going back");
    assert!(closing < 1.0);
    for _ in 0..60 {
        time += 1.0 / 60.0;
        pass(&ctx, time, None);
    }
    assert_eq!(
        pass(&ctx, time + 0.1, None),
        None,
        "gone once back at its source"
    );
    // Shown only briefly, it turns back halfway and still goes away.
    for _ in 0..5 {
        time += 1.0 / 60.0;
        pass(&ctx, time, Some(at));
    }
    for _ in 0..60 {
        time += 1.0 / 60.0;
        pass(&ctx, time, None);
    }
    assert_eq!(pass(&ctx, time + 0.1, None), None);
}
