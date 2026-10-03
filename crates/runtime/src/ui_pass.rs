//! One egui pass over the screen: the shell builds the frame from the screen's projection, and
//! what it drew becomes the UI half of the frame the renderer presents.

use openpaint_app::ScreenProjection;
use openpaint_render::{ScreenFrame, UiFrame};
use openpaint_ui::{Shell, design};

use crate::canvas::Canvas;

/// What one UI pass needs.
pub(crate) struct UiPass<'a> {
    pub(crate) shell: &'a mut Shell,
    pub(crate) projection: &'a ScreenProjection,
    /// The screen in pixels.
    pub(crate) size: [u32; 2],
    pub(crate) pixels_per_point: f32,
    pub(crate) events: Vec<egui::Event>,
}

/// What one UI pass gives.
pub(crate) struct UiOutput {
    pub(crate) frame: UiFrame,
    /// egui wants the next frame immediately (animations, first-frame layout).
    pub(crate) repaint: bool,
    /// A text field has the focus and waits for typing.
    pub(crate) keyboard: bool,
}

pub(crate) fn screen_frame<'a>(
    canvas: Option<&'a Canvas>,
    shell: &Shell,
    ui: UiFrame,
) -> ScreenFrame<'a> {
    let placement = canvas.zip(shell.canvas_area());
    let pixels_per_point = ui.pixels_per_point;
    ScreenFrame {
        surround: design::SURROUND,
        canvas: placement.map(|(canvas, area)| canvas.placement(&area, pixels_per_point)),
        ui,
    }
}

/// One UI pass over the screen.
pub(crate) fn run_ui(ctx: &egui::Context, pass: UiPass<'_>) -> UiOutput {
    let UiPass {
        shell,
        projection,
        size,
        pixels_per_point,
        events,
    } = pass;
    let points = egui::vec2(size[0] as f32, size[1] as f32) / pixels_per_point;
    let mut input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, points)),
        events,
        ..Default::default()
    };
    input
        .viewports
        .entry(egui::ViewportId::ROOT)
        .or_default()
        .native_pixels_per_point = Some(pixels_per_point);
    let output = ctx.run_ui(input, |ui| shell.show(ui, projection));
    let repaint = output
        .viewport_output
        .get(&egui::ViewportId::ROOT)
        .is_some_and(|viewport| viewport.repaint_delay.is_zero());
    let keyboard = output.platform_output.ime.is_some();
    let primitives = ctx.tessellate(output.shapes, output.pixels_per_point);
    UiOutput {
        frame: UiFrame {
            primitives,
            textures: output.textures_delta,
            pixels_per_point: output.pixels_per_point,
        },
        repaint,
        keyboard,
    }
}
