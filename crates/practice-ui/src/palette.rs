//! Practice's own colours beside the ui's paper and ink (D40): what a check, a constructed
//! exercise and the practice book draw with. Never chrome.

use egui::Color32;

/// The marking red a check lays over a practice copy: the original or the answer, and what of the
/// copy is off.
pub(crate) const CORRECTION: Color32 = Color32::from_rgb(200, 58, 44);
/// What a constructed exercise gives in the frame drawn in (S97: a box's horizon, corner and
/// edges, an ellipse's frame): a solid slate grey, as if printed on the sheet, cool and even
/// beside Draw's black ink and Sketch's warm grainy pencil.
pub(crate) const PRINTED: Color32 = Color32::from_rgb(112, 122, 134);
/// The practice book's seals (S89): its only colour besides the user's own colours.
pub(crate) const VERMILION: Color32 = Color32::from_rgb(184, 58, 38);
/// Words in the seal's red (the bar's Time, S124): a shade deeper, to read as text.
pub(crate) const VERMILION_INK: Color32 = Color32::from_rgb(162, 48, 31);
/// The pencil the practice book draws what lies ahead with.
pub(crate) const PENCIL: Color32 = Color32::from_rgb(189, 186, 177);
/// A photo's frame covered while a drawing from life is made (S151): paper a shade darker than
/// the sheet, and the closed eye on it in a darker pencil.
pub(crate) const COVER: Color32 = Color32::from_rgb(244, 241, 235);
pub(crate) const COVER_EYE: Color32 = Color32::from_rgb(169, 164, 155);
