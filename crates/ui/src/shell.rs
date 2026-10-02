//! The screen skeleton: a top bar over the canvas, which fills the rest of the screen and is
//! left unpainted so the renderer's canvas shows through.

/// Top bar height in points; a 48-point bar holds 48 px hit targets at any density.
pub const TOP_BAR_HEIGHT: f32 = 48.0;

/// Builds one frame of the shell inside the root `ui`.
pub fn shell(ui: &mut egui::Ui) {
    egui::Panel::top("top_bar")
        .exact_size(TOP_BAR_HEIGHT)
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.label("OpenPaint");
            });
        });
}
