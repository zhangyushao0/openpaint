//! Actions' last line: the app's version and storage format, quiet, under the Language section,
//! nothing to tap; the language's words in either language.

use openpaint_app::{BrushLibrary, Language, SessionProjection, Toolbox, Transfers};

use super::ActionsPanel;
use crate::components::{ControlMap, chrome, controls};
use crate::design;
use crate::text::set_language;

const LINE: &str = "OpenPaint 1.0.0 · format 1";

/// The open panel after a second of frames in `language`: its controls and every text with its
/// top.
fn settled(panel: &mut ActionsPanel, language: Language) -> (ControlMap, Vec<(String, f32)>) {
    let ctx = egui::Context::default();
    design::install(&ctx);
    let toolbox = Toolbox::new(BrushLibrary::built_in().expect("brushes"));
    let projection = SessionProjection::default()
        .with_brush(toolbox.projection())
        .with_transfers(Transfers::default().projection());
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600.0, 1600.0));
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
            set_language(&ctx, language);
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
        version: Some(("1.0.0".to_owned(), 1)),
        ..ActionsPanel::default()
    };
    let (map, texts) = settled(&mut panel, Language::English);
    let pen = map.find("Pen buttons").expect("the Pen section").rect[3];
    let language = map.find("中文").expect("the Language section").rect[3];
    let (_, line) = texts
        .iter()
        .find(|(text, _)| text == LINE)
        .unwrap_or_else(|| panic!("no version line in {texts:?}"));
    assert!(
        *line >= language && language > pen,
        "{pen} < {language} <= {line}"
    );
    assert!(
        texts.iter().all(|(_, top)| top <= line),
        "the last line: {texts:?}"
    );
    assert!(
        map.iter().all(|control| control.name != LINE),
        "nothing to tap"
    );
    // Without a version (a shell nobody gave one) the panel ends with the Language section.
    let mut bare = ActionsPanel {
        open: true,
        ..ActionsPanel::default()
    };
    let (_, texts) = settled(&mut bare, Language::English);
    assert!(texts.iter().all(|(text, _)| !text.starts_with("OpenPaint")));
}

#[test]
fn in_chinese_the_words_change_and_the_controls_keep_their_names() {
    let mut panel = ActionsPanel {
        open: true,
        version: Some(("1.0.0".to_owned(), 1)),
        ..ActionsPanel::default()
    };
    let (english, _) = settled(&mut panel, Language::English);
    let (chinese, texts) = settled(&mut panel, Language::Chinese);
    let names = |map: &ControlMap| map.names();
    assert_eq!(names(&english), names(&chinese));
    for shown in [
        "操作",
        "笔按键",
        "语言",
        "English",
        "中文",
        "OpenPaint 1.0.0 · 格式 1",
    ] {
        assert!(
            texts.iter().any(|(text, _)| text == shown),
            "{shown} in {texts:?}"
        );
    }
    assert!(texts.iter().all(|(text, _)| text != "Pen buttons"));
}
