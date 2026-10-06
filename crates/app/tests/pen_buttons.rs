//! The pen's buttons (S41): held tools come back on release (after the lift when released on
//! the glass), Undo, Redo and Switch tool act once per press, the eraser end erases, and the
//! toolbox stores the tool chosen and the map, never a held tool.

use openpaint_app::{
    BrushLibrary, ButtonAction, ButtonMap, ButtonTracker, Contact, ContactPhase, Device, HeldTool,
    HistoryIntent, InputBatch, InputSample, Intent, PenButton, PenButtons, Tool, ToolIntent,
    Toolbox,
};
use openpaint_core::{Color, StrokeTool};

const NONE: PenButtons = PenButtons {
    primary: false,
    secondary: false,
};
const FIRST: PenButtons = PenButtons {
    primary: true,
    secondary: false,
};
const SECOND: PenButtons = PenButtons {
    primary: false,
    secondary: true,
};

fn batch(device: Device, phase: ContactPhase, buttons: PenButtons) -> InputBatch {
    let sample = InputSample {
        position: [10.0, 10.0],
        pressure: if phase == ContactPhase::Hover {
            0.0
        } else {
            0.5
        },
        tilt: 0.0,
        azimuth: 0.0,
        time_ms: 0,
    };
    InputBatch {
        contacts: vec![Contact {
            id: 0,
            device,
            phase,
            samples: vec![sample],
        }],
        pen: Some(buttons),
    }
}

fn pen(phase: ContactPhase, buttons: PenButtons) -> InputBatch {
    batch(Device::Pen, phase, buttons)
}

/// The tracker around `batches` as the canvas routes them: what comes before each batch's
/// contacts, a marker for the contacts, what comes after. Applies tool intents to `toolbox`.
struct Run {
    tracker: ButtonTracker,
    toolbox: Toolbox,
    map: ButtonMap,
}

impl Run {
    fn new(map: ButtonMap) -> Self {
        let library = BrushLibrary::built_in().expect("built-in brushes");
        Self {
            tracker: ButtonTracker::default(),
            toolbox: Toolbox::new(library),
            map,
        }
    }

    /// The intents around `batch`, `None` standing for its contacts.
    fn step(&mut self, batch: &InputBatch) -> Vec<Option<Intent>> {
        let mut out: Vec<Option<Intent>> = Vec::new();
        out.extend(self.tracker.before(batch, self.map).into_iter().map(Some));
        out.push(None);
        out.extend(self.tracker.after(batch, self.map).into_iter().map(Some));
        for intent in out.iter().flatten() {
            if let Intent::Tool(tool) = intent {
                self.toolbox.dispatch_tool(tool);
            }
        }
        out
    }

    fn tool(&self) -> StrokeTool {
        self.toolbox.style().expect("style").tool()
    }
}

fn hold(tool: Option<HeldTool>) -> Option<Intent> {
    Some(Intent::Tool(ToolIntent::Hold(tool)))
}

#[test]
fn the_first_button_held_while_hovering_erases_until_it_is_released() {
    let mut run = Run::new(ButtonMap::default());
    assert_eq!(run.step(&pen(ContactPhase::Hover, NONE)), [None]);
    let pressed = run.step(&pen(ContactPhase::Hover, FIRST));
    assert_eq!(pressed, [hold(Some(HeldTool::Eraser)), None]);
    assert_eq!(run.tool(), StrokeTool::Erase);
    assert_eq!(
        run.toolbox.projection().tool,
        Tool::Eraser,
        "the ring shows it"
    );
    for phase in [ContactPhase::Down, ContactPhase::Move, ContactPhase::Up] {
        assert_eq!(run.step(&pen(phase, FIRST)), [None], "still held");
        assert_eq!(run.tool(), StrokeTool::Erase);
    }
    let released = run.step(&pen(ContactPhase::Hover, NONE));
    assert_eq!(released, [hold(None), None]);
    assert_eq!(
        run.tool(),
        StrokeTool::Paint,
        "the brush chosen before comes back"
    );
    assert_eq!(
        run.toolbox.state().tool,
        Tool::Brush,
        "a held tool is never stored"
    );
}

#[test]
fn a_button_pressed_with_the_touch_down_holds_before_the_contact() {
    let mut run = Run::new(ButtonMap::default());
    let down = run.step(&pen(ContactPhase::Down, FIRST));
    assert_eq!(
        down,
        [hold(Some(HeldTool::Eraser)), None],
        "the stroke erases"
    );
}

