//! Actions' last line (S52): the app's version and library schema, quiet, under the Pen section,
//! nothing to tap.

use openpaint_app::{BrushLibrary, SessionProjection, Toolbox, Transfers};

use super::ActionsPanel;
use crate::components::{ControlMap, chrome, controls};
use crate::design;

const LINE: &str = "OpenPaint 0.7.0 · schema 22";

/// The open panel after a second of frames: its controls and every text with its top.
fn settled(panel: &mut ActionsPanel) -> (ControlMap, Vec<(String, f32)>) {
    let ctx = egui::Context::default();
    design::install(&ctx);
    let toolbox = Toolbox::new(BrushLibrary::built_in().expect("brushes"));
    let projection = SessionProjection::default()
        .with_brush(toolbox.projection())
        .with_transfers(Transfers::default().projection());
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600.0, 1400.0));
    let mut last = (ControlMap::default(), Vec::new());
    for frame in 0..90 {
        let input = egui::RawInput {
            screen_rect: Some(screen),
            time: Some(f64::from(frame) / 60.0),
            ..Default::default()
        };
        let mut map = ControlMap::default();
        let mut output = ctx.run_ui(input, |ui| {
            let ctx = ui.ctx().clone();
            chrome::begin(&ctx);
            controls::begin(&ctx);
            // The island the panel grows out of, as the shell lays it out.
            let island = egui::Rect::from_min_size(egui::pos2(8.0, 8.0), egui::vec2(240.0, 48.0));
            chrome::island(&ctx, egui::Id::new("left island"), island);
            panel.show(&ctx, &projection, &mut Vec::new());
            chrome::finish(&ctx);
            map = controls::take(&ctx);
        });
        output.textures_delta.clear();
        let texts = output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) => Some((text.galley.text().to_owned(), text.pos.y)),
                _ => None,
            })
            .collect();
        last = (map, texts);
    }
    last
}

#[test]
fn the_version_is_the_quiet_last_line() {
    let mut panel = ActionsPanel {
        open: true,
        version: LINE.to_owned(),
        ..ActionsPanel::default()
    };
    let (map, texts) = settled(&mut panel);
    let pen = map.find("Pen buttons").expect("the Pen section").rect[3];
    let (_, line) = texts
        .iter()
        .find(|(text, _)| text == LINE)
        .unwrap_or_else(|| panic!("no version line in {texts:?}"));
    assert!(*line >= pen, "under Pen buttons: {line} < {pen}");
    assert!(
        texts.iter().all(|(_, top)| top <= line),
        "the last line: {texts:?}"
    );
    assert!(
        map.iter().all(|control| control.name != LINE),
        "nothing to tap"
    );
    // Without a version (a shell nobody gave one) the panel ends with the Pen section.
    let mut bare = ActionsPanel {
        open: true,
        ..ActionsPanel::default()
    };
    let (_, texts) = settled(&mut bare);
    assert!(texts.iter().all(|(text, _)| !text.starts_with("OpenPaint")));
}
