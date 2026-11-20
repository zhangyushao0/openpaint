//! Actions' last line: the app's version and storage format, quiet, under the Language section,
//! nothing to tap; the language's words in either language. Share's Replay… row with the
//! record's strokes (S163), none on a mode's sheet; the panel on the tablet's screen.

use openpaint_app::{
    Availability, BrushLibrary, Intent, Language, ReplayIntent, ReplayShown, SessionProjection,
    Toolbox, Transfers,
};

use super::ActionsPanel;
use crate::components::{ControlMap, chrome, controls};
use crate::design;
use crate::text::set_language;

const LINE: &str = "OpenPaint 1.0.0 · format 1";

/// A screen: its size in points and its density.
type Screen = (egui::Vec2, f32);
const SQUARE: Screen = (egui::vec2(1600.0, 1600.0), 1.0);
/// The tablet's 2880 × 1800 pixels.
const TABLET: Screen = (egui::vec2(2880.0 / 1.75, 1800.0 / 1.75), 1.75);

/// A canvas's projection with `replay` as Actions' row.
fn projection(replay: ReplayShown) -> SessionProjection {
    let toolbox = Toolbox::new(BrushLibrary::built_in().expect("brushes"));
    SessionProjection::default()
        .with_brush(toolbox.projection())
        .with_transfers(Transfers::default().projection())
        .with_replay(replay)
}

/// The open panel over `projection` after a second of frames in `language` on `screen`, the last
/// with `events`: its controls, every text with where it lies (points), and what it asked for.
fn laid(
    panel: &mut ActionsPanel,
    (projection, language): (&SessionProjection, Language),
    (points, ppp): Screen,
    events: &[egui::Event],
) -> (ControlMap, Vec<(String, egui::Rect)>, Vec<Intent>) {
    let ctx = egui::Context::default();
    design::install(&ctx);
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, points);
    let mut last = (ControlMap::default(), Vec::new(), Vec::new());
    for frame in 0..90 {
        let mut input = egui::RawInput {
            screen_rect: Some(screen),
            time: Some(f64::from(frame) / 60.0),
            events: if frame == 89 {
                events.to_vec()
            } else {
                Vec::new()
            },
            ..Default::default()
        };
        let root = input.viewports.entry(egui::ViewportId::ROOT).or_default();
        root.native_pixels_per_point = Some(ppp);
        let (mut map, mut taps) = (ControlMap::default(), Vec::new());
        let mut output = ctx.run_ui(input, |ui| {
            let ctx = ui.ctx().clone();
            set_language(&ctx, language);
            chrome::begin(&ctx);
            controls::begin(&ctx);
            // The island the panel grows out of, as the shell lays it out.
            let island = egui::Rect::from_min_size(egui::pos2(8.0, 8.0), egui::vec2(240.0, 48.0));
            chrome::island(&ctx, egui::Id::new("left island"), island);
            panel.show(&ctx, projection, &mut taps);
            chrome::finish(&ctx);
            map = controls::take(&ctx);
        });
        output.textures_delta.clear();
        let texts = output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) => {
                    let rect = text.galley.rect.translate(text.pos.to_vec2());
                    Some((text.galley.text().to_owned(), rect))
                }
                _ => None,
            })
            .collect();
        last = (map, texts, taps);
    }
    last
}

/// The open panel without a replay's row on a square screen: its controls and every text with
/// its top.
fn settled(panel: &mut ActionsPanel, language: Language) -> (ControlMap, Vec<(String, f32)>) {
    let shown = (&projection(ReplayShown::None), language);
    let (map, texts, _) = laid(panel, shown, SQUARE, &[]);
    let tops = texts.into_iter().map(|(text, rect)| (text, rect.top()));
    (map, tops.collect())
}

/// A tap at the middle of the control `name` of `map` on a screen of `ppp`.
fn tap(map: &ControlMap, name: &str, ppp: f32) -> Vec<egui::Event> {
    let [x, y] = map.find(name).expect("the control").at([0.5, 0.5]);
    let at = egui::pos2(x, y) / ppp;
    let button = |pressed| egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    vec![egui::Event::PointerMoved(at), button(true), button(false)]
}

fn open() -> ActionsPanel {
    ActionsPanel {
        open: true,
        version: Some(("1.0.0".to_owned(), 1)),
        ..ActionsPanel::default()
    }
}

#[test]
fn the_version_is_the_quiet_last_line() {
    let mut panel = open();
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
    let mut panel = open();
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

#[test]
fn share_offers_the_replay_with_its_strokes_and_a_tap_opens_it() {
    let offered = projection(ReplayShown::Offered {
        strokes: 1288,
        availability: Availability::Available,
    });
    for (language, words) in [
        (Language::English, ["Replay…", "1,288 strokes"]),
        (Language::Chinese, ["回放…", "1,288 笔"]),
    ] {
        let mut panel = open();
        let (map, texts, _) = laid(&mut panel, (&offered, language), SQUARE, &[]);
        assert!(map.find("Replay…").is_ok(), "named by its English");
        let shown = |word: &str| texts.iter().any(|(text, _)| text == word);
        assert!(words.iter().all(|word| shown(word)), "{texts:?}");
        let share = map.find("Share JPEG…").expect("Share's rows").rect[3];
        assert!(
            map.find("Replay…").expect("the row").rect[1] >= share,
            "Share's last row"
        );
        let events = tap(&map, "Replay…", 1.0);
        let (_, _, taps) = laid(&mut panel, (&offered, language), SQUARE, &events);
        assert_eq!(taps, [Intent::Replay(ReplayIntent::Open)]);
        assert!(!panel.open, "the replay takes the screen");
    }
    // Nothing to replay: the row greyed, its tap asks nothing.
    let empty = projection(ReplayShown::Offered {
        strokes: 0,
        availability: Availability::NothingLeft,
    });
    let mut panel = open();
    let (map, _, _) = laid(&mut panel, (&empty, Language::English), SQUARE, &[]);
    let events = tap(&map, "Replay…", 1.0);
    let (_, _, taps) = laid(&mut panel, (&empty, Language::English), SQUARE, &events);
    assert!(taps.is_empty() && panel.open);
    // A mode's sheet: no row.
    let (map, _) = settled(&mut open(), Language::English);
    assert!(map.find("Replay…").is_err());
}

#[test]
fn on_the_tablet_the_panel_ends_on_the_screen() {
    let offered = projection(ReplayShown::Offered {
        strokes: 1288,
        availability: Availability::Available,
    });
    for language in Language::ALL {
        let mut panel = open();
        let (map, texts, _) = laid(&mut panel, (&offered, language), TABLET, &[]);
        let bottom = TABLET.0.y;
        for control in map.iter() {
            assert!(control.rect[3] / TABLET.1 <= bottom, "{}", control.name);
        }
        for (text, rect) in &texts {
            assert!(rect.bottom() <= bottom, "{text} at {rect:?}");
        }
    }
}
