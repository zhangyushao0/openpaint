//! What the toolbox keeps for the pen and the colour panel (S41, S43): the side buttons' map
//! (three buttons, Undo for the third at first) and the palette's swatches, saved and restored
//! as settings, never undo steps.

use openpaint_app::{BrushLibrary, ButtonAction, ButtonMap, PenButton, ToolIntent, Toolbox};
use openpaint_core::Color;

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
    assert_eq!(
        ButtonMap::default().action(PenButton::Tertiary),
        ButtonAction::Undo
    );
    assert_eq!(toolbox.save_request(), None);
    let undo = ToolIntent::SetButton {
        button: PenButton::Secondary,
        action: ButtonAction::Undo,
    };
    toolbox.dispatch_tool(&undo);
    toolbox.dispatch_tool(&ToolIntent::SetButton {
        button: PenButton::Tertiary,
        action: ButtonAction::Redo,
    });
    let saved = toolbox.save_request().expect("changed");
    assert_eq!(saved.buttons.secondary, ButtonAction::Undo);
    assert_eq!(saved.buttons.tertiary, ButtonAction::Redo);
    toolbox.dispatch_tool(&undo);
    assert_eq!(
        toolbox.save_request(),
        None,
        "the same again changes nothing"
    );
    let mut other = Toolbox::new(BrushLibrary::built_in().expect("built-in brushes"));
    assert!(other.restore(&saved));
    let restored = ButtonMap {
        primary: ButtonAction::Eraser,
        secondary: ButtonAction::Undo,
        tertiary: ButtonAction::Redo,
    };
    assert_eq!(other.projection().buttons, restored);
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
