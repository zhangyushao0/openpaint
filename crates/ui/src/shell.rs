//! The screen: a top bar over the rest. On the canvas the bar holds Gallery and Actions at its
//! left end and at its right end, right to left, the colour button, Layers, Eraser, Brush and a
//! notice (a failed save or open, an export, the reference image); the canvas frame fills the
//! rest, the side bar (size, eyedropper, opacity, undo, redo) floats over its left edge, the
//! reference window floats where the hand left it, and the Actions, colour, layer and brush
//! panels open over it, one at a time. While the fingers move the view, a quiet line under the
//! bar tells its zoom and turn, then fades.
//! In the gallery the bar holds the screen's name and "+", and the cards fill the rest.

use egui::{Align, Layout};
use openpaint_app::{
    BrushKey, CanvasView, ContactPhase, EditorAction, GalleryIntent, InputBatch, Intent,
    ScreenProjection, SessionProjection,
};
use openpaint_core::CanvasSize;
use openpaint_render::RgbaImage;

use crate::actions_panel::ActionsPanel;
use crate::brush_panel::BrushPanel;
use crate::canvas_input::{CanvasFrame, CanvasInput};
use crate::color_panel::ColorPanel;
use crate::components::{ScreenArea, canvas_frame, text_button, top_bar, view_status, view_text};
use crate::gallery::GalleryScreen;
use crate::layer_panel::LayerPanel;
use crate::notices::top_bar_notice;
use crate::reference_window::ReferenceWindow;
use crate::side_bar::SideBar;

/// The popovers, one open at a time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Panel {
    Actions,
    Layers,
    Brushes,
    Colors,
}

/// Which screen the last pass showed.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum Showing {
    #[default]
    Starting,
    Canvas,
    Gallery,
}

/// The view status shows this long after the view stopped, then fades for `STATUS_FADE_MS`.
const STATUS_HOLD_MS: i64 = 700;
const STATUS_FADE_MS: i64 = 300;

/// What the canvas showed in the last pass, for routing this frame's contacts.
#[derive(Debug, Clone, Copy)]
struct Shown {
    canvas: CanvasSize,
    view: CanvasView,
    picking: bool,
    drawing: bool,
}

/// The screen's UI-local state between frames.
#[derive(Debug, Default)]
pub struct Shell {
    /// The area under the top bar: where the canvas shows, or the gallery's cards.
    canvas: Option<ScreenArea>,
    showing: Showing,
    shown: Option<Shown>,
    pixels_per_point: f32,
    /// The input clock for this frame (`set_now`).
    now_ms: i64,
    input: CanvasInput,
    actions: ActionsPanel,
    layers: LayerPanel,
    brushes: BrushPanel,
    colors: ColorPanel,
    side: SideBar,
    reference: ReferenceWindow,
    gallery: GalleryScreen,
    /// Intents from taps on the chrome since the last `route`.
    taps: Vec<Intent>,
}

impl Shell {
    /// The brush picker's preview of each brush, drawn by the renderer at the screen's density
    /// (`design::BRUSH_PREVIEW` points); loaded as textures by the next pass.
    pub fn set_brush_previews(&mut self, previews: Vec<(BrushKey, RgbaImage)>) {
        self.brushes.set_previews(previews);
    }

    /// Builds one frame inside the root `ui` from `projection`, remembers where the canvas is
    /// and collects the buttons tapped. egui hands input to the first pass of a frame only, so
    /// a tap is collected once however many passes run.
    pub fn show(&mut self, ui: &mut egui::Ui, projection: &ScreenProjection) {
        let showing = match projection {
            ScreenProjection::Starting => Showing::Starting,
            ScreenProjection::Canvas(_) => Showing::Canvas,
            ScreenProjection::Gallery(_) => Showing::Gallery,
        };
        if showing != self.showing {
            // A new screen starts with its panels and dialogs closed.
            self.close_panels();
            self.side.hide();
            self.gallery.back();
            self.input = CanvasInput::default();
            self.showing = showing;
        }
        self.pixels_per_point = ui.ctx().pixels_per_point();
        self.shown = None;
        let rect = match projection {
            ScreenProjection::Starting => {
                top_bar(ui, |ui| {
                    ui.label("OpenPaint");
                });
                canvas_frame(ui)
            }
            ScreenProjection::Canvas(session) => self.canvas_screen(ui, session),
            ScreenProjection::Gallery(gallery) => {
                top_bar(ui, |ui| self.gallery.bar(ui, gallery, &mut self.taps));
                egui::CentralPanel::no_frame()
                    .show(ui, |ui| {
                        self.gallery.body(ui, gallery, &mut self.taps);
                        ui.max_rect()
                    })
                    .inner
            }
        };
        self.canvas = Some(ScreenArea::from_points(rect, ui.ctx().pixels_per_point()));
    }

