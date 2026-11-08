//! What a mode lays into the screen the shell draws (D112): the shell knows no mode, only the
//! hooks it calls, each exactly where its part of the screen is drawn (egui lays out text into its
//! glyph atlas in the order it is drawn, so the order is part of the picture). On the canvas a
//! mode's sheet (`CanvasHook`) draws its marks over the canvas through the view, adds to the left
//! island, holds the top band's middle with its own chrome (its notices then under it), may take
//! the place of the side bar, the panels and the right island with tools of its own, and tells
//! its notices; in the gallery (`GalleryHook`) it lays a card first in the grid, a page over the
//! gallery with an island of its own, its dialogs and its notices after the gallery's own. Its
//! taps are its own: the runtime takes them from the mode and applies them after the frame's
//! painting intents; what it asks of the painting goes to the shell's taps it is lent.

use egui::{Context, Rect, Ui};
use openpaint_app::{BrushProjection, Intent, Language, SessionProjection, ViewMapping, Viewport};

use crate::color_panel::ColorPanel;
use crate::components::ScreenArea;
use crate::components::carry::Opening;
use crate::text::Text;

/// The hooks of one frame: a mode's canvas while the canvas shows its sheet, its gallery part
/// while the gallery shows; none without a mode.
#[derive(Default)]
pub struct Hooks<'a> {
    pub canvas: Option<&'a mut dyn CanvasHook>,
    pub gallery: Option<&'a mut dyn GalleryHook>,
}

impl Hooks<'_> {
    /// The same hooks again, for one more call (a pass egui runs twice, then the routing).
    pub fn reborrow(&mut self) -> Hooks<'_> {
        Hooks {
            canvas: (self.canvas.as_mut()).map(|hook| &mut **hook as &mut dyn CanvasHook),
            gallery: (self.gallery.as_mut()).map(|hook| &mut **hook as &mut dyn GalleryHook),
        }
    }
}

/// A mode's sheet on the canvas: its canvas holds the band's middle; notices go under it.
pub trait CanvasHook {
    /// Its marks over the canvas, mapped through `at` on a screen of `pixels_per_point`; after
    /// QuickShape's marks.
    fn marks(&mut self, ctx: &Context, at: &ViewMapping, pixels_per_point: f32);
    /// Its controls in the left island, after Gallery; what it asks of the painting to `taps`.
    fn left(&mut self, ui: &mut Ui, taps: &mut Vec<Intent>);
    /// Its bar in the band's middle, its tools and its card, with what the shell lends it; where
    /// they lie, which the canvas takes no contact from.
    fn chrome(&mut self, ctx: &Context, lent: &mut Chrome<'_>) -> Vec<ScreenArea>;
    /// Its own tools show: no side bar, no panels and no right island.
    fn own_tools(&self) -> bool;
    /// What it tells in the notice pill, after a failed save and the display's reset.
    fn notice(&self, language: Language) -> Option<String>;
    /// Its notice was answered with OK.
    fn dismiss(&mut self);
    /// What a fill dropped outside its clip says, in place of the selection's words.
    fn outside(&self) -> Option<Text>;
    /// A control of its took a tap this frame (the gesture trackers start again).
    fn tapped(&self) -> bool;
    /// A tap moved the view on (Next): the view glides there.
    fn glides(&self) -> bool;
}

/// What the shell lends a mode's chrome: the canvas's projection, where the canvas shows, the
/// taps the painting takes, and the colour panel's wheel.
pub struct Chrome<'a> {
    pub session: &'a SessionProjection,
    /// Where the canvas shows, once laid out.
    pub viewport: Option<Viewport>,
    pub taps: &'a mut Vec<Intent>,
    colors: &'a mut ColorPanel,
}

impl<'a> Chrome<'a> {
    pub(crate) const fn new(
        (session, viewport): (&'a SessionProjection, Option<Viewport>),
        taps: &'a mut Vec<Intent>,
        colors: &'a mut ColorPanel,
    ) -> Self {
        Self {
            session,
            viewport,
            taps,
            colors,
        }
    }

    /// The colour panel's wheel outside the panel at `brush`'s colour, keeping the hue the hand
    /// left on a grey; its changes go to the painting's taps.
    pub fn inline_wheel(&mut self, ui: &mut Ui, brush: &BrushProjection) {
        self.colors.inline_wheel(ui, brush, self.taps);
    }
}

/// How far a mode's page lies over the gallery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    /// None shows.
    None,
    /// It shows, still growing over the gallery.
    Over,
    /// It covers the gallery: nothing of the gallery needs drawing.
    Covers,
}

/// A mode's part of the gallery.
pub trait GalleryHook {
    /// While its page shows, the top band holds only the page's island, drawn here; whether it
    /// does.
    fn island(&mut self, ctx: &Context) -> bool;
    /// Its page over the gallery, if it shows; the gallery's taps to `taps`.
    fn page(&mut self, ctx: &Context, taps: &mut Vec<Intent>) -> Page;
    /// Its card in `rect`, first in the grid (none while a search is typed); where its picture
    /// lies, which a sheet going back lands in.
    fn card(&mut self, ui: &mut Ui, rect: Rect, enabled: bool, taps: &mut Vec<Intent>) -> Rect;
    /// Its dialogs, after the gallery's own.
    fn dialogs(&mut self, ctx: &Context, taps: &mut Vec<Intent>);
    /// What it tells in the gallery's notice pill once the gallery has nothing to.
    fn notice(&self) -> Option<Text>;
    /// Its notice was answered with OK.
    fn dismiss(&mut self);
    /// What it opened since it was last asked, which the canvas grows out of.
    fn opening(&mut self) -> Option<Opening>;
    /// Closes its page and dialogs as Cancel would; whether one was open.
    fn back(&mut self) -> bool;
}
