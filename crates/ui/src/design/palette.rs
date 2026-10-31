//! The paper and the ink (D40): every colour, line weight and shadow the chrome uses.
//! Near-neutral warm paper, so the chrome never tints how colours on the canvas are seen; one
//! ink for text, icons and lines, one deeper ink for what is active; panels drawn as ink
//! outlines, never as filled blocks.

use egui::Color32;

/// Islands, panels, popovers, dialogs and pills: the paper the chrome is drawn on.
pub const PANEL: Color32 = Color32::from_rgb(250, 248, 244);
/// The ink outline of every island and panel.
pub const PANEL_RULE: Color32 = Color32::from_rgb(70, 67, 62);
/// Weight of that outline, in points.
pub const PANEL_RULE_WIDTH: f32 = 1.25;
/// The desk the canvas lies on: a little greyer than the paper, so the white canvas stands out.
pub const DESK: Color32 = Color32::from_rgb(233, 230, 224);
/// Text, icons and lines.
pub const INK: Color32 = Color32::from_rgb(38, 37, 35);
/// What is active now: the chosen tool, row or open panel.
pub const INK_ACTIVE: Color32 = Color32::from_rgb(22, 28, 44);
/// Quieter text: details, notes, section names.
pub const INK_SECONDARY: Color32 = Color32::from_rgb(104, 100, 94);
/// What cannot be used now: still visible, clearly not tappable.
pub const INK_DISABLED: Color32 = Color32::from_rgb(186, 182, 174);
/// The outline of a text field.
pub const BUTTON_RULE: Color32 = Color32::from_rgb(70, 67, 62);
/// Faint ink: separators across panels and the divider between groups in a bar.
pub const DIVIDER: Color32 = Color32::from_rgb(186, 182, 174);
/// Under the hand while it presses a control.
pub const PRESSED: Color32 = Color32::from_black_alpha(18);
/// The light cells of the checker under a layer's picture, where the layer is empty (the others
/// are white).
pub const CHECKER: Color32 = Color32::from_rgb(228, 226, 222);
/// The canvas's and the gallery pictures' shadow on the desk: strength at the sheet's edge.
pub const SHEET_SHADOW: u8 = 34;
/// The marking red a check lays over a practice copy: the original or the answer, and what of the
/// copy is off. Never chrome.
pub const CORRECTION: Color32 = Color32::from_rgb(200, 58, 44);
/// What a constructed exercise gives in the frame drawn in (S97: a box's horizon, corner and
/// edges, an ellipse's frame): a solid slate grey, as if printed on the sheet, cool and even
/// beside Draw's black ink and Sketch's warm grainy pencil. Never chrome.
pub const PRINTED: Color32 = Color32::from_rgb(112, 122, 134);
/// The practice book's seals (S89): its only colour besides the user's own colours. Never chrome.
pub const VERMILION: Color32 = Color32::from_rgb(184, 58, 38);
/// The pencil the practice book draws what lies ahead with.
pub const PENCIL: Color32 = Color32::from_rgb(189, 186, 177);
