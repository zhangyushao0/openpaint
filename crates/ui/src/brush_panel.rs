//! The brush picker and the tool buttons: Brush, Smudge and Eraser sit in the right island before
//! Layers (Procreate's order). The current tool's button opens the picker as a popover over the
//! canvas (Procreate's brush library, cut to five fixed sets, S47): header "Brushes", "Smudge"
//! or "Eraser", the sets as words, then one row per brush of the set shown with its name and a
//! stroke it painted, the tool's brush outlined in ink (`brush_list.rs`). The smudge and the
//! eraser choose from the same brushes and sets. A tap chooses the brush; the panel stays until a
//! contact lands on the canvas outside it, and a pen stroke there draws with the brush just
//! chosen. Under the rows, the chosen brush's StreamLine slider (0 to 100 %). Tapping the chosen
//! brush's row again turns the panel into its editor (`brush_editor.rs`, S46) from any of the
//! three tools, as all three draw with the same brushes; Back, or the panel closing, returns to
//! the list, on the set the editor's Set row moved the brush to, if it did.

use std::collections::HashMap;
use std::fmt;

use openpaint_app::{BrushKey, BrushProjection, Intent, SessionProjection, Tool, ToolIntent};
use openpaint_render::RgbaImage;

use crate::brush_editor::BrushEditor;
use crate::brush_list::{ListState, list};
use crate::brush_strip::StripStrokes;
use crate::components::{Opener, ScreenArea, choices, icon_choice, popover};
use crate::design::{INK, Icon};

/// The panel's UI-local state and the brushes' preview pictures.
#[derive(Default)]
pub(crate) struct BrushPanel {
    open: bool,
    /// Where the open panel lies on screen, in pixels, from the last pass.
    area: Option<ScreenArea>,
    /// Where the current tool's button lies, in points: the panel comes out of it.
    button: Option<egui::Rect>,
    /// Previews waiting for the next pass, which loads them as textures.
    pending: Vec<(BrushKey, RgbaImage)>,
    textures: HashMap<BrushKey, egui::TextureHandle>,
    /// The editor shows in the panel's place.
    editor: Option<BrushEditor>,
    /// The set the list shows and a row the hand carries; anew each time the panel opens.
    list: ListState,
}

impl fmt::Debug for BrushPanel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BrushPanel")
            .field("open", &self.open)
            .field("area", &self.area)
            .field("previews", &(self.pending.len() + self.textures.len()))
            .field("editor", &self.editor)
            .field("list", &self.list)
            .finish()
    }
}

impl BrushPanel {
    pub(crate) fn set_previews(&mut self, previews: Vec<(BrushKey, RgbaImage)>) {
        self.pending = previews;
        self.textures.clear();
    }

    /// One brush's preview, made anew since it changed.
    pub(crate) fn set_picture(&mut self, key: BrushKey, image: RgbaImage) {
        self.pending.push((key, image));
    }

    /// The strip's strokes while the editor shows.
    pub(crate) fn strip(&self) -> Option<StripStrokes> {
        self.editor.as_ref()?.strip.strokes()
    }

    pub(crate) fn set_strip_picture(&mut self, image: RgbaImage) {
        if let Some(editor) = &mut self.editor {
            editor.strip.set_picture(image);
        }
    }

    /// Whether the list of brushes shows: only then are pictures of changed brushes made.
    pub(crate) const fn listed(&self) -> bool {
        self.open && self.editor.is_none()
    }

    /// The pen's contacts on the editor's strip, before the canvas sees them.
    pub(crate) fn route_strip(&mut self, batches: &[openpaint_app::InputBatch]) {
        if let Some(editor) = &mut self.editor {
            editor.strip.route(batches);
        }
    }

    /// The Brush, Smudge and Eraser buttons, left to right, in the right island; the one of the
    /// pen's tool carries the ink ring (Procreate's model), which slides to the tool chosen;
    /// none while the Selection or the Transform tool is on (the ring fades where it is).
    /// Tapping the other tool's button makes the pen use it (and leaves the Selection tool);
    /// tapping the current tool's button opens or closes its brushes. Returns whether a tap
    /// opened the panel.
    pub(crate) fn buttons(
        &mut self,
        ui: &mut egui::Ui,
        projection: &SessionProjection,
        taps: &mut Vec<Intent>,
    ) -> bool {
        let Some(brush) = projection.brush() else {
            return false;
        };
        // While the Selection or the Transform tool is on, the pen does not paint: no ring.
        let other = projection.selection().tool || projection.transform().tool;
        choices(ui, "tools", |ui, group| {
            let mut opened = false;
            for (tool, icon, label) in [
                (Tool::Brush, Icon::Brush, "Brushes"),
                (Tool::Smudge, Icon::Smudge, "Smudge"),
                (Tool::Eraser, Icon::Eraser, "Eraser"),
            ] {
                let current = brush.tool == tool && !other;
                let response = icon_choice(ui, group, (icon, label), current);
                if brush.tool == tool {
                    self.button = Some(response.rect);
                }
                if !response.clicked() {
                    continue;
                }
                if current {
                    self.open = !self.open;
                    self.editor = None;
                    self.list = ListState::default();
                    opened = self.open;
                } else {
                    self.close();
                    taps.push(Intent::Tool(ToolIntent::Use(tool)));
                }
            }
            opened
        })
    }

