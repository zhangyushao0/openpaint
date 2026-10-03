//! The screen: the canvas (or the gallery) on the whole screen, with the controls floating over
//! it as islands (D40). On the canvas: Gallery and Actions in the left island; Brush, Eraser,
//! Layers and the colour in the right one; a notice (a failed save or open, an export, the
//! reference image) in a pill left of it; the side bar (size, eyedropper, opacity, undo, redo)
//! over the left edge; the reference window where the hand left it; the Actions, colour, layer
//! and brush panels under the islands, one at a time. While the fingers move the view, a quiet
//! pill in the top band tells its zoom and turn, then fades. The pen hovering over the canvas
//! shows the brush's outline, the eraser shows it while it erases, and a contact picking a
//! colour shows the loupe (`pen_marks`). In the gallery: its name, "+" in an island, the cards.
//! What comes and goes moves (D44): panels, pills, dialogs, the loupe and the reference window
//! grow out of where they come from; an artwork grows out of its card and goes back into it; a
//! quick pinch settles the view at fit.

use openpaint_app::{
    BrushKey, CanvasView, EditorAction, GalleryIntent, InputBatch, Intent, ScreenProjection,
    SessionProjection, ViewIntent, ViewMapping,
};
use openpaint_core::{CanvasSize, Color};
use openpaint_render::{BrushCursor, RgbaImage};

use crate::canvas_input::{CanvasFrame, CanvasInput, touched_outside};
use crate::color_drop::{ColorDrop, DropFrame};
use crate::components::carry::{Opening, sheet_at};
use crate::components::{
    CanvasArea, Place, ScreenArea, ViewSettle, canvas_frame, chrome, island, moving, text_button,
};
use crate::design::motion::{OPEN, SETTLE};
use crate::design::{DESK, SPACE};
use crate::drop_bar;
use crate::gallery::GalleryScreen;
use crate::notices::notice_pill;
use crate::panels::{Panel, Panels};
use crate::pen_marks;
use crate::reference_window::ReferenceWindow;
use crate::selection_bar::{self, SelectionBar};
use crate::sheet::{self, Showing};
use crate::side_bar::SideBar;

/// What the canvas showed in the last pass, for routing this frame's contacts.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Shown {
    pub(crate) canvas: CanvasSize,
    pub(crate) view: CanvasView,
    pub(crate) picking: bool,
    /// The Selection tool is on.
    pub(crate) selecting: bool,
    pub(crate) drawing: bool,
    /// The colour a drop carries and the threshold it starts with.
    pub(crate) color: Option<Color>,
    pub(crate) threshold: f32,
    /// The current tool's brush diameter in canvas pixels.
    pub(crate) brush_size: Option<f32>,
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
    pub(crate) now_ms: i64,
    pub(crate) input: CanvasInput,
    panels: Panels,
    side: SideBar,
    selection: SelectionBar,
    reference: ReferenceWindow,
    pub(crate) drop: ColorDrop,
    /// Where the fill bar lies, from the last pass.
    drop_bar: Option<ScreenArea>,
    /// Where the top islands and the notice pill lie, from the last pass.
    islands: Vec<ScreenArea>,
    gallery: GalleryScreen,
    /// Intents from taps on the chrome since the last `route`.
    taps: Vec<Intent>,
    /// The view the screen shows while it settles, and the artwork on its way from its card.
    view: ViewSettle,
    opening: Option<Opening>,
    /// Which artwork the canvas came from, and where it showed last (points).
    came_from: Option<openpaint_app::ProjectId>,
    shown_at: Option<egui::Rect>,
    /// Something moved in the last pass.
    moving: bool,
}

impl Shell {
    /// The brush picker's preview of each brush, drawn by the renderer at the screen's density
    /// (`design::BRUSH_PREVIEW` points); loaded as textures by the next pass.
    pub fn set_brush_previews(&mut self, previews: Vec<(BrushKey, RgbaImage)>) {
        self.panels.brushes.set_previews(previews);
    }

