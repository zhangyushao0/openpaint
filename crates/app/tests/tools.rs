//! Tools and colour: the eraser has its own brush and sizes, the colour and its recent ones are
//! editor state that survives a restart, the eyedropper commits its last preview and ends when it
//! picks, and Alpha Lock is an
//! undoable layer change that the eraser respects.

use openpaint_app::{
    BrushIntent, BrushLibrary, Contact, ContactPhase, Device, HistoryIntent, InputSample, Intent,
    LayerIntent, RECENT_COLORS, SMUDGE_STRENGTH, Session, StrokeIntent, Tool, ToolIntent, Toolbox,
};
use openpaint_core::{CanvasSize, Color, StrokeBlend, StrokeTool};

fn toolbox() -> Toolbox {
    Toolbox::new(BrushLibrary::built_in().expect("built-in brushes"))
}

fn rgb(r: f32, g: f32, b: f32) -> Color {
    Color::rgb(r, g, b).expect("colour")
}

fn color(color: Color, settled: bool) -> ToolIntent {
    ToolIntent::Color { color, settled }
}

#[test]
fn the_eraser_keeps_its_own_brush_size_and_opacity() {
    let mut toolbox = toolbox();
    let library = toolbox.library().clone();
    let [pencil, ink, _, _, _, soft, ..] = library.brushes() else {
        panic!("the built-in brushes");
    };
    toolbox.dispatch(&BrushIntent::Choose(pencil.key().clone()));
    toolbox.dispatch_tool(&ToolIntent::Use(Tool::Eraser));
    let eraser = toolbox.projection();
    assert_eq!(
        (eraser.tool, eraser.current),
        (Tool::Eraser, ink.key().clone())
    );
    assert_eq!(toolbox.style().expect("style").tool(), StrokeTool::Erase);
    toolbox.dispatch(&BrushIntent::Choose(soft.key().clone()));
    toolbox.dispatch(&BrushIntent::Opacity {
        opacity: 0.5,
        settled: true,
    });
    toolbox.dispatch_tool(&ToolIntent::Use(Tool::Brush));
    let brush = toolbox.projection();
    assert_eq!((brush.current, brush.opacity), (pencil.key().clone(), 1.0));
    assert_eq!(toolbox.style().expect("style").tool(), StrokeTool::Paint);
    toolbox.dispatch_tool(&ToolIntent::Use(Tool::Eraser));
    let back = toolbox.projection();
    assert_eq!((back.current, back.opacity), (soft.key().clone(), 0.5));
    let mut reopened = self::toolbox();
    assert!(reopened.restore(&toolbox.state()));
    assert_eq!(reopened.projection(), toolbox.projection());
}

#[test]
fn a_settled_colour_paints_joins_the_recent_ones_and_is_stored() {
    let mut toolbox = toolbox();
    assert_eq!(toolbox.projection().color, Color::BLACK);
    toolbox.dispatch_tool(&color(rgb(0.2, 0.4, 0.6), false));
    assert_eq!(toolbox.save_request(), None, "the wheel is still held");
    assert!(toolbox.projection().recent.is_empty());
    let [r, g] = [230, 26].map(|k: u8| f32::from(k) / 255.0);
    let red = rgb(r, g, g);
    toolbox.dispatch_tool(&color(red, true));
    let shown = toolbox.projection();
    assert_eq!(shown.color, red);
    assert_eq!(*shown.recent, [red]);
    assert_eq!(toolbox.style().expect("style").color(), red);
    let state = toolbox.save_request().expect("stored once settled");
    assert_eq!(state.color, [r, g, g]);
    for step in 0..12u8 {
        toolbox.dispatch_tool(&color(rgb(f32::from(step) / 12.0, 0.0, 1.0), true));
    }
    assert_eq!(toolbox.projection().recent.len(), RECENT_COLORS);
    let mut reopened = self::toolbox();
    assert!(reopened.restore(&toolbox.state()));
    assert_eq!(reopened.projection(), toolbox.projection());
}

