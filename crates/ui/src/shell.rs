//! The screen: the canvas (or the gallery) on the whole screen, with the controls floating over
//! it as islands (D40). On the canvas: Gallery and Actions in the left island; Brush, Eraser,
//! Layers and the colour in the right one; a notice (a failed save or open, an export, the
//! reference image) in a pill left of it; the side bar (size, eyedropper, opacity, undo, redo)
//! over the left edge; the reference window where the hand left it; the Actions, colour, layer
//! and brush panels under the islands, one at a time. While the fingers move the view, a quiet
//! pill in the top band tells its zoom and turn, then fades. The pen hovering over the canvas
//! shows the brush's outline, the eraser shows it while it erases, and a contact picking a
//! colour shows the loupe (`pen_marks`). In the gallery: its name, "+" in an island, the cards.

use openpaint_app::{
    BrushKey, CanvasView, ContactPhase, EditorAction, GalleryIntent, InputBatch, Intent,
    ScreenProjection, SessionProjection,
};
use openpaint_core::{CanvasSize, Color};
use openpaint_render::{BrushCursor, RgbaImage};

use crate::actions_panel::ActionsPanel;
use crate::brush_panel::BrushPanel;
use crate::canvas_input::{CanvasFrame, CanvasInput};
use crate::color_drop::{ColorDrop, DropFrame};
use crate::color_panel::ColorPanel;
use crate::components::{
    CanvasArea, Place, ScreenArea, canvas_frame, island, text_button, view_status, view_text,
};
use crate::design::{DESK, SPACE, paint_sheet_shadow};
use crate::drop_bar;
use crate::gallery::GalleryScreen;
use crate::layer_panel::LayerPanel;
use crate::notices::notice_pill;
use crate::pen_marks;
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
    /// The colour a drop carries and the threshold it starts with.
    color: Option<Color>,
    threshold: f32,
    /// The current tool's brush diameter in canvas pixels.
    brush_size: Option<f32>,
}

