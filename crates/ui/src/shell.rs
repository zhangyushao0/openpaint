//! The screen: the canvas (or the home) on the whole screen, with the controls floating over
//! it as islands (D40). On the canvas: Gallery and Actions in the left island; Brush, Eraser,
//! Layers and the colour in the right one; a notice (a failed save or open, an export, the
//! reference image) in a pill left of it; the side bar (size, eyedropper, opacity, undo, redo)
//! over the left edge; the reference window where the hand left it; the Actions, colour, layer
//! and brush panels under the islands, one at a time. While the fingers move the view, a quiet
//! pill in the top band tells its zoom and turn, then fades. The pen hovering over the canvas
//! shows the brush's outline, the eraser shows it while it erases, and a contact picking a
//! colour shows the loupe (`pen_marks`). On the home: a mode's column beside the studio's cards,
//! "+" and the search in the top band (`home/`). What comes and goes moves (D44): panels, pills, dialogs, the loupe and the reference window
//! grow out of where they come from; an artwork grows out of its card and goes back into it; a
//! quick pinch settles the view at fit. A mode's screens come in through the frame's hooks
//! (`hooks.rs`), each called where its part of the screen lies.

use openpaint_app::{
    GalleryIntent, Intent, Language, ScreenProjection, SessionProjection, ViewMapping,
};
use openpaint_render::BrushCursor;

use crate::adjust_panel::AdjustPanel;
use crate::canvas_input::CanvasInput;
use crate::color_drop::ColorDrop;
use crate::components::carry::{self, Opening, sheet_at};
use crate::components::{
    CanvasArea, ControlMap, ScreenArea, ViewSettle, canvas_frame, chrome, controls, moving,
};
use crate::copy_bar::CopyBar;
use crate::design::motion::OPEN;
use crate::drop_bar;
use crate::home::HomeScreen;
use crate::hooks::{CanvasHook, Chrome, HomeHook, Hooks};
use crate::panels::Panels;
use crate::pen_marks;
use crate::reference_window::ReferenceWindow;
use crate::selection_bar::{self, SelectionBar};
use crate::shape_bar::{self, ShapeBar};
use crate::sheet::{self, Showing};
use crate::side_bar::SideBar;
use crate::text::{self, Names};
use crate::transform_bar::TransformBar;

#[path = "shell_islands.rs"]
mod islands;
#[path = "shell_route.rs"]
mod route;
#[path = "shell_shown.rs"]
mod shown;
pub(crate) use shown::Shown;

/// The screen's UI-local state between frames.
#[derive(Debug, Default)]
pub struct Shell {
    /// Where the canvas shows and fits, or the home.
    canvas: Option<CanvasArea>,
    showing: Showing,
    shown: Option<Shown>,
    pixels_per_point: f32,
    /// The input clock for this frame (`set_now`).
    pub(crate) now_ms: i64,
    pub(crate) input: CanvasInput,
    /// This frame's contacts (`set_contacts`), routed ahead for the view the pass lays with.
    contacts: Vec<openpaint_app::InputBatch>,
    panels: Panels,
    side: SideBar,
    selection: SelectionBar,
    transform: TransformBar,
    adjust: AdjustPanel,
    copy: CopyBar,
    /// QuickShape's pill.
    shape: ShapeBar,
    /// Where a mode's chrome lies on screen, from the last pass (`CanvasHook::chrome`).
    mode_areas: Vec<ScreenArea>,
    /// The shape's handles while it is edited and the guide's centre while it is on, in canvas
    /// pixels, from the last pass.
    handles: Vec<[f32; 2]>,
    guide_centre: Option<[f32; 2]>,
    reference: ReferenceWindow,
    pub(crate) drop: ColorDrop,
    /// Where the fill bar lies, from the last pass.
    drop_bar: Option<ScreenArea>,
    /// Where the top islands and the notice pill lie, from the last pass.
    islands: Vec<ScreenArea>,
    home: HomeScreen,
    /// Intents from taps on the chrome since the last `route` (a mode keeps its own).
    taps: Vec<Intent>,
    /// The view the screen shows while it settles, and the artwork on its way from its card.
    view: ViewSettle,
    opening: Option<Opening>,
    /// Which artwork the canvas came from, where it showed last (points; a mode's sheet the part
    /// shown, with its canvas pixels: `sheet::left_from`) and the picture it left with (S144).
    came_from: Option<openpaint_app::ProjectId>,
    shown_at: Option<(egui::Rect, Option<[u32; 4]>)>,
    carried: Option<openpaint_app::Thumbnail>,
    /// Something moved in the last pass.
    moving: bool,
    /// The controls of the last pass.
    controls: ControlMap,
    /// The GPU device was made again: the notice says so until OK or the next stroke.
    reset_notice: bool,
    /// The language every word shows in, and the names a mode gives.
    language: Language,
    names: Names,
}