    /// The panel while it is open or closing, its taps pushed to `taps`.
    pub(crate) fn show(
        &mut self,
        ctx: &egui::Context,
        projection: &SessionProjection,
        taps: &mut Vec<Intent>,
    ) {
        self.load(ctx);
        self.area = None;
        let Some(brush) = projection.brush() else {
            return;
        };
        let opener = Opener {
            island: "right island",
            button: self.button.unwrap_or(egui::Rect::ZERO),
        };
        let (open, textures) = (self.open, &self.textures);
        let mut back = false;
        let mut edit = false;
        let (editor, state) = (&mut self.editor, &mut self.list);
        if let Some(editor) = editor.as_mut() {
            editor.strip.hide();
        }
        let rect = popover(ctx, "brushes", (opener, open), |ui| match editor.as_mut() {
            Some(editor) if open => back = editor.page(ui, brush, taps),
            Some(_) => {}
            None => edit = list(ui, brush, (state, textures), taps),
        });
        // The editor's Set row moved the brush: the list shows that set when it is back.
        if let Some(set) = self.editor.as_mut().and_then(|editor| editor.moved.take()) {
            self.list.shown = Some(set);
        }
        if back {
            self.editor = None;
        } else if edit {
            self.list.drag = None;
            self.editor = Some(BrushEditor::default());
        }
        self.area = rect.map(|rect| ScreenArea::from_points(rect, ctx.pixels_per_point()));
        self.dialogs(ctx, brush, taps);
    }

    /// The editor's dialogs over the screen, which then answers every contact.
    fn dialogs(&mut self, ctx: &egui::Context, brush: &BrushProjection, taps: &mut Vec<Intent>) {
        let Some(editor) = self.editor.as_mut().filter(|_| self.open) else {
            return;
        };
        let (shown, deleted) = editor.dialogs(ctx, brush, taps);
        if shown {
            let screen = ctx.content_rect();
            self.area = Some(ScreenArea::from_points(screen, ctx.pixels_per_point()));
        }
        if deleted {
            self.editor = None;
        }
    }

    /// The back button while the editor shows: its dialog, its dynamics page or itself goes.
    /// Returns whether it was used.
    pub(crate) fn back(&mut self) -> bool {
        let Some(editor) = self.editor.as_mut().filter(|_| self.open) else {
            return false;
        };
        if !editor.back() {
            self.editor = None;
        }
        true
    }

    /// Where the open panel lies, which the canvas must not take contacts from.
    pub(crate) const fn area(&self) -> Option<ScreenArea> {
        self.area
    }

    pub(crate) fn close(&mut self) {
        self.open = false;
        self.area = None;
        self.editor = None;
        self.list = ListState::default();
    }

    fn load(&mut self, ctx: &egui::Context) {
        for (key, image) in self.pending.drain(..) {
            let [width, height] = image.size().map(|side| side as usize);
            let picture = egui::ColorImage::from_rgba_unmultiplied([width, height], &ink(&image));
            let options = egui::TextureOptions::LINEAR;
            match self.textures.get_mut(&key) {
                Some(texture) => texture.set(picture, options),
                None => {
                    let name = format!("brush preview {}", key.as_str());
                    let texture = ctx.load_texture(name, picture, options);
                    self.textures.insert(key, texture);
                }
            }
        }
    }
}

/// A preview painted in black on white as `INK` with the paint's strength as alpha, so it sits
/// on any panel. The evaluator blends on encoded values: a pixel's share of the way from white
/// to black is that strength.
fn ink(image: &RgbaImage) -> Vec<u8> {
    let (pixels, _) = image.pixels().as_chunks::<4>();
    let mut out = Vec::with_capacity(image.pixels().len());
    for [value, ..] in pixels {
        out.extend_from_slice(&[INK.r(), INK.g(), INK.b(), 255 - value]);
    }
    out
}
