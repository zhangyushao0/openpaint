//! The layer panel: the Layers button in the right island opens it as a popover over
//! the canvas (Procreate's model). Header: "Layers", the new group button and the add button.
//! Rows (`layer_rows.rs`): the layers, top of the stack first, each with a picture of its layer
//! alone, the one drawn on highlighted, each with its blend letter and eye, a group's row above
//! its layers; a row held still is dragged to another place (`layer_drag.rs`). Footer
//! (`layer_options.rs`): the highlighted layer's options and its group's. Delete asks nothing:
//! undo brings the layer back, as it does every option. Rename opens a dialog with the name in
//! a text field. Any contact landing on the canvas outside the panel closes it; a pen stroke
//! there also draws.

use std::collections::HashSet;

use openpaint_app::{EditorAction, Intent, LayerIntent, SessionProjection};
use openpaint_core::{GroupId, LayerId, LayerName, MAX_NAME_CHARS};
use openpaint_render::RgbaImage;

use crate::components::{
    Opener, Placement, ScreenArea, answer_row, dialog, icon_button, icon_toggle, popover,
    text_field,
};
use crate::design::{Icon, LAYER_PICTURE, SPACE, TOUCH_TARGET};
use crate::layer_drag::Drag;
use crate::layer_options::{footer, footer_rows};
use crate::layer_rows::{Pictures, RowState, rows};

/// Rows of touch targets the panel keeps free of layer rows besides its footer: the header and
/// the room above and below the panel.
const KEPT_ROWS: f32 = 3.0;

/// Renaming a layer: its name as typed so far.
#[derive(Debug)]
struct Rename {
    layer: LayerId,
    text: String,
    /// The field takes the focus in the next pass.
    focus: bool,
    /// Where Rename was tapped, in points: the dialog grows from there.
    from: egui::Rect,
}

/// The panel's UI-local state.
#[derive(Default)]
pub(crate) struct LayerPanel {
    open: bool,
    /// Where the open panel lies on screen, in pixels, from the last pass.
    area: Option<ScreenArea>,
    /// Where the Layers button lies, in points: the panel comes out of it.
    button: Option<egui::Rect>,
    /// The closed groups: the panel's own state, no artwork change and not stored.
    collapsed: HashSet<GroupId>,
    drag: Option<Drag>,
    rename: Option<Rename>,
    /// Pictures from the runtime waiting for the next pass to load them, and those loaded.
    arrived: Vec<((LayerId, bool), RgbaImage)>,
    pictures: Pictures,
}

impl std::fmt::Debug for LayerPanel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LayerPanel")
            .field("open", &self.open)
            .field("drag", &self.drag)
            .field("rename", &self.rename)
            .field("pictures", &self.pictures.len())
            .finish_non_exhaustive()
    }
}

impl LayerPanel {
    /// The Layers button, in the right island. Returns whether it opened the panel.
    pub(crate) fn button(&mut self, ui: &mut egui::Ui) -> bool {
        let response = icon_toggle(ui, Icon::Layers, "Layers", self.open);
        self.button = Some(response.rect);
        if response.clicked() {
            self.open = !self.open;
            return self.open;
        }
        false
    }

    /// Whether the panel is open: only then are pictures of its layers made.
    pub(crate) const fn is_open(&self) -> bool {
        self.open
    }

    /// The picture of `layer`'s paint, or with `mask` its mask's grey: premultiplied RGBA.
    pub(crate) fn set_picture(&mut self, (layer, mask): (LayerId, bool), image: RgbaImage) {
        self.arrived.push(((layer, mask), image));
    }

    /// Another artwork shows: no picture of the last one stays.
    pub(crate) fn forget_pictures(&mut self) {
        self.drop_pictures();
        self.drag = None;
        self.rename = None;
    }

    /// Lets every picture go (S49: the GPU memory budget's last step); rows show none until
    /// new ones come.
    pub(crate) fn drop_pictures(&mut self) {
        self.arrived.clear();
        self.pictures.clear();
    }

    /// Bytes of the pictures' textures.
    pub(crate) fn picture_bytes(&self) -> u64 {
        let side = |texture: &egui::TextureHandle| {
            let [width, height] = texture.size();
            (width * height * 4) as u64
        };
        self.pictures.values().map(side).sum()
    }

