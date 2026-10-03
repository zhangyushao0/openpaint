//! The theme, defined once (ARCHITECTURE section 10): neutral dark greys for the chrome, so it
//! never tints how colours on the canvas are seen; an 8-point spacing scale; one type scale;
//! 48-point hit targets. Components read these values; screens never style widgets inline.

mod icons;

use egui::{Color32, FontId, TextStyle, Vec2};

pub use icons::{ICON_SIZE, Icon, paint as paint_icon};

/// The spacing unit in points; every gap and margin is a multiple of it.
pub const SPACE: f32 = 8.0;
/// Smallest side of anything the hand taps, in points (48 px at density 1, more on a tablet).
pub const TOUCH_TARGET: f32 = 48.0;
/// Corner radius of pressed and selected highlights, in points.
pub const CORNER: f32 = SPACE;

/// Bars and panels.
pub const CHROME: Color32 = Color32::from_gray(38);
/// Text and icons on the chrome.
pub const ON_CHROME: Color32 = Color32::from_gray(222);
/// Secondary text on the chrome: readable on every face, quieter than the main text.
pub const ON_CHROME_SECONDARY: Color32 = Color32::from_gray(170);
/// Icons of actions that are unavailable now: still visible, clearly not tappable.
pub const ON_CHROME_DISABLED: Color32 = Color32::from_gray(92);
/// Behind a control while the hand presses it.
pub const PRESSED: Color32 = Color32::from_gray(64);
/// The face of a text button at rest: lighter than the chrome, darker than pressed.
pub const RAISED: Color32 = Color32::from_gray(52);
/// Behind the chosen item of a list and a button whose panel is open: lighter than pressed.
pub const SELECTED: Color32 = Color32::from_gray(80);
/// The edge of a panel over the canvas, so it stands apart from dark artwork too.
pub const OUTLINE: Color32 = Color32::from_gray(72);
/// Width of a panel over the canvas, in points.
pub const PANEL_WIDTH: f32 = 320.0;
/// Width of a dialog, in points.
pub const DIALOG_WIDTH: f32 = 400.0;
/// Over everything behind a dialog: it dims, so the dialog is the one thing to answer.
pub const BACKDROP: Color32 = Color32::from_black_alpha(140);
/// A gallery card's picture, in points: 448 x 336 pixels at density 1.75, the largest
/// thumbnail the runtime makes.
pub const CARD_PICTURE: Vec2 = Vec2::new(256.0, 192.0);
/// Between gallery cards and around the grid, in points.
pub const CARD_GAP: f32 = 4.0 * SPACE;
/// Behind the canvas where the canvas does not reach, as RGBA bytes for the renderer.
pub const SURROUND: [u8; 4] = [24, 24, 24, 255];
/// A brush's stroke preview in the brush picker, in points.
pub const BRUSH_PREVIEW: Vec2 = Vec2::new(160.0, 48.0);
/// Length of a side bar slider, in points.
pub const SIDE_SLIDER: f32 = 176.0;

/// The type scale in points: one size per role.
const TYPE_SCALE: [(TextStyle, f32); 5] = [
    (TextStyle::Small, 12.0),
    (TextStyle::Body, 16.0),
    (TextStyle::Button, 16.0),
    (TextStyle::Heading, 20.0),
    (TextStyle::Monospace, 14.0),
];

/// Installs the theme into `ctx`. Call once, before the first frame.
pub fn install(ctx: &egui::Context) {
    ctx.set_theme(egui::Theme::Dark);
    ctx.all_styles_mut(|style| {
        style.visuals.panel_fill = CHROME;
        style.visuals.window_fill = CHROME;
        style.visuals.override_text_color = Some(ON_CHROME);
        style.spacing.item_spacing = Vec2::splat(SPACE);
        style.spacing.button_padding = Vec2::new(2.0 * SPACE, SPACE);
        style.spacing.interact_size = Vec2::splat(TOUCH_TARGET);
        for (text, size) in TYPE_SCALE {
            let family = match text {
                TextStyle::Monospace => FontId::monospace(size),
                _ => FontId::proportional(size),
            };
            style.text_styles.insert(text, family);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_sets_the_touch_target_and_the_type_scale() {
        let ctx = egui::Context::default();
        install(&ctx);
        let style = ctx.global_style();
        assert_eq!(style.spacing.interact_size, Vec2::splat(TOUCH_TARGET));
        assert_eq!(style.visuals.panel_fill, CHROME);
        let body = &style.text_styles[&TextStyle::Body];
        assert_eq!(body.size, 16.0);
    }
}