#[test]
fn a_release_on_the_glass_waits_for_the_lift() {
    let mut run = Run::new(ButtonMap::default());
    run.step(&pen(ContactPhase::Hover, FIRST));
    run.step(&pen(ContactPhase::Down, FIRST));
    assert_eq!(run.step(&pen(ContactPhase::Move, NONE)), [None]);
    assert_eq!(
        run.tool(),
        StrokeTool::Erase,
        "the stroke is never cut in two"
    );
    let lifted = run.step(&pen(ContactPhase::Up, NONE));
    assert_eq!(lifted, [None, hold(None)], "after the contacts lifted");
    assert_eq!(run.tool(), StrokeTool::Paint);
}

#[test]
fn a_press_on_the_glass_holds_from_the_next_contact() {
    let mut run = Run::new(ButtonMap::default());
    run.step(&pen(ContactPhase::Down, NONE));
    assert_eq!(run.step(&pen(ContactPhase::Move, FIRST)), [None]);
    assert_eq!(
        run.tool(),
        StrokeTool::Paint,
        "the stroke drawn goes on painting"
    );
    let lifted = run.step(&pen(ContactPhase::Up, FIRST));
    assert_eq!(lifted, [None, hold(Some(HeldTool::Eraser))]);
    assert_eq!(run.step(&pen(ContactPhase::Down, FIRST)), [None]);
    assert_eq!(run.tool(), StrokeTool::Erase);
}

#[test]
fn the_second_button_picks_colours_while_held_and_every_contact_picks() {
    let mut run = Run::new(ButtonMap::default());
    run.step(&pen(ContactPhase::Hover, SECOND));
    assert!(run.toolbox.picking());
    let red = Color::rgb(1.0, 0.0, 0.0).expect("red");
    let blue = Color::rgb(0.0, 0.0, 1.0).expect("blue");
    for color in [red, blue] {
        run.toolbox.dispatch_tool(&ToolIntent::Preview(color));
        run.toolbox.dispatch_tool(&ToolIntent::Pick);
        assert_eq!(run.toolbox.projection().color, color);
        assert!(
            run.toolbox.picking(),
            "still held: the next contact picks too"
        );
    }
    run.step(&pen(ContactPhase::Hover, NONE));
    assert!(!run.toolbox.picking());
    assert_eq!(run.tool(), StrokeTool::Paint);
    assert_eq!(run.toolbox.projection().recent[..2], [blue, red]);
}

#[test]
fn undo_and_redo_fire_once_per_press_hovering_or_touching() {
    let map = ButtonMap {
        primary: ButtonAction::Undo,
        secondary: ButtonAction::Redo,
    };
    let mut run = Run::new(map);
    let undo = Some(Intent::History(HistoryIntent::Undo));
    let redo = Some(Intent::History(HistoryIntent::Redo));
    assert_eq!(
        run.step(&pen(ContactPhase::Hover, FIRST)),
        [undo.clone(), None]
    );
    assert_eq!(
        run.step(&pen(ContactPhase::Hover, FIRST)),
        [None],
        "held: once"
    );
    assert_eq!(run.step(&pen(ContactPhase::Hover, NONE)), [None]);
    let both = PenButtons {
        primary: true,
        secondary: true,
    };
    let pressed = run.step(&pen(ContactPhase::Hover, both));
    assert_eq!(pressed, [undo.clone(), redo, None]);
    run.step(&pen(ContactPhase::Hover, NONE));
    // Pressed while drawing: after the lift, never during the stroke (D20).
    run.step(&pen(ContactPhase::Down, NONE));
    assert_eq!(run.step(&pen(ContactPhase::Move, FIRST)), [None]);
    assert_eq!(run.step(&pen(ContactPhase::Up, FIRST)), [None, undo]);
    assert_eq!(
        run.step(&pen(ContactPhase::Hover, FIRST)),
        [None],
        "not again"
    );
}

#[test]
fn switch_tool_swaps_brush_and_eraser_once_per_press() {
    let map = ButtonMap {
        primary: ButtonAction::SwitchTool,
        secondary: ButtonAction::Nothing,
    };
    let mut run = Run::new(map);
    run.step(&pen(ContactPhase::Hover, FIRST));
    assert_eq!(run.tool(), StrokeTool::Erase);
    run.step(&pen(ContactPhase::Hover, FIRST));
    assert_eq!(run.tool(), StrokeTool::Erase, "held: once");
    run.step(&pen(ContactPhase::Hover, NONE));
    run.step(&pen(ContactPhase::Hover, FIRST));
    assert_eq!(run.tool(), StrokeTool::Paint);
    assert_eq!(
        run.toolbox.state().tool,
        Tool::Brush,
        "a switch is a choice: stored"
    );
    run.toolbox.dispatch_tool(&ToolIntent::Use(Tool::Smudge));
    run.step(&pen(ContactPhase::Hover, NONE));
    run.step(&pen(ContactPhase::Hover, FIRST));
    assert_eq!(run.tool(), StrokeTool::Erase, "from the smudge too");
    // Nothing does nothing.
    assert_eq!(run.step(&pen(ContactPhase::Hover, SECOND)), [None]);
}