#[test]
fn the_eyedropper_ends_when_a_colour_settles_or_a_tool_is_chosen() {
    let mut toolbox = toolbox();
    toolbox.dispatch_tool(&ToolIntent::Use(Tool::Eraser));
    toolbox.dispatch_tool(&ToolIntent::Eyedropper(true));
    assert!(toolbox.picking() && toolbox.projection().picking);
    toolbox.dispatch_tool(&color(rgb(0.5, 0.5, 0.0), false));
    assert!(toolbox.picking(), "still looking");
    toolbox.dispatch_tool(&color(rgb(0.5, 0.5, 0.0), true));
    assert!(!toolbox.picking());
    assert_eq!(
        toolbox.projection().tool,
        Tool::Eraser,
        "back to the tool before"
    );
    toolbox.dispatch_tool(&ToolIntent::Eyedropper(true));
    toolbox.dispatch_tool(&ToolIntent::Use(Tool::Brush));
    assert!(!toolbox.picking());
    toolbox.dispatch_tool(&ToolIntent::Eyedropper(true));
    toolbox.dispatch_tool(&ToolIntent::Eyedropper(false));
    assert!(!toolbox.picking());
    assert_eq!(
        toolbox.save_request().map(|state| state.tool),
        Some(Tool::Brush),
        "the eyedropper itself is never stored"
    );
}

#[test]
fn picking_commits_the_last_preview_and_keeps_the_colour_until_then() {
    let mut toolbox = toolbox();
    let (blue, green) = (rgb(0.0, 0.0, 1.0), rgb(0.0, 1.0, 0.0));
    toolbox.dispatch_tool(&ToolIntent::Preview(blue));
    assert_eq!(toolbox.projection().preview, None, "only while picking");
    toolbox.dispatch_tool(&ToolIntent::Eyedropper(true));
    toolbox.dispatch_tool(&ToolIntent::Pick);
    assert!(
        toolbox.picking(),
        "a lift with nothing previewed keeps looking"
    );
    toolbox.dispatch_tool(&ToolIntent::Preview(blue));
    toolbox.dispatch_tool(&ToolIntent::Preview(green));
    let shown = toolbox.projection();
    assert_eq!((shown.preview, shown.color), (Some(green), Color::BLACK));
    // The contact drifted over a control: no new preview; lifting keeps the last one.
    toolbox.dispatch_tool(&ToolIntent::Pick);
    let picked = toolbox.projection();
    assert_eq!((picked.color, picked.preview), (green, None));
    assert_eq!(*picked.recent, [green]);
    assert!(!picked.picking);
    toolbox.dispatch_tool(&ToolIntent::Eyedropper(true));
    toolbox.dispatch_tool(&ToolIntent::Preview(blue));
    toolbox.dispatch_tool(&ToolIntent::Eyedropper(false));
    toolbox.dispatch_tool(&ToolIntent::Pick);
    assert_eq!(
        toolbox.projection().color,
        green,
        "turned off: nothing picked"
    );
}

#[test]
fn the_smudge_has_its_own_brush_and_strength_and_is_stored() {
    let mut toolbox = toolbox();
    let library = toolbox.library().clone();
    let [_, ink, _, _, _, soft, ..] = library.brushes() else {
        panic!("the built-in brushes");
    };
    toolbox.dispatch_tool(&ToolIntent::Use(Tool::Smudge));
    let smudge = toolbox.projection();
    assert_eq!(
        (smudge.tool, smudge.current),
        (Tool::Smudge, soft.key().clone())
    );
    assert_eq!(smudge.opacity, SMUDGE_STRENGTH, "draweave's 70 %");
    let style = toolbox.style().expect("style");
    assert_eq!(
        (style.tool(), style.opacity()),
        (StrokeTool::Smudge, SMUDGE_STRENGTH)
    );
    toolbox.dispatch(&BrushIntent::Choose(ink.key().clone()));
    toolbox.dispatch(&BrushIntent::Opacity {
        opacity: 0.4,
        settled: true,
    });
    toolbox.dispatch_tool(&ToolIntent::Use(Tool::Brush));
    let brush = toolbox.projection();
    assert_eq!(
        (brush.current, brush.opacity),
        (ink.key().clone(), 1.0),
        "the brush keeps its own"
    );
    let state = toolbox.save_request().expect("changed");
    assert_eq!(state.smudge.brush, ink.key().as_str());
    let mut reopened = self::toolbox();
    assert!(reopened.restore(&state));
    reopened.dispatch_tool(&ToolIntent::Use(Tool::Smudge));
    let back = reopened.projection();
    assert_eq!((back.current, back.opacity), (ink.key().clone(), 0.4));
}

