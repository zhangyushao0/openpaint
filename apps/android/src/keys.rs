//! Keyboard input from the Kotlin host as the runtime's `TextInput`: the input connection's
//! composing and committed text, and the editing keys. Plain Rust, tested on the dev machine;
//! the codes must match `TextInput.kt`.

use openpaint_runtime::{EditKey, TextInput};

/// `kind` of `Native.text`: text being composed.
pub const COMPOSE: i32 = 0;
/// `kind` of `Native.text`: final text.
pub const COMMIT: i32 = 1;

/// The text call `kind` carrying `text`; `None` for an unknown kind.
#[must_use]
pub fn text_input(kind: i32, text: String) -> Option<TextInput> {
    match kind {
        COMPOSE => Some(TextInput::Compose(text)),
        COMMIT => Some(TextInput::Commit(text)),
        _ => None,
    }
}

/// The editing key with code `code` (`Native.key`): 0 backspace, 1 delete, 2 enter, 3 left,
/// 4 right.
#[must_use]
pub fn edit_key(code: i32) -> Option<TextInput> {
    let key = match code {
        0 => EditKey::Backspace,
        1 => EditKey::Delete,
        2 => EditKey::Enter,
        3 => EditKey::Left,
        4 => EditKey::Right,
        _ => return None,
    };
    Some(TextInput::Key(key))
}

/// Deleting around the cursor, counts in characters; negative counts are none.
#[must_use]
pub fn delete_surrounding(before: i32, after: i32) -> TextInput {
    let count = |n: i32| usize::try_from(n).unwrap_or(0);
    TextInput::DeleteSurrounding {
        before: count(before),
        after: count(after),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_map_to_text_input() {
        assert_eq!(
            text_input(COMMIT, "a".into()),
            Some(TextInput::Commit("a".into()))
        );
        assert_eq!(
            text_input(COMPOSE, "b".into()),
            Some(TextInput::Compose("b".into()))
        );
        assert_eq!(text_input(7, "c".into()), None);
        assert_eq!(edit_key(2), Some(TextInput::Key(EditKey::Enter)));
        assert_eq!(edit_key(9), None);
        assert_eq!(
            delete_surrounding(-1, 2),
            TextInput::DeleteSurrounding {
                before: 0,
                after: 2
            }
        );
    }
}
