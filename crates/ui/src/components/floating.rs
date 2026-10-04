//! The floating window: a picture over the canvas in a paper mat with an ink outline, with no
//! title bar (Procreate's reference window, restrained). Only while asked for, a slim bar
//! across its top holds a move grip at the left and icon buttons at the right end, and a resize
//! mark sits in its bottom-right corner. The window's owner routes the contacts on the grip,
//! the mark and the picture from the raw input; this only draws and lays out. Nothing under the
//! window takes a contact that starts on it.

use egui::{Align, Layout, Order, Rect, Sense, Stroke, UiBuilder, Vec2};

use crate::components::chrome::Source;
use crate::components::controls;
use crate::components::paper::paper;
use crate::design::{
    CORNER, ICON_SIZE, INK, Icon, PANEL, PANEL_RULE, PANEL_RULE_WIDTH, SPACE, TOUCH_TARGET,
    paint_icon,
};

/// The paper between the window's outline and its picture, in points.
pub const MAT: f32 = SPACE / 2.0;

/// Where the window's parts lie this frame, in points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloatingLayout {
    /// The whole window.
    pub window: Rect,
    /// The picture, inside the mat.
    pub picture: Rect,
    /// While the bar shows: the bar, its grip (the bar left of the buttons) and the resize
    /// mark's touch target.
    pub bar: Option<Rect>,
    pub grip: Option<Rect>,
    pub resize: Option<Rect>,
}

/// What the window gave this frame.
pub struct Floating<B, T> {
    pub layout: FloatingLayout,
    /// What the buttons returned, while the bar shows.
    pub buttons: Option<B>,
    /// What the body returned.
    pub body: T,
}

/// Shows the window `id` over `window` (points), the bar too when `bar`, grown out of `source`
/// when it comes and going back into it once it is no longer shown (D44). `add_body` paints
/// the picture into the rectangle it is given; `add_buttons` lays out right to left at the
/// bar's right end. The paper and its outline are the motion's (`paper`).
pub fn floating<B, T>(
    ctx: &egui::Context,
    id: &str,
    (window, bar, source): (Rect, bool, Source),
    add_buttons: impl FnOnce(&mut egui::Ui) -> B,
    add_body: impl FnOnce(&mut egui::Ui, Rect) -> T,
) -> Floating<B, T> {
    let area = egui::Area::new(egui::Id::new(id))
        .order(Order::Middle)
        .movable(false)
        .constrain(false)
        .fade_in(false)
        .fixed_pos(window.min);
    let (shown, _) = paper(ctx, egui::Id::new(id), source, area, |ui| {
        ui.spacing_mut().item_spacing = Vec2::ZERO;
        let (window, _) = ui.allocate_exact_size(window.size(), Sense::hover());
        let picture = window.shrink(MAT);
        // Named after `id`: the picture; while the bar shows, its grip and the resize mark too.
        controls::register(ui, &format!("{id} picture"), picture);
        let body = add_body(ui, picture);
        let mut layout = FloatingLayout {
            window,
            picture,
            bar: None,
            grip: None,
            resize: None,
        };
        let buttons = bar.then(|| show_bar(ui, &mut layout, add_buttons));
        if let (Some(grip), Some(resize)) = (layout.grip, layout.resize) {
            controls::register(ui, &format!("{id} bar"), grip);
            controls::register(ui, &format!("{id} corner"), resize);
        }
        Floating {
            layout,
            buttons,
            body,
        }
    });
    shown
}

/// The bar over the picture's top and the resize mark in its bottom-right corner.
fn show_bar<B>(
    ui: &mut egui::Ui,
    layout: &mut FloatingLayout,
    add_buttons: impl FnOnce(&mut egui::Ui) -> B,
) -> B {
    let window = layout.window;
    let bar = Rect::from_min_size(window.min, Vec2::new(window.width(), TOUCH_TARGET));
    let painter = ui.painter().clone();
    let corner = CORNER as u8;
    let top = egui::CornerRadius {
        nw: corner,
        ne: corner,
        sw: 0,
        se: 0,
    };
    painter.rect_filled(bar, top, PANEL);
    let rule = Stroke::new(PANEL_RULE_WIDTH, PANEL_RULE);
    painter.hline(bar.x_range(), bar.bottom(), rule);
    let mut row = ui.new_child(
        UiBuilder::new()
            .max_rect(bar)
            .layout(Layout::right_to_left(Align::Center)),
    );
    let buttons = add_buttons(&mut row);
    let grip = Rect::from_min_max(bar.min, egui::pos2(row.min_rect().left(), bar.bottom()));
    let icon = |rect: Rect| Rect::from_center_size(rect.center(), Vec2::splat(ICON_SIZE));
    let grip_icon = Rect::from_min_size(bar.min, Vec2::splat(TOUCH_TARGET));
    paint_icon(&painter, Icon::Move, icon(grip_icon), INK);
    let resize = Rect::from_min_size(
        window.max - Vec2::splat(TOUCH_TARGET),
        Vec2::splat(TOUCH_TARGET),
    );
    let mark = Rect::from_center_size(resize.center(), Vec2::splat(ICON_SIZE + SPACE));
    painter.rect(mark, CORNER / 2.0, PANEL, rule, egui::StrokeKind::Inside);
    paint_icon(&painter, Icon::Resize, icon(resize), INK);
    layout.bar = Some(bar);
    layout.grip = Some(grip);
    layout.resize = Some(resize);
    buttons
}