#[test]
fn a_smudge_ends_where_its_line_got_to_and_moves_colour_on_a_locked_layer() {
    let mut toolbox = toolbox();
    let canvas = CanvasSize::new(200, 100).expect("size");
    let mut session = Session::new(canvas, toolbox.style().expect("style"));
    draw(&mut session).expect("paint");
    let layer = session.artwork().active();
    let lock = Intent::Layer(LayerIntent::SetAlphaLock {
        layer,
        locked: true,
    });
    session.dispatch(lock).expect("lock");
    toolbox.dispatch_tool(&ToolIntent::Use(Tool::Smudge));
    session.set_brush(toolbox.style().expect("style"));
    session.set_streamline(openpaint_app::StreamLine::new(1.0).expect("amount"));
    draw(&mut session).expect("smudge an alpha-locked layer");
    let strokes = session.artwork().layers()[0].strokes();
    let smudge = strokes.last().expect("the smudge");
    assert_eq!(smudge.blend(), StrokeBlend::SmudgeLocked);
    let end = smudge.samples().last().expect("samples").position();
    assert!(end.x() < 50.0, "the line stays behind the pen: {}", end.x());
}

fn pen(phase: ContactPhase, x: f32, time_ms: i64) -> Intent {
    let sample = InputSample {
        position: [x, 40.0],
        pressure: 0.7,
        tilt: 0.0,
        azimuth: 0.0,
        time_ms,
    };
    Intent::Stroke(StrokeIntent::Contact(Contact {
        id: 1,
        device: Device::Pen,
        phase,
        samples: vec![sample],
    }))
}

fn draw(session: &mut Session) -> Result<(), openpaint_app::AppError> {
    session.dispatch(pen(ContactPhase::Down, 10.0, 0))?;
    session.dispatch(pen(ContactPhase::Move, 30.0, 8))?;
    session.dispatch(pen(ContactPhase::Up, 50.0, 16))
}

#[test]
fn alpha_lock_is_an_undoable_layer_change_the_eraser_respects() {
    let mut toolbox = toolbox();
    let canvas = CanvasSize::new(200, 100).expect("size");
    let mut session = Session::new(canvas, toolbox.style().expect("style"));
    draw(&mut session).expect("paint");
    let layer = session.artwork().active();
    let lock = |locked| Intent::Layer(LayerIntent::SetAlphaLock { layer, locked });
    session.dispatch(lock(true)).expect("lock");
    let rows = session.projection().layers().expect("layers").rows.clone();
    assert!(rows[0].alpha_locked);
    draw(&mut session).expect("paint atop");
    toolbox.dispatch_tool(&ToolIntent::Use(Tool::Eraser));
    session.set_brush(toolbox.style().expect("style"));
    assert!(draw(&mut session).is_err(), "nothing to take away");
    let blends: Vec<StrokeBlend> = session.artwork().layers()[0]
        .strokes()
        .iter()
        .map(|stroke| stroke.blend())
        .collect();
    assert_eq!(blends, [StrokeBlend::Over, StrokeBlend::Atop]);
    session.dispatch(lock(false)).expect("unlock");
    draw(&mut session).expect("erase");
    session
        .dispatch(Intent::History(HistoryIntent::Undo))
        .expect("undo the eraser");
    session
        .dispatch(Intent::History(HistoryIntent::Undo))
        .expect("undo the unlock");
    let props = *session.artwork().layers()[0].props();
    assert!(props.alpha_locked(), "undo brings the lock back");
}