    /// Builds one frame inside the root `ui` from `projection`, remembers where the canvas is
    /// and collects the buttons tapped. egui hands input to the first pass of a frame only, so
    /// a tap is collected once however many passes run.
    pub fn show(&mut self, ui: &mut egui::Ui, projection: &ScreenProjection) {
        let ctx = ui.ctx().clone();
        chrome::begin(&ctx);
        let showing = Showing::of(projection);
        if showing != self.showing {
            self.switch(&ctx, showing);
        }
        self.pixels_per_point = ctx.pixels_per_point();
        self.shown = None;
        self.islands.clear();
        let rect = match projection {
            ScreenProjection::Starting => {
                let rect = canvas_frame(ui);
                if let Some(opening) = &self.opening {
                    sheet_at(&ctx, opening.picture.as_ref(), opening.from);
                }
                rect
            }
            ScreenProjection::Canvas(session) => self.canvas_screen(ui, session),
            ScreenProjection::Gallery(gallery) => {
                self.gallery.islands(&ctx, gallery, &mut self.taps);
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
        chrome::finish(&ctx);
        self.moving = moving(&ctx);
    }

    /// A new screen starts with its panels and dialogs closed. An artwork opened from its
    /// card grows out of it; the canvas left goes back into its card.
    fn switch(&mut self, ctx: &egui::Context, showing: Showing) {
        chrome::forget(ctx);
        self.panels.close();
        self.side.hide();
        self.input = CanvasInput::default();
        self.drop.reset();
        self.drop_bar = None;
        self.view.stop();
        match (self.showing, showing) {
            (Showing::Gallery, _) => {
                self.opening = self.gallery.take_opening();
                self.came_from = self.opening.as_ref().and_then(|opening| opening.id);
            }
            (Showing::Canvas, Showing::Gallery) => {
                if let Some(from) = self.shown_at.take() {
                    self.gallery.arrive(from, self.came_from);
                }
            }
            _ => {}
        }
        if showing != Showing::Starting {
            self.gallery.back();
        }
        self.showing = showing;
    }

    fn canvas_screen(&mut self, ui: &mut egui::Ui, projection: &SessionProjection) -> egui::Rect {
        let brush = projection.brush();
        self.shown = projection.canvas().map(|canvas| Shown {
            canvas,
            view: projection.view(),
            picking: brush.is_some_and(|brush| brush.picking),
            selecting: projection.selection().tool,
            drawing: projection.drawing(),
            color: brush.map(|brush| brush.color),
            threshold: projection.fill().threshold,
            brush_size: brush.map(|brush| brush.size),
        });
        let rect = canvas_frame(ui);
        let ctx = ui.ctx().clone();
        let area = CanvasArea::from_points(rect, self.pixels_per_point);
        self.arrived();
        if let Some(mapping) = self.mapping(&area) {
            self.view.advance(&ctx);
            let shown = self.view.shown(mapping);
            self.shown_at =
                sheet::show_sheet(&ctx, rect, self.shown, &shown, self.pixels_per_point);
            selection_bar::show_path(&ctx, projection, &shown, self.pixels_per_point);
        }
        sheet::show_view(self, &ctx, &area);
        self.top_islands(&ctx, projection);
        self.selection.show(&ctx, projection, &mut self.taps);
        self.side
            .show(&ctx, projection, self.scale(&area), &mut self.taps);
        let opener = self.panels.actions.button_rect();
        self.reference
            .show(&ctx, projection, (self.now_ms, opener), &mut self.taps);
        self.panels.show(&ctx, projection, &mut self.taps);
        self.drop_bar = drop_bar::show(&mut self.drop, &ctx, projection, self.pixels_per_point);
        let picking_at = self.reference.picking_at().or(self.input.picking_at());
        pen_marks::show_loupe(&ctx, picking_at, brush, self.pixels_per_point);
        rect
    }

    /// The first canvas frame after an artwork was opened from its card: the canvas starts
    /// growing out of where the card showed it.
    fn arrived(&mut self) {
        if let (Some(opening), Some(shown)) = (self.opening.take(), self.shown) {
            let ppp = self.pixels_per_point;
            let from = opening.from;
            let origin = [from.min.x * ppp, from.min.y * ppp];
            let size = [from.width() * ppp, from.height() * ppp];
            self.view
                .start(ViewMapping::placed(shown.canvas, origin, size), OPEN);
        }
    }

    /// The left island (Gallery, Actions), the right one (Brush, Eraser, Layers, the colour)
    /// and, left of it, the notice pill. Opening one panel closes the others.
    fn top_islands(&mut self, ctx: &egui::Context, projection: &SessionProjection) {
        let panels = &mut self.panels;
        let ((), left) = island(ctx, "left island", Place::top_left(), |ui| {
            let enabled = projection.availability(EditorAction::Gallery).is_offered();
            if text_button(ui, "Gallery", enabled).clicked() {
                self.taps.push(Intent::Gallery(GalleryIntent::Show));
            }
            if panels.actions.button(ui) {
                panels.close_but(Panel::Actions);
            }
            if self.selection.button(ui, projection, &mut self.taps) {
                panels.close();
            }
        });
        let ((), right) = island(ctx, "right island", Place::top_right(0.0), |ui| {
            if panels.brushes.buttons(ui, projection, &mut self.taps) {
                panels.close_but(Panel::Brushes);
            }
            if panels.layers.button(ui) {
                panels.close_but(Panel::Layers);
            }
            if panels.colors.button(ui, projection) {
                panels.close_but(Panel::Colors);
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

    /// The view's map in `area` now, while the canvas shows.
    fn mapping(&self, area: &CanvasArea) -> Option<ViewMapping> {
        let shown = self.shown?;
        let viewport = area.viewport(self.pixels_per_point);
        Some(shown.view.mapping(shown.canvas, &viewport))
    }

    /// What the canvas showed in this pass.
    pub(crate) const fn shown(&self) -> Option<Shown> {
        self.shown
    }

    /// Screen pixels per canvas pixel in `area` now.
    pub(crate) fn scale(&self, area: &CanvasArea) -> f32 {
        self.mapping(area).map_or(1.0, |mapping| mapping.scale())
    }

    /// What the screen shows of the canvas whose view maps as `mapping` this frame: the same,
    /// or, while the view settles (a quick pinch, an artwork opened from its card), the way
    /// there. Input never uses it.
    #[must_use]
    pub fn displayed(&self, mapping: ViewMapping) -> ViewMapping {
        self.view.shown(mapping)
    }

    /// The input clock now (the platform time input samples carry), for the frame about to be
    /// shown and routed; call before `show`.
    pub fn set_now(&mut self, now_ms: i64) {
        self.now_ms = now_ms;
    }

    /// The pen's outline in screen pixels: the current brush at full size around where the pen
    /// hovers over open canvas, or, while the pen erases there, `erasing` (the eraser's tip:
    /// centre and radius in canvas pixels); `None` over controls, during multi-touch, while
    /// picking, while the brush draws and off the canvas screen.
    #[must_use]
    pub fn cursor(&self, erasing: Option<([f32; 2], f32)>) -> Option<BrushCursor> {
        // While the Selection tool is on the pen draws no paint: no brush outline.
        let (shown, area) = (self.shown.filter(|shown| !shown.selecting)?, self.canvas?);
        let ppp = self.pixels_per_point;
        let mapping = shown.view.mapping(shown.canvas, &area.viewport(ppp));
        let erasing = erasing.filter(|_| self.input.pressing().is_some());
        pen_marks::cursor(self.input.hover(), erasing, shown.brush_size, &mapping, ppp)
    }

    /// True while the screen changes without input: something moving (D44), the view status
    /// fading, or fingers held for a shortcut that repeats. The host should keep frames coming.
    #[must_use]
    pub fn animating(&self) -> bool {
        let fading = self.input.navigated().is_some_and(|moved| {
            self.now_ms.saturating_sub(moved) < sheet::STATUS_HOLD_MS + sheet::STATUS_FADE_MS
        });
        let held = self.input.holding() || self.drop.holding() || self.reference.bar_shown();
        self.moving || (self.showing == Showing::Canvas && (fading || held))
    }

    /// True while something moves (D44): a scenario steps its clock until it rests.
    #[must_use]
    pub const fn moving(&self) -> bool {
        self.moving
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
        let panel = self.panels.area();
        let covers: Vec<ScreenArea> = [self.side.area(), self.reference.area(), panel]
            .into_iter()
            .chain([self.drop_bar, self.selection.area()])
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
            selecting: shown.selecting,
            drawing: shown.drawing,
            control: !self.taps.is_empty(),
        };
        let drop = DropFrame {
            swatch: self.panels.colors.button_area(),
            color: shown.color,
            threshold: shown.threshold,
            bar: self.drop_bar,
        };
        let mut intents = self.reference.route(batches, now_ms, shown.picking);
        let (dropped, rest) = self.drop.route(&frame, &drop, batches);
        intents.extend(dropped);
        if self.drop.carried().is_some() {
            self.panels.close();
        }
        intents.extend(self.input.route(&frame, &rest));
        if panel.is_some() && touched_outside(&area.screen, &covers, batches) {
            self.panels.close();
        }
        self.settle_view(&intents, frame.mapping());
        // Choosing a tool ends continuing to fill.
        if self.taps.iter().any(|tap| matches!(tap, Intent::Tool(_))) {
            self.drop.set_continuing(false);
        }
        intents.append(&mut self.taps);
        intents
    }

    /// A quick pinch settles the view at fit from where it showed; a contact on the canvas or a
    /// navigation ends any settling at once, so the hand always acts on what it sees.
    fn settle_view(&mut self, intents: &[Intent], mapping: ViewMapping) {
        for intent in intents {
            match intent {
                Intent::View(ViewIntent::Fit) => self.view.start(self.view.shown(mapping), SETTLE),
                Intent::View(_) | Intent::Stroke(_) | Intent::Selection(_) => self.view.stop(),
                _ => {}
            }
        }
    }
}