    fn canvas_screen(&mut self, ui: &mut egui::Ui, projection: &SessionProjection) -> egui::Rect {
        self.shown = projection.canvas().map(|canvas| Shown {
            canvas,
            view: projection.view(),
            picking: projection.brush().is_some_and(|brush| brush.picking),
            drawing: projection.drawing(),
        });
        top_bar(ui, |ui| {
            let enabled = projection.availability(EditorAction::Gallery).is_offered();
            if text_button(ui, "Gallery", enabled).clicked() {
                self.taps.push(Intent::Gallery(GalleryIntent::Show));
            }
            // One panel at a time: opening one closes the others.
            if self.actions.button(ui) {
                self.close_panels_but(Panel::Actions);
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if self.colors.button(ui, projection) {
                    self.close_panels_but(Panel::Colors);
                }
                if self.layers.button(ui) {
                    self.close_panels_but(Panel::Layers);
                }
                if self.brushes.buttons(ui, projection, &mut self.taps) {
                    self.close_panels_but(Panel::Brushes);
                }
                top_bar_notice(ui, projection, &mut self.taps);
            });
        });
        let rect = canvas_frame(ui);
        let ctx = ui.ctx();
        self.show_view(ctx, rect);
        self.side.show(ctx, projection, &mut self.taps);
        self.reference.show(ctx, projection, &mut self.taps);
        self.actions.show(ctx, projection, &mut self.taps);
        self.layers.show(ctx, projection, &mut self.taps);
        self.brushes.show(ctx, projection, &mut self.taps);
        self.colors.show(ctx, projection, &mut self.taps);
        rect
    }

    /// The input clock now (the platform time input samples carry), for the frame about to be
    /// shown and routed; call before `show`.
    pub fn set_now(&mut self, now_ms: i64) {
        self.now_ms = now_ms;
    }

    /// The view status while the fingers move the view and briefly after.
    fn show_view(&self, ctx: &egui::Context, rect: egui::Rect) {
        let (Some(shown), Some(moved)) = (self.shown, self.input.navigated()) else {
            return;
        };
        let since = self.now_ms.saturating_sub(moved);
        let fading = since.saturating_sub(STATUS_HOLD_MS).max(0);
        let strength = 1.0 - fading as f32 / STATUS_FADE_MS as f32;
        if strength <= 0.0 {
            return;
        }
        let area = ScreenArea::from_points(rect, self.pixels_per_point);
        let viewport = area.viewport(self.pixels_per_point);
        let scale = shown.view.mapping(shown.canvas, &viewport).scale();
        view_status(ctx, &view_text(scale, shown.view.rotation()), strength);
    }

    /// True while the screen changes without input: the view status fading, or fingers held
    /// for a shortcut that repeats. The host should keep frames coming.
    #[must_use]
    pub fn animating(&self) -> bool {
        let fading = self.input.navigated().is_some_and(|moved| {
            self.now_ms.saturating_sub(moved) < STATUS_HOLD_MS + STATUS_FADE_MS
        });
        self.showing == Showing::Canvas && (fading || self.input.holding())
    }

    fn close_panels(&mut self) {
        self.actions.close();
        self.layers.close();
        self.brushes.close();
        self.colors.close();
    }

    fn close_panels_but(&mut self, open: Panel) {
        if open != Panel::Actions {
            self.actions.close();
        }
        if open != Panel::Layers {
            self.layers.close();
        }
        if open != Panel::Brushes {
            self.brushes.close();
        }
        if open != Panel::Colors {
            self.colors.close();
        }
    }

    /// The system's back button: from the canvas it goes to the gallery; in the gallery it
    /// closes an open dialog. Returns whether it was used; otherwise the system's back applies.
    pub fn back(&mut self, projection: &ScreenProjection) -> bool {
        match projection {
            ScreenProjection::Canvas(_) => {
                self.taps.push(Intent::Gallery(GalleryIntent::Show));
                true
            }
            ScreenProjection::Gallery(_) => self.gallery.back(),
            ScreenProjection::Starting => false,
        }
    }

    /// Where the canvas shows on screen (the area under the top bar), once a frame has been
    /// laid out.
    #[must_use]
    pub fn canvas_area(&self) -> Option<ScreenArea> {
        self.canvas
    }

    /// The intents this frame asks for: on the canvas its contacts first, then the taps on the
    /// chrome, so a stroke lifted just before undo is tapped is committed before it is undone.
    /// Call after `show`.
    pub fn route(&mut self, batches: &[InputBatch]) -> Vec<Intent> {
        let now_ms = self.now_ms;
        let area = self.canvas.filter(|_| self.showing == Showing::Canvas);
        let (Some(area), Some(shown)) = (area, self.shown) else {
            return std::mem::take(&mut self.taps);
        };
        let panel = self
            .actions
            .area()
            .or(self.layers.area())
            .or(self.brushes.area())
            .or(self.colors.area());
        let covers: Vec<ScreenArea> = [self.side.area(), self.reference.area(), panel]
            .into_iter()
            .flatten()
            .collect();
        let frame = CanvasFrame {
            area,
            covers: &covers,
            canvas: shown.canvas,
            view: shown.view,
            pixels_per_point: self.pixels_per_point,
            now_ms,
            picking: shown.picking,
            drawing: shown.drawing,
            control: !self.taps.is_empty(),
        };
        let mut intents = self.input.route(&frame, batches);
        if panel.is_some() && touched_outside(&area, &covers, batches) {
            self.close_panels();
        }
        intents.append(&mut self.taps);
        intents
    }
}

/// True when a contact of `batches` touches down on the canvas `area` outside every one of
/// `covers`.
fn touched_outside(area: &ScreenArea, covers: &[ScreenArea], batches: &[InputBatch]) -> bool {
    batches
        .iter()
        .flat_map(|batch| &batch.contacts)
        .filter(|contact| contact.phase == ContactPhase::Down)
        .filter_map(|contact| contact.samples.first())
        .any(|first| {
            area.contains(first.position)
                && !covers.iter().any(|cover| cover.contains(first.position))
        })
}
