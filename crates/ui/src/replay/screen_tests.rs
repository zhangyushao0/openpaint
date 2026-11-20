//! The replay's screen at the tablet's size (2880 × 1800 pixels at 1.75), driven through the
//! shell as the runtime drives it: its controls by name in both languages and on screen, nothing
//! of the studio, the canvas at fit above the bar, the track's seek and the segment's handles, the
//! pen doing nothing, Back; the video's last line. The screen these tests and the export panel's
//! (`export_tests.rs`) drive.

use openpaint_app::{
    Availability, BrushLibrary, Contact, ContactPhase, Device, ExportChoices, InputBatch,
    InputSample, Intent, Language, ReplayFacts, ReplayIntent, ReplayProjection, ReplayShown,
    ScreenProjection, Session, Speed, StrokeLabel, Toolbox, Transfers, VideoLength, VideoNotice,
    VideoSize,
};
use openpaint_core::CanvasSize;

use super::{caption_band, video_caption};
use crate::components::{ControlMap, TRACK_OVER};
use crate::dates::Date;
use crate::design::{self, SPACE, TextFonts};
use crate::hooks::Hooks;
use crate::shell::Shell;

pub(super) const PPP: f32 = 1.75;
const PIXELS: [f32; 2] = [2880.0, 1800.0];

/// The texts a pass painted, each with where it lies (points).
pub(super) type Texts = Vec<(String, egui::Rect)>;

/// The shell on a tablet-sized screen, one pass at a time at 60 a second.
pub(super) struct Screen {
    ctx: egui::Context,
    shell: Shell,
    frame: u32,
    pub(super) map: ControlMap,
}

impl Screen {
    pub(super) fn new(language: Language) -> Self {
        let ctx = egui::Context::default();
        design::install(&ctx);
        let mut shell = Shell::default();
        shell.set_language(language);
        Self {
            ctx,
            shell,
            frame: 0,
            map: ControlMap::default(),
        }
    }

    /// One pass over `projection` with `events`, then this frame's `contacts` routed: the
    /// intents, the controls and the texts.
    fn pass(
        &mut self,
        projection: &ScreenProjection,
        (events, contacts): (Vec<egui::Event>, &[InputBatch]),
    ) -> (Vec<Intent>, Texts) {
        let points = egui::vec2(PIXELS[0], PIXELS[1]) / PPP;
        let mut input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, points)),
            events,
            time: Some(f64::from(self.frame) / 60.0),
            ..Default::default()
        };
        let root = input.viewports.entry(egui::ViewportId::ROOT).or_default();
        root.native_pixels_per_point = Some(PPP);
        self.frame += 1;
        let shell = &mut self.shell;
        let mut output = self
            .ctx
            .run_ui(input, |ui| shell.show(ui, projection, Hooks::default()));
        output.textures_delta.clear();
        self.map = self.shell.controls().clone();
        let intents = self.shell.route(contacts, Hooks::default());
        let texts = (output.shapes.iter())
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) => {
                    let rect = text.galley.rect.translate(text.pos.to_vec2());
                    Some((text.galley.text().to_owned(), rect))
                }
                _ => None,
            })
            .collect();
        (intents, texts)
    }

    /// A second of passes: everything that moves at rest.
    pub(super) fn settle(&mut self, projection: &ScreenProjection) -> Texts {
        let mut texts = Vec::new();
        for _ in 0..60 {
            texts = self.pass(projection, (Vec::new(), &[])).1;
        }
        texts
    }

    /// A tap on the control `name` of the last pass: what it asked for.
    pub(super) fn tap(&mut self, projection: &ScreenProjection, name: &str) -> Vec<Intent> {
        let control = (self.map.find(name)).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        let [x, y] = control.at([0.5, 0.5]);
        let events = press(egui::pos2(x, y) / PPP, &[true, false]);
        let (intents, _) = self.pass(projection, (events, &[]));
        self.settle(projection);
        intents
    }
}

fn press(at: egui::Pos2, pressed: &[bool]) -> Vec<egui::Event> {
    let button = |pressed| egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    let mut events = vec![egui::Event::PointerMoved(at)];
    events.extend(pressed.iter().map(|&down| button(down)));
    events
}

/// 14 Nov 2026 21:07 UTC, in ms.
fn began() -> u64 {
    let day = Date {
        year: 2026,
        month: 11,
        day: 14,
    };
    u64::try_from(day.days()).expect("after 1970") * 86_400_000 + (21 * 3600 + 7 * 60) * 1000
}