/// The screen's UI-local state between frames.
#[derive(Debug, Default)]
pub struct Shell {
    /// Where the canvas shows and fits, or the gallery's cards.
    canvas: Option<CanvasArea>,
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
    drop: ColorDrop,
    /// Where the fill bar lies, from the last pass.
    drop_bar: Option<ScreenArea>,
    /// Where the top islands and the notice pill lie, from the last pass.
    islands: Vec<ScreenArea>,
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
            self.drop.reset();
            self.drop_bar = None;
            self.showing = showing;
        }
        self.pixels_per_point = ui.ctx().pixels_per_point();
        self.shown = None;
        self.islands.clear();
        let rect = match projection {
            ScreenProjection::Starting => canvas_frame(ui),
            ScreenProjection::Canvas(session) => self.canvas_screen(ui, session),
            ScreenProjection::Gallery(gallery) => {
                self.gallery.islands(ui.ctx(), gallery, &mut self.taps);
                // On the desk, so the list's edge fade has its colour.
                egui::CentralPanel::default()
                    .frame(egui::Frame::new().fill(DESK))
                    .show(ui, |ui| {
                        self.gallery.body(ui, gallery, &mut self.taps);
                        ui.max_rect()
                    })
                    .inner
            }
        };
        self.canvas = Some(CanvasArea::from_points(rect, self.pixels_per_point));
    }

    fn canvas_screen(&mut self, ui: &mut egui::Ui, projection: &SessionProjection) -> egui::Rect {
        let brush = projection.brush();
        self.shown = projection.canvas().map(|canvas| Shown {
            canvas,
            view: projection.view(),
            picking: brush.is_some_and(|brush| brush.picking),
            drawing: projection.drawing(),
            color: brush.map(|brush| brush.color),
            threshold: projection.fill().threshold,
            brush_size: brush.map(|brush| brush.size),
        });
        let rect = canvas_frame(ui);
        let ctx = ui.ctx().clone();
        let area = CanvasArea::from_points(rect, self.pixels_per_point);
        self.show_sheet(&ctx, rect, &area);
        self.show_view(&ctx, &area);
        self.top_islands(&ctx, projection);
        self.side
            .show(&ctx, projection, self.scale(&area), &mut self.taps);
        let now_ms = self.now_ms;
        self.reference
            .show(&ctx, projection, now_ms, &mut self.taps);
        self.actions.show(&ctx, projection, &mut self.taps);
        self.layers.show(&ctx, projection, &mut self.taps);
        self.brushes.show(&ctx, projection, &mut self.taps);
        self.colors.show(&ctx, projection, &mut self.taps);
        self.drop_bar = drop_bar::show(&mut self.drop, &ctx, projection, self.pixels_per_point);
        let picking_at = self.reference.picking_at().or(self.input.picking_at());
        pen_marks::show_loupe(&ctx, picking_at, brush, self.pixels_per_point);
        rect
    }

    /// The left island (Gallery, Actions), the right one (Brush, Eraser, Layers, the colour)
    /// and, left of it, the notice pill. Opening one panel closes the others.
    fn top_islands(&mut self, ctx: &egui::Context, projection: &SessionProjection) {
        let ((), left) = island(ctx, "left island", Place::top_left(), |ui| {
            let enabled = projection.availability(EditorAction::Gallery).is_offered();
            if text_button(ui, "Gallery", enabled).clicked() {
                self.taps.push(Intent::Gallery(GalleryIntent::Show));
            }
            if self.actions.button(ui) {
                self.close_panels_but(Panel::Actions);
            }
        });
        let ((), right) = island(ctx, "right island", Place::top_right(0.0), |ui| {
            if self.brushes.buttons(ui, projection, &mut self.taps) {
                self.close_panels_but(Panel::Brushes);
            }
            if self.layers.button(ui) {
                self.close_panels_but(Panel::Layers);
            }
            if self.colors.button(ui, projection) {
                self.close_panels_but(Panel::Colors);
            }
        });
        let place = Place::top_right(right.width() + SPACE);
        let notice = notice_pill(ctx, projection, place, &mut self.taps);
        let ppp = self.pixels_per_point;
        self.islands = [Some(left), Some(right), notice]
            .into_iter()
            .flatten()
            .map(|rect| ScreenArea::from_points(rect, ppp))
            .collect();
    }

    /// Screen pixels per canvas pixel in `area` now.
    fn scale(&self, area: &CanvasArea) -> f32 {
        self.shown.map_or(1.0, |shown| {
            let viewport = area.viewport(self.pixels_per_point);
            shown.view.mapping(shown.canvas, &viewport).scale()
        })
    }

    /// The canvas's soft shadow on the desk, under everything else and outside the canvas, so
    /// the white sheet stands apart from the paper chrome however the view turns.
    fn show_sheet(&self, ctx: &egui::Context, rect: egui::Rect, area: &CanvasArea) {
        let Some(shown) = self.shown else {
            return;
        };
        let ppp = self.pixels_per_point;
        let mapping = shown.view.mapping(shown.canvas, &area.viewport(ppp));
        let [w, h] = [shown.canvas.width(), shown.canvas.height()].map(|side| side as f32);
        let corners = [[0.0, 0.0], [w, 0.0], [w, h], [0.0, h]].map(|corner| {
            let [x, y] = mapping.to_screen(corner);
            egui::pos2(x / ppp, y / ppp)
        });
        let layer = egui::LayerId::new(egui::Order::Background, egui::Id::new("canvas sheet"));
        let painter = ctx.layer_painter(layer).with_clip_rect(rect);
        paint_sheet_shadow(&painter, corners);
    }

    /// The input clock now (the platform time input samples carry), for the frame about to be
    /// shown and routed; call before `show`.
    pub fn set_now(&mut self, now_ms: i64) {
        self.now_ms = now_ms;
    }

    /// The view status while the fingers move the view and briefly after.
    fn show_view(&self, ctx: &egui::Context, area: &CanvasArea) {
        let (Some(shown), Some(moved)) = (self.shown, self.input.navigated()) else {
            return;
        };
        let since = self.now_ms.saturating_sub(moved);
        let fading = since.saturating_sub(STATUS_HOLD_MS).max(0);
        let strength = 1.0 - fading as f32 / STATUS_FADE_MS as f32;
        if strength <= 0.0 {
            return;
        }
        let text = view_text(self.scale(area), shown.view.rotation());
        view_status(ctx, &text, strength, self.drop.bar().is_some());
    }

    /// The pen's outline in screen pixels: the current brush at full size around where the pen
    /// hovers over open canvas, or, while the pen erases there, `erasing` (the eraser's tip:
    /// centre and radius in canvas pixels); `None` over controls, during multi-touch, while
    /// picking, while the brush draws and off the canvas screen.
    #[must_use]
    pub fn cursor(&self, erasing: Option<([f32; 2], f32)>) -> Option<BrushCursor> {
        let (shown, area) = (self.shown?, self.canvas?);
        let ppp = self.pixels_per_point;
        let mapping = shown.view.mapping(shown.canvas, &area.viewport(ppp));
        let erasing = erasing.filter(|_| self.input.pressing().is_some());
        pen_marks::cursor(self.input.hover(), erasing, shown.brush_size, &mapping, ppp)
    }

    /// True while the screen changes without input: the view status fading, or fingers held
    /// for a shortcut that repeats. The host should keep frames coming.
    #[must_use]
    pub fn animating(&self) -> bool {
        let fading = self.input.navigated().is_some_and(|moved| {
            self.now_ms.saturating_sub(moved) < STATUS_HOLD_MS + STATUS_FADE_MS
        });
        let held = self.input.holding() || self.drop.holding() || self.reference.bar_shown();
        self.showing == Showing::Canvas && (fading || held)
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

    /// Where the canvas shows and fits (the whole screen, fitting under the top band), once a
    /// frame has been laid out.
    #[must_use]
    pub fn canvas_area(&self) -> Option<CanvasArea> {
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
            .chain([self.drop_bar])
            .flatten()
            .chain(self.islands.iter().copied())
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
        let drop = DropFrame {
            swatch: self.colors.button_area(),
            color: shown.color,
            threshold: shown.threshold,
            bar: self.drop_bar,
        };
        let mut intents = self.reference.route(batches, now_ms, shown.picking);
        let (dropped, rest) = self.drop.route(&frame, &drop, batches);
        intents.extend(dropped);
        if self.drop.carried().is_some() {
            self.close_panels();
        }
        intents.extend(self.input.route(&frame, &rest));
        if panel.is_some() && touched_outside(&area.screen, &covers, batches) {
            self.close_panels();
        }
        // Choosing a tool ends continuing to fill.
        if self.taps.iter().any(|tap| matches!(tap, Intent::Tool(_))) {
            self.drop.set_continuing(false);
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