    /// The panel while it is open or closing, its taps pushed to `taps`.
    pub(crate) fn show(
        &mut self,
        ctx: &egui::Context,
        projection: &SessionProjection,
        taps: &mut Vec<Intent>,
    ) {
        self.area = None;
        let Some(list) = projection.layers() else {
            return;
        };
        self.load(ctx, list);
        let opener = Opener {
            island: "right island",
            button: self.button.unwrap_or(egui::Rect::ZERO),
        };
        let picture = picture_size(projection);
        let (collapsed, drag, pictures) = (&mut self.collapsed, &mut self.drag, &self.pictures);
        let mut asked = None;
        let rect = popover(ctx, "layers", (opener, self.open), |ui| {
            header(ui, projection, taps);
            let grouped = list.active_row().is_some_and(|row| row.group.is_some());
            let kept = (KEPT_ROWS + footer_rows(grouped, list.on_mask)) * TOUCH_TARGET;
            let room = ui.ctx().content_rect().height() - kept;
            let enabled = projection
                .availability(EditorAction::EditLayer)
                .is_offered();
            let state = RowState {
                collapsed,
                drag,
                pictures,
                picture,
            };
            rows(ui, (list, enabled), room, state, taps);
            asked = footer(ui, list, projection, taps);
        });
        if let (Some(from), Some(active)) = (asked, list.active_row()) {
            self.rename = Some(Rename {
                layer: active.id,
                text: active.name.as_str().to_owned(),
                focus: true,
                from,
            });
        }
        self.area = rect.map(|rect| ScreenArea::from_points(rect, ctx.pixels_per_point()));
        if self.rename.is_some() {
            // The dialog answers every contact: none reaches the canvas.
            let screen = ctx.content_rect();
            self.area = Some(ScreenArea::from_points(screen, ctx.pixels_per_point()));
            self.rename_dialog(ctx, taps);
        }
    }

    /// The name in a text field with Cancel and Done; Done (or Enter) renames when the text is
    /// a name; Cancel or a tap outside keeps the old one.
    fn rename_dialog(&mut self, ctx: &egui::Context, taps: &mut Vec<Intent>) {
        let Some(rename) = &mut self.rename else {
            return;
        };
        let from = (Placement::Top, rename.from);
        let ((cancel, done), outside) = dialog(ctx, "rename layer", from, |ui| {
            ui.heading("Rename layer");
            let field = text_field(ui, "Name", &mut rename.text, MAX_NAME_CHARS, rename.focus);
            if field.response.has_focus() {
                rename.focus = false;
            }
            let valid = LayerName::new(&rename.text).is_ok();
            let (cancel, done) = answer_row(ui, "Cancel", "Done", valid);
            (cancel, done || (field.submitted && valid))
        });
        if done && let Ok(name) = LayerName::new(&rename.text) {
            taps.push(Intent::Layer(LayerIntent::Rename {
                layer: rename.layer,
                name,
            }));
            self.rename = None;
        } else if cancel || outside {
            self.rename = None;
        }
    }

    /// The back button while renaming: the dialog closes, the name stays. Returns whether it
    /// was open.
    pub(crate) fn cancel_rename(&mut self) -> bool {
        self.rename.take().is_some()
    }

    /// Loads the pictures that arrived; drops those of layers no longer listed.
    fn load(&mut self, ctx: &egui::Context, list: &openpaint_app::LayerList) {
        for ((layer, mask), image) in self.arrived.drain(..) {
            let [width, height] = image.size().map(|side| side as usize);
            let picture =
                egui::ColorImage::from_rgba_premultiplied([width, height], image.pixels());
            let name = format!("layer picture {} {mask}", layer.get());
            let texture = ctx.load_texture(name, picture, egui::TextureOptions::LINEAR);
            self.pictures.insert((layer, mask), texture);
        }
        self.pictures.retain(|(layer, mask), _| {
            list.rows
                .iter()
                .any(|row| row.id == *layer && (!*mask || row.mask.is_some()))
        });
    }

    /// Where the open panel lies, which the canvas must not take contacts from.
    pub(crate) fn area(&self) -> Option<ScreenArea> {
        self.area
    }

    /// One tap outside closes the panel.
    pub(crate) fn close(&mut self) {
        self.open = false;
        self.area = None;
        self.drag = None;
    }
}

/// The canvas fitted in `LAYER_PICTURE`, in points.
fn picture_size(projection: &SessionProjection) -> egui::Vec2 {
    let Some(canvas) = projection.canvas() else {
        return LAYER_PICTURE;
    };
    let [width, height] = [canvas.width(), canvas.height()].map(|side| side.max(1) as f32);
    let scale = (LAYER_PICTURE.x / width).min(LAYER_PICTURE.y / height);
    egui::vec2(width * scale, height * scale)
}

/// The largest picture of a layer in pixels at `pixels_per_point`: what the runtime makes.
#[must_use]
pub(crate) fn picture_pixels(pixels_per_point: f32) -> [u32; 2] {
    [LAYER_PICTURE.x, LAYER_PICTURE.y].map(|side| (side * pixels_per_point).round().max(1.0) as u32)
}

fn header(ui: &mut egui::Ui, projection: &SessionProjection, taps: &mut Vec<Intent>) {
    ui.horizontal(|ui| {
        ui.set_height(TOUCH_TARGET);
        ui.add_space(2.0 * SPACE);
        ui.heading("Layers");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let enabled = projection.availability(EditorAction::AddLayer).is_offered();
            if icon_button(ui, Icon::Add, "Add layer", enabled).clicked() {
                taps.push(Intent::Layer(LayerIntent::Add));
            }
            if icon_button(ui, Icon::Group, "New group", enabled).clicked() {
                taps.push(Intent::Layer(LayerIntent::NewGroup));
            }
        });
    });
}
