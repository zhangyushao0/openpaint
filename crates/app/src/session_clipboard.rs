//! The session's side of copy and paste (`clipboard.rs`): each command acts on the version
//! shown now (a transform or an adjustment under way ends first), inside the selection when
//! there is one, which stays. Cut is one history step; Paste, Duplicate and Cut & Paste frame
//! the pasted layer with the Transform tool, and the commit records it as one step.

use openpaint_core::{Artwork, Copied};

use super::Session;
use crate::clipboard::{Clipboard, ClipboardAction};
use crate::error::AppError;
use crate::intent::Intent;
use crate::projection::EditorAction;
use crate::selection::SelectionIntent;

impl Session {
    /// Applies the copy menu's `action`, copying into and pasting from `clipboard` (the app's).
    ///
    /// # Errors
    /// `Unavailable` when the menu does not offer it now; `Core` or `TransformRefused` when the
    /// artwork refuses it.
    pub fn clipboard(
        &mut self,
        action: ClipboardAction,
        clipboard: &mut Clipboard,
    ) -> Result<(), AppError> {
        let editor_action = EditorAction::Clipboard(action);
        let availability = self
            .projection()
            .with_clipboard(clipboard.canvas())
            .availability(editor_action);
        if !availability.is_available() {
            return Err(AppError::Unavailable {
                action: editor_action,
                availability,
            });
        }
        self.end_drafts(&Intent::Clipboard(action));
        self.fills.interrupt();
        self.layers.interrupt();
        let artwork = self.history.current().clone();
        let clip = self.selection.mask().cloned();
        let copy = |all| artwork.copied(all, clip.as_ref());
        match action {
            ClipboardAction::Copy => clipboard.hold(copy(false)?),
            ClipboardAction::CopyAll => clipboard.hold(copy(true)?),
            ClipboardAction::Cut => {
                let cut = artwork.cut(clip.as_ref())?;
                self.history.record(cut.artwork);
            }
            ClipboardAction::Duplicate => {
                let copied = copy(false)?;
                clipboard.hold(copied.clone());
                self.paste((&artwork, &artwork), &copied)?;
            }
            ClipboardAction::CutPaste => {
                let copied = copy(false)?;
                let cut = artwork.cut(clip.as_ref())?.artwork;
                clipboard.hold(copied.clone());
                self.paste((&artwork, &cut), &copied)?;
            }
            ClipboardAction::Paste => {
                if let Some(copied) = clipboard.held().cloned() {
                    self.paste((&artwork, &artwork), &copied)?;
                }
            }
        }
        Ok(())
    }

    /// `copied` on a new layer above the active one of `onto`, framed by the Transform tool;
    /// undo at its first step goes back to `from`, the version history holds now.
    fn paste(
        &mut self,
        (from, onto): (&Artwork, &Artwork),
        copied: &Copied,
    ) -> Result<(), AppError> {
        let canvas = from.canvas();
        self.selection.handle(canvas, SelectionIntent::Tool(false));
        self.transforms.paste(from, onto, copied)
    }
}
