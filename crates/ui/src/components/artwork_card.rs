//! The artwork card of the gallery: the artwork's picture, and under it its title with the
//! duplicate and delete buttons. Tapping the picture opens the artwork, tapping the title
//! renames it. Pressed, the picture darkens under the hand.

use egui::{Align, Color32, Layout, Rect, Sense, Stroke, TextStyle, UiBuilder, Vec2};

use crate::components::icon_button;
use crate::design::{
    CARD_PICTURE, CHROME, Icon, ON_CHROME, ON_CHROME_DISABLED, ON_CHROME_SECONDARY, OUTLINE, SPACE,
    TOUCH_TARGET,
};

/// What one card shows.
pub struct ArtworkCard<'a> {
    /// Stays the same for one artwork from frame to frame.
    pub id: egui::Id,
    pub title: &'a str,
    /// The thumbnail, `None` until the artwork has been left once since it was made.
    pub picture: Option<&'a egui::TextureHandle>,
    pub enabled: bool,
}

/// What was tapped on a card this frame.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CardTaps {
    pub open: bool,
    pub rename: bool,
    pub duplicate: bool,
    pub delete: bool,
}

/// The card's size in points.
#[must_use]
pub fn card_size() -> Vec2 {
    CARD_PICTURE + Vec2::new(0.0, TOUCH_TARGET)
}

pub fn artwork_card(ui: &mut egui::Ui, card: &ArtworkCard<'_>) -> CardTaps {
    let sense = if card.enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, _) = ui.allocate_exact_size(card_size(), Sense::hover());
    let picture = Rect::from_min_size(rect.min, CARD_PICTURE);
    let open = ui.interact(picture, card.id, sense);
    paint_picture(ui, card, picture, open.is_pointer_button_down_on());
    let row = Rect::from_min_max(picture.left_bottom(), rect.max);
    let mut taps = CardTaps {
        open: open.clicked(),
        ..CardTaps::default()
    };
    let mut buttons = ui.new_child(
        UiBuilder::new()
            .id_salt(card.id)
            .max_rect(row)
            .layout(Layout::right_to_left(Align::Center)),
    );
    buttons.spacing_mut().item_spacing = Vec2::ZERO;
    taps.delete = icon_button(&mut buttons, Icon::Delete, "Delete artwork", card.enabled).clicked();
    taps.duplicate = icon_button(
        &mut buttons,
        Icon::Duplicate,
        "Duplicate artwork",
        card.enabled,
    )
    .clicked();
    let title = Rect::from_min_max(
        row.min,
        egui::pos2(row.max.x - 2.0 * TOUCH_TARGET, row.max.y),
    );
    let rename = ui.interact(title, open.id.with("title"), sense);
    taps.rename = rename.clicked();
    paint_title(ui, card, title);
    taps
}

/// The thumbnail fitted into the picture box, centred on its bottom edge, with an outline; a dark face with "No
/// preview" when there is none.
fn paint_picture(ui: &egui::Ui, card: &ArtworkCard<'_>, rect: Rect, pressed: bool) {
    let painter = ui.painter();
    let Some(texture) = card.picture else {
        painter.rect_filled(rect, 0.0, CHROME);
        painter.rect_stroke(
            rect,
            0.0,
            Stroke::new(1.0, OUTLINE),
            egui::StrokeKind::Inside,
        );
        let font = TextStyle::Small.resolve(ui.style());
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "No preview",
            font,
            ON_CHROME_SECONDARY,
        );
        return;
    };
    let size = texture.size_vec2();
    let scale = (rect.width() / size.x).min(rect.height() / size.y);
    // On the bottom of the box, so every title sits just under its picture.
    let shown = Rect::from_center_size(rect.center_bottom(), size * scale)
        .translate(Vec2::new(0.0, -size.y * scale / 2.0));
    let uv = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
    let tint = if pressed {
        Color32::from_gray(170)
    } else {
        Color32::WHITE
    };
    painter.image(texture.id(), shown, uv, tint);
    painter.rect_stroke(
        shown,
        0.0,
        Stroke::new(1.0, OUTLINE),
        egui::StrokeKind::Outside,
    );
}

/// The title on one line, cut with an ellipsis when it is too long.
fn paint_title(ui: &egui::Ui, card: &ArtworkCard<'_>, rect: Rect) {
    let color = if card.enabled {
        ON_CHROME
    } else {
        ON_CHROME_DISABLED
    };
    let mut job = egui::text::LayoutJob::single_section(
        card.title.to_owned(),
        egui::TextFormat::simple(TextStyle::Body.resolve(ui.style()), color),
    );
    job.wrap = egui::text::TextWrapping::truncate_at_width(rect.width() - SPACE);
    let galley = ui.painter().layout_job(job);
    let at = egui::pos2(rect.min.x, rect.center().y - galley.size().y / 2.0);
    ui.painter().galley(at, galley, color);
}