/// A replay as the mock shows it: playing at 4×, 1:24 of 3:18, two sittings, stroke 412.
pub(super) fn replay() -> ReplayProjection {
    ReplayProjection {
        playing: true,
        speed: Speed::Four,
        at_ms: 84_400,
        total_ms: 198_000,
        at: 0.42,
        sittings: vec![0.33, 0.61],
        stroke: Some(StrokeLabel {
            number: 412,
            began_ms: Some(began()),
        }),
        tip: Some([1400.0, 900.0]),
        export: ExportChoices {
            length: VideoLength::Highlight,
            lengths_ms: [30_000, 201_000, 51_000],
            segment: [0.26, 0.51],
            segment_ms: [52_000, 100_000],
            size: VideoSize::Hd,
            sizes: [Some([1728, 1080]), Some([2880, 1800])],
            hold: true,
            caption: true,
        },
        video: VideoNotice::None,
        controls: Availability::Available,
    }
}

/// The canvas screen of a 2880 × 1800 artwork with `replay` open.
pub(super) fn open(replay: ReplayProjection) -> ScreenProjection {
    let toolbox = Toolbox::new(BrushLibrary::built_in().expect("brushes"));
    let canvas = CanvasSize::new(2880, 1800).expect("a size");
    let session = Session::new(canvas, toolbox.style().expect("a style"))
        .projection()
        .with_brush(toolbox.projection())
        .with_transfers(Transfers::default().projection())
        .with_replay(ReplayShown::Open(Box::new(replay)));
    ScreenProjection::Canvas(Box::new(session))
}

pub(super) fn asked(intent: ReplayIntent) -> Vec<Intent> {
    vec![Intent::Replay(intent)]
}

/// Every control lies on the screen; every text too, none over another.
pub(super) fn nothing_clipped(map: &ControlMap, texts: &Texts) {
    for control in map.iter() {
        let [left, top, right, bottom] = control.rect;
        let inside = left >= 0.0 && top >= 0.0 && right <= PIXELS[0] && bottom <= PIXELS[1];
        assert!(inside, "{} at {:?}", control.name, control.rect);
    }
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(PIXELS[0], PIXELS[1]));
    for (index, (text, rect)) in texts.iter().enumerate() {
        assert!(screen.contains_rect(*rect * PPP), "{text} at {rect:?}");
        for (other, at) in &texts[index + 1..] {
            let overlap = rect.intersect(*at);
            let apart = overlap.width() <= 0.5 || overlap.height() <= 0.5;
            assert!(apart, "{text} over {other}");
        }
    }
}

pub(super) fn has(texts: &Texts, shown: &str) -> bool {
    texts.iter().any(|(text, _)| text == shown)
}

#[test]
fn the_replay_shows_its_own_chrome_at_the_tablet_s_size_in_both_languages() {
    let projection = open(replay());
    let mut names = Vec::new();
    for (language, stroke) in [
        (Language::English, "14 Nov 21:07 · stroke 412"),
        (Language::Chinese, "11月14日 21:07 · 第 412 笔"),
    ] {
        let mut screen = Screen::new(language);
        let texts = screen.settle(&projection);
        let map = screen.map.clone();
        for name in [
            "Back",
            "Export video…",
            "Pause",
            "Replay track",
            "1×",
            "4×",
            "16×",
        ] {
            assert!(map.find(name).is_ok(), "{name} in {:?}", map.names());
        }
        for studio in ["Actions", "Undo", "Layers", "Brush", "Gallery"] {
            assert!(map.find(studio).is_err(), "no {studio}");
        }
        assert!(
            has(&texts, stroke) && has(&texts, "1:24 / 3:18"),
            "{texts:?}"
        );
        nothing_clipped(&map, &texts);
        // The canvas fits between the top band and the bar, and shows there whatever the view.
        let area = screen.shell.canvas_area().expect("a canvas area");
        let play = map.find("Pause").expect("the bar");
        let bar_top = map.find("Replay track").expect("the track").rect[1] - TRACK_OVER * PPP;
        let fit_bottom = (area.fit.origin[1] + area.fit.size[1] as i32) as f32;
        assert!(
            fit_bottom <= bar_top.min(play.rect[1]) - SPACE,
            "{fit_bottom} {bar_top}"
        );
        let view = area.viewport(PPP);
        let fit =
            openpaint_app::CanvasView::FIT.mapping(CanvasSize::new(2880, 1800).unwrap(), &view);
        let zoomed =
            openpaint_app::CanvasView::FIT.mapping(CanvasSize::new(100, 100).unwrap(), &view);
        assert_eq!(screen.shell.displayed(zoomed), fit);
        let tip = screen
            .shell
            .cursor(None)
            .expect("the ring at the pen's tip");
        assert_eq!(tip.centre, fit.to_screen([1400.0, 900.0]));
        names.push(map.names());
    }
    assert_eq!(names[0], names[1], "named by their English in both");
}

