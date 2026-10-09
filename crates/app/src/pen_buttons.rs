//! The pen's buttons (S41, S43, D70): what each of three side buttons does is the author's
//! choice from a short list, stored with the toolbox; the eraser end always erases. Android
//! reports the side buttons as the button state of every pen event, hovering or touching;
//! `ButtonTracker` turns their changes into intents placed around the contacts of the same
//! batch:
//!
//! - Eraser and Eyedropper act while held: the pen's contacts made while the button is down use
//!   that tool, and the tool chosen before comes back once it is up. A press or a release while
//!   the pen touches the glass takes effect when it lifts, so a stroke or a pick is never cut in
//!   two.
//! - Undo, Redo and Switch tools act once per press, hovering or touching; a press while
//!   touching acts when the pen lifts (no undo while a stroke is drawn, D20).
//! - The eraser end (`Device::Eraser`) holds the eraser from the moment it comes into range
//!   until it leaves or lifts.

use crate::history::HistoryIntent;
use crate::input::{ContactPhase, Device, InputBatch, PenButtons};
use crate::intent::Intent;
use crate::tool_intent::ToolIntent;

/// A side button of the pen, as Android names their bits: on the author's Wacom pen from the
/// tip outwards primary, secondary and tertiary (measured 2026-10-09).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PenButton {
    Primary,
    Secondary,
    Tertiary,
}

impl PenButton {
    pub const ALL: [Self; 3] = [Self::Primary, Self::Secondary, Self::Tertiary];

    /// The button's name on the pen buttons' page.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Primary => "Side button 1",
            Self::Secondary => "Side button 2",
            Self::Tertiary => "Side button 3",
        }
    }

    /// Whether `buttons` holds this one down.
    const fn down(self, buttons: PenButtons) -> bool {
        match self {
            Self::Primary => buttons.primary,
            Self::Secondary => buttons.secondary,
            Self::Tertiary => buttons.tertiary,
        }
    }
}

/// What a side button does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonAction {
    Nothing,
    /// The eraser while held.
    Eraser,
    /// The eyedropper while held.
    Eyedropper,
    /// Once per press.
    Undo,
    Redo,
    /// Brush to eraser, or eraser to brush, once per press.
    SwitchTool,
}

impl ButtonAction {
    pub const ALL: [Self; 6] = [
        Self::Nothing,
        Self::Eraser,
        Self::Eyedropper,
        Self::Undo,
        Self::Redo,
        Self::SwitchTool,
    ];

    /// The word the pen buttons' page shows.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Nothing => "Nothing",
            Self::Eraser => "Eraser",
            Self::Eyedropper => "Eyedropper",
            Self::Undo => "Undo",
            Self::Redo => "Redo",
            Self::SwitchTool => "Switch tool",
        }
    }

    /// The tool this action holds while its button is down, if it is one that holds.
    const fn held(self) -> Option<HeldTool> {
        match self {
            Self::Eraser => Some(HeldTool::Eraser),
            Self::Eyedropper => Some(HeldTool::Eyedropper),
            Self::Nothing | Self::Undo | Self::Redo | Self::SwitchTool => None,
        }
    }

    /// The intent this action fires once per press, if it is one that fires.
    fn fired(self) -> Option<Intent> {
        match self {
            Self::Undo => Some(Intent::History(HistoryIntent::Undo)),
            Self::Redo => Some(Intent::History(HistoryIntent::Redo)),
            Self::SwitchTool => Some(Intent::Tool(ToolIntent::Switch)),
            Self::Nothing | Self::Eraser | Self::Eyedropper => None,
        }
    }
}

/// A tool a pen button holds for as long as it is down.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeldTool {
    Eraser,
    Eyedropper,
}

/// What each side button does. At first the first erases and the second picks colours while
/// held, the buttons most painting apps give a Wacom pen (author, 2026-10-05); the third
/// undoes (S43, an agent's choice).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ButtonMap {
    pub primary: ButtonAction,
    pub secondary: ButtonAction,
    pub tertiary: ButtonAction,
}