#[test]
fn the_eraser_end_erases_while_in_range_and_the_tip_does_not() {
    let mut run = Run::new(ButtonMap::default());
    let eraser = |phase| batch(Device::Eraser, phase, NONE);
    assert_eq!(
        run.step(&eraser(ContactPhase::Hover)),
        [hold(Some(HeldTool::Eraser)), None]
    );
    assert_eq!(run.step(&eraser(ContactPhase::Down)), [None]);
    assert_eq!(run.tool(), StrokeTool::Erase);
    assert_eq!(run.step(&eraser(ContactPhase::Up)), [None, hold(None)]);
    run.step(&eraser(ContactPhase::Hover));
    assert_eq!(run.step(&eraser(ContactPhase::Leave)), [hold(None), None]);
    // Turned round: the tip paints even with an eraser button mapped to Nothing.
    let mut run = Run::new(ButtonMap {
        primary: ButtonAction::Nothing,
        secondary: ButtonAction::Nothing,
    });
    run.step(&eraser(ContactPhase::Down));
    assert_eq!(
        run.tool(),
        StrokeTool::Erase,
        "erases whatever the buttons do"
    );
    run.step(&eraser(ContactPhase::Up));
    assert_eq!(run.step(&pen(ContactPhase::Hover, FIRST)), [None]);
    assert_eq!(run.tool(), StrokeTool::Paint);
}

#[test]
fn touches_and_button_only_batches_are_handled_alike() {
    let mut run = Run::new(ButtonMap::default());
    run.step(&pen(ContactPhase::Hover, FIRST));
    // A finger's event has no pen buttons: it neither presses nor releases them.
    let finger = InputBatch {
        pen: None,
        ..batch(Device::Finger, ContactPhase::Down, NONE)
    };
    assert_eq!(run.step(&finger), [None]);
    assert_eq!(run.tool(), StrokeTool::Erase);
    // Android's button release with no movement: a pen batch of no contacts.
    let release = InputBatch {
        contacts: Vec::new(),
        pen: Some(NONE),
    };
    assert_eq!(run.step(&release), [hold(None), None]);
}

#[test]
fn the_map_is_the_authors_and_is_stored() {
    let library = BrushLibrary::built_in().expect("built-in brushes");
    let mut toolbox = Toolbox::new(library);
    assert_eq!(toolbox.projection().buttons, ButtonMap::default());
    assert_eq!(
        ButtonMap::default().action(PenButton::Primary),
        ButtonAction::Eraser
    );
    assert_eq!(
        ButtonMap::default().action(PenButton::Secondary),
        ButtonAction::Eyedropper
    );
    assert_eq!(toolbox.save_request(), None);
    let undo = ToolIntent::SetButton {
        button: PenButton::Secondary,
        action: ButtonAction::Undo,
    };
    toolbox.dispatch_tool(&undo);
    let saved = toolbox.save_request().expect("changed");
    assert_eq!(saved.buttons.secondary, ButtonAction::Undo);
    toolbox.dispatch_tool(&undo);
    assert_eq!(
        toolbox.save_request(),
        None,
        "the same again changes nothing"
    );
    let mut other = Toolbox::new(BrushLibrary::built_in().expect("built-in brushes"));
    assert!(other.restore(&saved));
    assert_eq!(other.projection().buttons.secondary, ButtonAction::Undo);
}

#[test]
fn swatches_are_added_used_removed_and_stored() {
    let library = BrushLibrary::built_in().expect("built-in brushes");
    let mut toolbox = Toolbox::new(library);
    let red = Color::rgb(1.0, 0.0, 0.0).expect("red");
    toolbox.dispatch_tool(&ToolIntent::Color {
        color: red,
        settled: true,
    });
    toolbox.dispatch_tool(&ToolIntent::AddSwatch);
    toolbox.dispatch_tool(&ToolIntent::AddSwatch);
    assert_eq!(*toolbox.projection().swatches, [red]);
    let saved = toolbox.save_request().expect("changed");
    assert_eq!(saved.swatches, [[1.0, 0.0, 0.0]]);
    toolbox.dispatch_tool(&ToolIntent::RemoveSwatch(red));
    assert!(toolbox.projection().swatches.is_empty());
    assert!(toolbox.save_request().is_some(), "removing is a change");
    let mut other = Toolbox::new(BrushLibrary::built_in().expect("built-in brushes"));
    assert!(other.restore(&saved));
    assert_eq!(*other.projection().swatches, [red]);
}