impl Shell {
    /// Builds one frame inside the root `ui` from `projection` and the mode's `hooks`, remembers
    /// where the canvas is and collects the buttons tapped. egui hands input to the first pass of
    /// a frame only, so a tap is collected once however many passes run.
    pub fn show(&mut self, ui: &mut egui::Ui, projection: &ScreenProjection, hooks: Hooks<'_>) {
        let ctx = ui.ctx().clone();
        chrome::begin(&ctx);
        controls::begin(&ctx);
        text::set_language(&ctx, self.language);
        text::set_names(&ctx, self.names);
        let Hooks { canvas, mut home } = hooks;
        let showing = Showing::of(projection);
        if showing != self.showing {
            self.switch(&ctx, showing, home.as_deref_mut());
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
            ScreenProjection::Canvas(session) => self.canvas_screen(ui, session, canvas),
            ScreenProjection::Gallery(projection) => {
                (self.home).show(ui, projection, (home, &mut self.taps))
            }
        };
        self.canvas = Some(CanvasArea::from_points(rect, self.pixels_per_point));
        chrome::finish(&ctx);
        self.moving = moving(&ctx);
        self.controls = controls::take(&ctx);
    }

    /// Every control the last pass laid out, by name, in screen pixels.
    #[must_use]
    pub const fn controls(&self) -> &ControlMap {
        &self.controls
    }

