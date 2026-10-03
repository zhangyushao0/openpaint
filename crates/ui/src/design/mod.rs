//! The theme, defined once (ARCHITECTURE section 10, D32, D40): paper-white line art. Light,
//! near-neutral warm paper islands drawn as ink outlines, floating over the desk the white
//! canvas lies on; ink-line icons in one weight; one deeper ink ring for the active state; the
//! system's plain sans at sizes that read at arm's length; an 8-point spacing scale; 48-point
//! hit targets. Components read these values; screens never style widgets inline.

mod fonts;
mod icon_tools;
mod icons;
mod paint;
mod palette;

use egui::style::{ScrollFadeStyle, ScrollStyle};
use egui::{Color32, FontId, Stroke, TextStyle, Vec2};

pub use fonts::{FontFile, TextFonts};
pub use icons::{ICON_SIZE, Icon, paint as paint_icon};
pub use paint::{
    ink_line, paint_icon_mark, paint_pressed, paint_row_mark, paint_sheet_shadow, panel_frame,
};
pub use palette::{
    BUTTON_RULE, DESK, INK, INK_ACTIVE, INK_DISABLED, INK_SECONDARY, PANEL, PANEL_RULE,
    PANEL_RULE_WIDTH, PRESSED,
};

/// The spacing unit in points; every gap and margin is a multiple of it.
pub const SPACE: f32 = 8.0;
/// Smallest side of anything the hand taps, in points (48 px at density 1, more on a tablet).
pub const TOUCH_TARGET: f32 = 48.0;
/// Corner radius of islands, panels, buttons and marks, in points.
pub const CORNER: f32 = 1.25 * SPACE;
/// Width of a panel over the canvas, in points.
pub const PANEL_WIDTH: f32 = 320.0;
/// Width of a dialog, in points.
pub const DIALOG_WIDTH: f32 = 400.0;
/// Over everything behind a dialog: a warm veil, so the dialog is the one thing to answer.
pub const BACKDROP: Color32 = Color32::from_rgba_premultiplied(30, 27, 22, 90);
/// A gallery card's picture, in points: 448 x 336 pixels at density 1.75, the largest
/// thumbnail the runtime makes.
pub const CARD_PICTURE: Vec2 = Vec2::new(256.0, 192.0);
/// Between gallery cards and around the grid, in points.
pub const CARD_GAP: f32 = 4.0 * SPACE;
/// Behind the canvas where the canvas does not reach (the desk), as RGBA bytes for the renderer.
pub const SURROUND: [u8; 4] = [DESK.r(), DESK.g(), DESK.b(), 255];
/// A brush's stroke preview in the brush picker, in points.
pub const BRUSH_PREVIEW: Vec2 = Vec2::new(160.0, 48.0);
/// Length of a side bar slider, in points.
pub const SIDE_SLIDER: f32 = 160.0;
/// Side of the colour wheel, in points: the hue ring around the saturation and value square.
pub const COLOR_WHEEL: f32 = 272.0;
/// Width of the colour wheel's hue ring, in points: wide enough to hold a fingertip.
pub const HUE_RING: f32 = 32.0;
/// Side of a recent colour's swatch, in points: eight fill a panel's width.
pub const RECENT_SWATCH: f32 = 38.0;
/// Width of the reference window, in points.
pub const REFERENCE_WIDTH: f32 = 360.0;
/// Least and most height of the reference window's picture, in points; between them it
/// follows the picture's shape.
pub const REFERENCE_HEIGHT: [f32; 2] = [180.0, 420.0];
/// Weight of hairlines: separators, outlines of fields and buttons, slider tracks, in points.
pub const HAIRLINE: f32 = 1.0;

/// The type scale in points: one size per role, large enough to read on a tablet held at arm's
/// length.
const TYPE_SCALE: [(TextStyle, f32); 5] = [
    (TextStyle::Small, 14.0),
    (TextStyle::Body, 18.0),
    (TextStyle::Button, 18.0),
    (TextStyle::Heading, 22.0),
    (TextStyle::Monospace, 15.0),
];

/// Installs the theme into `ctx`. Call once, before the first frame.
pub fn install(ctx: &egui::Context) {
    ctx.set_theme(egui::Theme::Light);
    ctx.all_styles_mut(|style| {
        let visuals = &mut style.visuals;
        visuals.panel_fill = DESK;
        visuals.window_fill = PANEL;
        visuals.window_stroke = Stroke::new(PANEL_RULE_WIDTH, PANEL_RULE);
        visuals.window_shadow = egui::epaint::Shadow::NONE;
        visuals.popup_shadow = egui::epaint::Shadow::NONE;
        visuals.extreme_bg_color = Color32::WHITE;
        visuals.override_text_color = Some(INK);
        visuals.text_cursor.stroke = Stroke::new(2.0, INK_ACTIVE);
        visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(22, 28, 44, 48);
        visuals.selection.stroke = Stroke::new(1.5, INK_ACTIVE);
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(HAIRLINE, INK_DISABLED);
        for widget in [&mut visuals.widgets.inactive, &mut visuals.widgets.hovered] {
            widget.bg_stroke = Stroke::new(HAIRLINE, BUTTON_RULE);
        }
        visuals.widgets.active.bg_stroke = Stroke::new(1.5, INK_ACTIVE);
        // A hover label shows at once and stays while the pen moves over the control.
        style.interaction.tooltip_delay = 0.0;
        style.interaction.show_tooltips_only_when_still = false;
        style.spacing.item_spacing = Vec2::splat(SPACE);
        style.spacing.button_padding = Vec2::new(2.0 * SPACE, SPACE);
        style.spacing.interact_size = Vec2::splat(TOUCH_TARGET);
        style.spacing.scroll = scrolling();
        for (text, size) in TYPE_SCALE {
            let family = match text {
                TextStyle::Monospace => FontId::monospace(size),
                _ => FontId::proportional(size),
            };
            style.text_styles.insert(text, family);
        }
    });
}

/// No scroll bar at rest: a cut-off last row and a soft fade at the edge say there is more; a
/// thin line shows only while the hand scrolls (D32).
fn scrolling() -> ScrollStyle {
    ScrollStyle {
        bar_width: 3.0,
        floating_width: 3.0,
        dormant_background_opacity: 0.0,
        dormant_handle_opacity: 0.0,
        active_background_opacity: 0.0,
        active_handle_opacity: 0.5,
        interact_background_opacity: 0.0,
        interact_handle_opacity: 0.7,
        fade: ScrollFadeStyle {
            strength: 0.8,
            size: 1.5 * SPACE,
        },
        ..ScrollStyle::floating()
    }
}

/// Puts the host's text faces (the system's sans, its Chinese face) in front of the built-in
/// one. The host finds and reads them; the UI only owns the families.
pub fn install_text_fonts(ctx: &egui::Context, fonts: TextFonts) {
    fonts::install(ctx, fonts);
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
        assert_eq!(style.visuals.window_fill, PANEL);
        let body = &style.text_styles[&TextStyle::Body];
        assert_eq!(body.size, 18.0);
    }
}
