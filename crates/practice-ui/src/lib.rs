//! Practice's screens (G9, D112): a practice sheet's marks, bar, tools, card and How on the canvas,
//! and in the gallery the Today card, the practice options and the practice book, with their words
//! in English and Chinese. The ui offers hooks a mode fills (`openpaint_ui::Hooks`); this crate
//! fills them (`PracticeScreens::hooked`), and the runtime composes the two: it takes practice's
//! taps after the frame's routing and applies them after the frame's painting intents.
//!
//! No direct GPU API, no file system (`cargo repo-check`).
#![forbid(unsafe_code)]

// The ui's `texts!` makes practice's word tables as it makes the ui's.
#[macro_use]
extern crate openpaint_ui;

mod components;
mod gallery;
mod palette;
mod screens;
mod sheet;
mod text;

pub use crate::screens::{Hooked, PracticeScreens, Shown};
pub use crate::text::NAMES;