impl Default for ButtonMap {
    fn default() -> Self {
        Self {
            primary: ButtonAction::Eraser,
            secondary: ButtonAction::Eyedropper,
            tertiary: ButtonAction::Undo,
        }
    }
}

impl ButtonMap {
    #[must_use]
    pub const fn action(self, button: PenButton) -> ButtonAction {
        match button {
            PenButton::Primary => self.primary,
            PenButton::Secondary => self.secondary,
            PenButton::Tertiary => self.tertiary,
        }
    }

    /// The map with `button` doing `action`.
    #[must_use]
    pub const fn with(mut self, button: PenButton, action: ButtonAction) -> Self {
        match button {
            PenButton::Primary => self.primary = action,
            PenButton::Secondary => self.secondary = action,
            PenButton::Tertiary => self.tertiary = action,
        }
        self
    }
}

/// Follows the pen's buttons and the eraser end across batches.
#[derive(Debug, Default)]
pub struct ButtonTracker {
    /// The side buttons as the pen's last batch reported them.
    buttons: PenButtons,
    /// The pen (tip or eraser end) is on the glass.
    touching: bool,
    /// The eraser end is in range: hovering or touching.
    eraser_end: bool,
    /// The tool the last `Hold` held.
    holding: Option<HeldTool>,
    /// Presses made while touching, fired when the pen lifts.
    waiting: Vec<Intent>,
}

impl ButtonTracker {
    /// What `batch` asks for before its contacts go anywhere: once-per-press actions of buttons
    /// pressed with the pen off the glass (later ones wait for the lift), and, with the pen off
    /// the glass, the tool the buttons and the eraser end hold now, so a contact touching down
    /// in this batch already uses it. Nothing for batches that are not the pen's.
    pub fn before(&mut self, batch: &InputBatch, map: ButtonMap) -> Vec<Intent> {
        let Some(buttons) = batch.pen else {
            return Vec::new();
        };
        let mut intents = Vec::new();
        let pressed = PenButton::ALL
            .into_iter()
            .filter(|button| button.down(buttons) && !button.down(self.buttons));
        for button in pressed {
            if let Some(intent) = map.action(button).fired() {
                if self.touching {
                    self.waiting.push(intent);
                } else {
                    intents.push(intent);
                }
            }
        }
        self.buttons = buttons;
        for contact in &batch.contacts {
            match (contact.device, contact.phase) {
                (Device::Eraser, ContactPhase::Hover | ContactPhase::Down | ContactPhase::Move) => {
                    self.eraser_end = true;
                }
                (Device::Eraser, _) | (Device::Pen, _) => self.eraser_end = false,
                _ => {}
            }
        }
        if !self.touching {
            self.hold(map, &mut intents);
        }
        intents
    }

    /// What `batch` asks for after its contacts: when the pen lifted in it, the presses that
    /// waited for the lift, then the tool the buttons hold now.
    pub fn after(&mut self, batch: &InputBatch, map: ButtonMap) -> Vec<Intent> {
        if batch.pen.is_none() {
            return Vec::new();
        }
        let touched = self.touching;
        for contact in &batch.contacts {
            if matches!(contact.device, Device::Pen | Device::Eraser) {
                self.touching = matches!(contact.phase, ContactPhase::Down | ContactPhase::Move);
            }
        }
        if !touched || self.touching {
            return Vec::new();
        }
        let mut intents = std::mem::take(&mut self.waiting);
        self.hold(map, &mut intents);
        intents
    }

    /// Holds the tool the eraser end or the buttons ask for (the first side button before the
    /// second, the second before the third), if it is not held already.
    fn hold(&mut self, map: ButtonMap, intents: &mut Vec<Intent>) {
        let wanted = if self.eraser_end {
            Some(HeldTool::Eraser)
        } else {
            PenButton::ALL
                .into_iter()
                .filter(|button| button.down(self.buttons))
                .find_map(|button| map.action(button).held())
        };
        if wanted != self.holding {
            self.holding = wanted;
            intents.push(Intent::Tool(ToolIntent::Hold(wanted)));
        }
    }
}