    /// A new screen starts with its panels and dialogs closed (a mode's on the home too,
    /// `home`). An artwork opened from its card grows out of it; the canvas left goes back
    /// into its card.
    fn switch(
        &mut self,
        ctx: &egui::Context,
        showing: Showing,
        home: Option<&mut (dyn HomeHook + '_)>,
    ) {
        chrome::forget(ctx);
        self.panels.close();
        self.panels.layers.forget_pictures();
        self.side.hide();
        self.copy.close();
        self.input = CanvasInput::default();
        self.drop.reset();
        self.drop_bar = None;
        self.view.stop();
        match (self.showing, showing) {
            (Showing::Gallery, _) => {
                self.opening = self.home.take_opening();
                self.came_from = self.opening.as_ref().and_then(|opening| opening.id);
            }
            (Showing::Canvas, Showing::Gallery) => {
                // The last canvas pass's: a mode's sheet goes back into the mode's place (or the
                // page it was opened from) with the picture it left with.
                let card = self.shown.is_some_and(|shown| shown.mode);
                let picture = self
                    .carried
                    .take()
                    .map(|picture| carry::texture(ctx, &picture));
                let into = (home.as_deref()).and_then(HomeHook::landing);
                if let Some((from, _)) = self.shown_at.take() {
                    self.home
                        .arrive(from, (self.came_from, card, into), picture);
                }
            }
            _ => {}
        }
        if showing != Showing::Starting {
            self.home.back(home);
        }
        self.showing = showing;
    }

    fn canvas_screen(
        &mut self,
        ui: &mut egui::Ui,
        projection: &SessionProjection,
        mut hook: Option<&mut (dyn CanvasHook + '_)>,
    ) -> egui::Rect {
        let brush = projection.brush();
        self.shown = Shown::of(projection, hook.is_some());
        let rect = canvas_frame(ui);
        let ctx = ui.ctx().clone();
        let area = CanvasArea::from_points(rect, self.pixels_per_point);
        self.arrived();
        if let Some(mapping) = self.foreseen(area) {
            self.view.advance(&ctx);
            let (shown, ppp) = (self.view.shown(mapping), self.pixels_per_point);
            let whole = sheet::show_sheet(&ctx, rect, self.shown, &shown, ppp);
            self.shown_at = sheet::left_from(whole, (rect, self.shown), &shown, ppp);
            selection_bar::show_path(&ctx, projection, &shown, self.pixels_per_point);
            self.transform.show_frame(&ctx, projection, &shown, ppp);
            shape_bar::show_marks(&ctx, projection, &shown, ppp);
            if let Some(hook) = hook.as_deref_mut() {
                hook.marks(&ctx, &shown, ppp);
            }
        }
        let editing = projection.shape().filter(|shape| shape.editing);
        self.handles = editing
            .map(|shape| shape.handles.clone())
            .unwrap_or_default();
        let guide = projection.guide();
        self.guide_centre = guide.on.then_some(guide.centre);
        self.shape.show(&ctx, projection, &mut self.taps);
        sheet::show_view(self, &ctx, &area);
        self.top_islands(&ctx, projection, hook.as_deref_mut());
        self.mode_chrome(&ctx, (projection, &area), hook.as_deref_mut());
        self.selection.show(&ctx, projection, &mut self.taps);
        self.transform.show(&ctx, projection, &mut self.taps);
        if hook.as_deref().is_some_and(|hook| hook.own_tools()) {
            // The mode's own tools: no side bar, and no panel without its button.
            self.side.hide();
            self.panels.close();
        } else {
            let scale = self.scale(&area);
            self.side.show(&ctx, projection, scale, &mut self.taps);
        }
        let opener = self.panels.actions.button_rect();
        self.reference
            .show(&ctx, projection, (self.now_ms, opener), &mut self.taps);
        let sheet = hook.is_some();
        self.panels.show(&ctx, (projection, sheet), &mut self.taps);
        self.adjust.show(&ctx, projection, &mut self.taps);
        if self.panels.actions.take_copy_menu() {
            self.copy.open();
        }
        self.copy.show(&ctx, projection, &mut self.taps);
        let shown = (projection, hook.is_some());
        self.drop_bar = drop_bar::show(&mut self.drop, &ctx, shown, self.pixels_per_point);
        let picking_at = self.reference.picking_at().or(self.input.picking_at());
        pen_marks::show_loupe(&ctx, picking_at, brush, self.pixels_per_point);
        rect
    }

    /// A mode's bar, tools and card, lent the canvas's projection, its room in `area`, the
    /// painting's taps and the colour wheel; where they lie, for routing.
    fn mode_chrome(
        &mut self,
        ctx: &egui::Context,
        (projection, area): (&SessionProjection, &CanvasArea),
        hook: Option<&mut (dyn CanvasHook + '_)>,
    ) {
        self.mode_areas.clear();
        if let Some(hook) = hook {
            let viewport = Some(area.viewport(self.pixels_per_point));
            let colors = &mut self.panels.colors;
            let mut lent = Chrome::new((projection, viewport), &mut self.taps, colors);
            self.mode_areas = hook.chrome(ctx, &mut lent);
        }
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

    /// The input clock now (the time input samples carry) for the next frame; call before `show`.
    pub fn set_now(&mut self, now_ms: i64) {
        self.now_ms = now_ms;
    }

    /// The contacts the next frame routes; call before `show`, so what it lays over the canvas
    /// lies where they move it (`foreseen`).
    pub fn set_contacts(&mut self, batches: &[openpaint_app::InputBatch]) {
        self.contacts = batches.to_vec();
    }

    /// The device's offset from UTC in minutes, for the dates the home shows.
    pub fn set_utc_offset(&mut self, minutes: i32) {
        self.home.utc_offset = minutes;
    }

    /// The app's version and storage format, which Actions' last line shows.
    pub fn set_version(&mut self, version: &str, format: u32) {
        self.panels.actions.version = Some((version.to_owned(), format));
    }

    /// The language the interface shows from the next pass on (D100).
    pub fn set_language(&mut self, language: Language) {
        self.language = language;
    }

    /// The names a mode gives, which show in the language from the next pass on (D112).
    pub fn set_names(&mut self, names: Names) {
        self.names = names;
    }

    /// The pen's outline in screen pixels: the current brush at full size around where the pen
    /// hovers over open canvas, or, while the pen erases there, `erasing` (the eraser's tip:
    /// centre and radius in canvas pixels); `None` over controls, during multi-touch, while
    /// picking, while the brush draws and off the canvas screen.
    #[must_use]
    pub fn cursor(&self, erasing: Option<([f32; 2], f32)>) -> Option<BrushCursor> {
        // While the Selection or the Transform tool is on, or an adjustment, the pen draws no
        // paint: no outline.
        let painting = |s: &Shown| !s.selecting && !s.transforming && !s.adjusting;
        let (shown, area) = (self.shown.filter(painting)?, self.canvas?);
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

    /// The system's back button: from the canvas it goes to the home; on the home it
    /// closes an open dialog (a mode's through `hooks`). Returns whether it was used; otherwise
    /// the system's back applies.
    pub fn back(&mut self, projection: &ScreenProjection, hooks: Hooks<'_>) -> bool {
        match projection {
            ScreenProjection::Canvas(_) if self.panels.layers.cancel_rename() => true,
            ScreenProjection::Canvas(_) if self.panels.brushes.back() => true,
            ScreenProjection::Canvas(_) => {
                self.taps.push(Intent::Gallery(GalleryIntent::Show));
                true
            }
            ScreenProjection::Gallery(_) => self.home.back(hooks.home),
            ScreenProjection::Starting => false,
        }
    }

    /// Where the canvas shows and fits (the whole screen, fitting under the top band), once a
    /// frame has been laid out.
    #[must_use]
    pub fn canvas_area(&self) -> Option<CanvasArea> {
        self.canvas
    }
}