#[test]
fn with_a_segment_chosen_its_handles_move_and_the_bar_tells_its_range() {
    let mut chosen = replay();
    chosen.export.length = VideoLength::Segment;
    let projection = open(chosen);
    let mut screen = Screen::new(Language::English);
    screen.settle(&projection);
    assert!(
        screen.map.find("Segment start").is_err(),
        "no handles with the panel closed"
    );
    let track = screen.map.find("Replay track").expect("the track").clone();
    let [x, y] = track.at([0.75, 0.5]);
    let seek = screen.pass(
        &projection,
        (press(egui::pos2(x, y) / PPP, &[true, false]), &[]),
    );
    match seek.0.as_slice() {
        [Intent::Replay(ReplayIntent::Seek(at))] => assert!((at - 0.75).abs() < 0.02, "{at}"),
        other => panic!("a seek, not {other:?}"),
    }
    screen.tap(&projection, "Export video…");
    let texts = screen.settle(&projection);
    assert!(has(&texts, "0:52 – 1:40 · 48 s"), "{texts:?}");
    let start = screen
        .map
        .find("Segment start")
        .expect("its handle")
        .at([0.5, 0.5]);
    let from = egui::pos2(start[0], start[1]) / PPP;
    screen.pass(&projection, (press(from, &[true]), &[]));
    let moved = vec![egui::Event::PointerMoved(from + egui::vec2(60.0, 0.0))];
    let (intents, _) = screen.pass(&projection, (moved, &[]));
    match intents.as_slice() {
        [Intent::Replay(ReplayIntent::Segment([a, b]))] => {
            assert!(*a > 0.26 && *a < *b, "{a}");
            assert!((b - 0.51).abs() < f32::EPSILON, "the end stays");
        }
        other => panic!("the segment, not {other:?}"),
    }
}

#[test]
fn the_pen_on_the_canvas_does_nothing_and_back_closes_the_replay() {
    let projection = open(replay());
    let mut screen = Screen::new(Language::English);
    screen.settle(&projection);
    let pen = |phase| InputBatch {
        contacts: vec![Contact {
            id: 1,
            device: Device::Pen,
            phase,
            samples: vec![InputSample {
                position: [1440.0, 900.0],
                pressure: 0.5,
                tilt: 0.0,
                azimuth: 0.0,
                time_ms: 0,
            }],
        }],
        pen: None,
    };
    for phase in [ContactPhase::Down, ContactPhase::Move, ContactPhase::Up] {
        let (intents, _) = screen.pass(&projection, (Vec::new(), &[pen(phase)]));
        assert!(intents.is_empty(), "{phase:?}: {intents:?}");
    }
    assert!(screen.shell.back(&projection, Hooks::default()));
    let (intents, _) = screen.pass(&projection, (Vec::new(), &[]));
    assert_eq!(intents, asked(ReplayIntent::Close));
    assert_eq!(screen.tap(&projection, "Back"), asked(ReplayIntent::Close));
    let mut making = replay();
    making.controls = Availability::Running;
    let projection = open(making);
    screen.settle(&projection);
    assert!(
        screen.shell.back(&projection, Hooks::default()),
        "used, never the system's"
    );
    let (intents, _) = screen.pass(&projection, (Vec::new(), &[]));
    assert!(intents.is_empty(), "Back waits for the video");
}

#[test]
fn the_last_line_is_a_band_of_its_own_with_the_words_inside_it() {
    assert_eq!(caption_band([1920, 1080]), [1920, 134]);
    assert_eq!(caption_band([1080, 1920]), [1080, 240]);
    let facts = ReplayFacts {
        strokes: 1288,
        layers: 6,
        spent_ms: Some(11_520_000),
        began_ms: Some([began(), began() + 4 * 86_400_000]),
    };
    for (language, band) in [
        (Language::English, [1920, 134]),
        (Language::Chinese, [1080, 240]),
    ] {
        let mut frame = video_caption(&TextFonts::default(), &facts, (language, 480), band);
        assert!((frame.pixels_per_point - 1.0).abs() < f32::EPSILON);
        assert!(
            !frame.textures.set.is_empty(),
            "the font atlas comes with it"
        );
        // The renderer applies them; here they are only looked at.
        frame.textures.clear();
        let [width, height] = band.map(|side| side as f32);
        let mut vertices = 0;
        for primitive in &frame.primitives {
            let egui::epaint::Primitive::Mesh(mesh) = &primitive.primitive else {
                panic!("meshes only");
            };
            for vertex in &mesh.vertices {
                let [x, y] = [vertex.pos.x, vertex.pos.y];
                assert!((-0.5..=width + 0.5).contains(&x) && (-0.5..=height + 0.5).contains(&y));
            }
            vertices += mesh.vertices.len();
        }
        assert!(vertices > 100, "the desk and the words: {vertices}");
    }
}
