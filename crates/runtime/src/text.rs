//! Text from the host's keyboard (the Android soft keyboard or a hardware one) for the text
//! field that has the focus, as egui's events. The UI asks for the keyboard through egui's IME
//! output; the host shows and hides it (`Runtime::keyboard_wanted`).

/// What the keyboard delivers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextInput {
    /// Text being composed (an input method's suggestion): shown in place, not final yet; an
    /// empty one clears the composition.
    Compose(String),
    /// Final text, typed or chosen; replaces a composition.
    Commit(String),
    /// Removes `before` characters before the cursor and `after` after it.
    DeleteSurrounding {
        before: usize,
        after: usize,
    },
    Key(EditKey),
}

/// The editing keys a text field answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditKey {
    Backspace,
    Delete,
    /// Ends the edit (a one-line field).
    Enter,
    Left,
    Right,
}

/// The egui events of `input`.
pub(crate) fn text_events(input: TextInput) -> Vec<egui::Event> {
    let ime = match input {
        TextInput::Compose(text) => egui::ImeEvent::Preedit {
            active_range_chars: None,
            text,
        },
        TextInput::Commit(text) => egui::ImeEvent::Commit(text),
        TextInput::DeleteSurrounding { before, after } => egui::ImeEvent::DeleteSurrounding {
            before_chars: before,
            after_chars: after,
        },
        TextInput::Key(key) => {
            let key = match key {
                EditKey::Backspace => egui::Key::Backspace,
                EditKey::Delete => egui::Key::Delete,
                EditKey::Enter => egui::Key::Enter,
                EditKey::Left => egui::Key::ArrowLeft,
                EditKey::Right => egui::Key::ArrowRight,
            };
            let event = |pressed| egui::Event::Key {
                key,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            };
            return vec![event(true), event(false)];
        }
    };
    vec![egui::Event::Ime(ime)]
}
