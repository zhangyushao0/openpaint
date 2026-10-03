//! Drawing one pass of the chrome (`chrome`): every surface's shape from its growth, the
//! islands and the panels growing out of them as one chrome pass, every other surface as paper
//! under its content with its outline over it, the content clipped to the shape while it moves.

use egui::layers::ShapeIdx;
use egui::{Id, LayerId, Order, Rect, Shape, Stroke, StrokeKind};
use openpaint_render::{ChromePanel, ChromeShapes, chrome_shape};

use crate::components::chrome::{Entry, LAYER, Source, State};
use crate::components::surface;
use crate::design::motion::smoothstep;
use crate::design::{
    BACKDROP, CORNER, INK_ACTIVE, MARK_LINE, PANEL, PANEL_RULE, PANEL_RULE_WIDTH, SPACE,
};

/// Where paper still going back after its owner stopped showing it is drawn.
const RETRACTING: &str = "chrome retracting";
/// Growth over which separate paper fades in from its source and out into it, so nothing is
/// left standing where it came from.
const PAPER_FADE: f32 = 0.2;

/// One surface's shape this pass and what to do with it.
struct Drawn {
    shape: surface::Shape,
    order: Order,
    /// The chrome pass's island it grows from.
    island: Option<usize>,
    laid: Option<(LayerId, Option<ShapeIdx>)>,
    finished: bool,
    veil: Option<f32>,
    /// The paper's opacity.
    alpha: f32,
}

/// What the pass draws: the islands and every surface, bottom first.
pub(crate) struct Frame {
    islands: Vec<(Id, Rect)>,
    drawn: Vec<Drawn>,
}

impl Frame {
    pub(crate) fn of(state: &State) -> Self {
        let mut order: Vec<&Entry> = state.entries.iter().collect();
        order.sort_by_key(|entry| (entry.growth.open, entry.growth.since));
        let drawn = order
            .into_iter()
            .filter(|entry| entry.target.is_positive())
            .map(|entry| drawn(entry, &state.islands))
            .collect();
        Self {
            islands: state.islands.clone(),
            drawn,
        }
    }

    pub(crate) fn paint(&self, ctx: &egui::Context) {
        let screen = ctx.content_rect();
        for drawn in &self.drawn {
            match drawn.laid {
                Some((layer, paper)) => clip(ctx, drawn, layer, paper, screen),
                None if drawn.island.is_none() => retracting(ctx, drawn),
                None => {}
            }
        }
        if self.islands.is_empty() {
            return;
        }
        let panels = self
            .drawn
            .iter()
            .filter_map(|drawn| {
                Some(ChromePanel {
                    rect: drawn.shape.rect,
                    radius: drawn.shape.radius,
                    ring_to_rule: drawn.shape.ring_to_rule,
                    island: drawn.island?,
                })
            })
            .collect();
        let shapes = ChromeShapes {
            islands: self
                .islands
                .iter()
                .map(|(_, rect)| (*rect, CORNER))
                .collect(),
            panels,
            paper: PANEL,
            rule: PANEL_RULE,
            active: INK_ACTIVE,
            rule_width: PANEL_RULE_WIDTH,
            ring_width: MARK_LINE,
        };
        let painter = ctx.layer_painter(LayerId::new(Order::Foreground, Id::new(LAYER)));
        for reach in reaches(&shapes, screen) {
            painter.add(chrome_shape(reach, shapes.clone()));
        }
    }
}

/// Where the chrome pass runs: around each island and the panels growing out of it, with room
/// for the outline and the join; reaches that meet are one, so nothing is drawn twice.
fn reaches(shapes: &ChromeShapes, screen: Rect) -> Vec<Rect> {
    let mut reaches: Vec<Rect> = shapes
        .islands
        .iter()
        .enumerate()
        .map(|(k, (island, _))| {
            shapes
                .panels
                .iter()
                .filter(|panel| panel.island == k)
                .fold(*island, |all, panel| all.union(panel.rect))
                .expand(2.0 * SPACE)
                .intersect(screen)
        })
        .collect();
    let mut merged = true;
    while merged {
        merged = false;
        'pairs: for i in 0..reaches.len() {
            for j in i + 1..reaches.len() {
                if reaches[i].intersects(reaches[j]) {
                    let other = reaches.remove(j);
                    reaches[i] = reaches[i].union(other);
                    merged = true;
                    break 'pairs;
                }
            }
        }
    }
    reaches
}

fn drawn(entry: &Entry, islands: &[(Id, Rect)]) -> Drawn {
    let growth = &entry.growth;
    let (shape, island, veil) = match entry.source {
        Source::Button { island, button } => {
            let index = islands.iter().position(|(id, _)| *id == island);
            let bottom = index.map_or(button.bottom(), |i| islands[i].1.bottom());
            let shape = surface::from_button(growth, button.center(), entry.target, bottom);
            (shape, index, None)
        }
        Source::Rect { rect, radius, veil } => {
            let shape = surface::from_rect(growth, rect, entry.target, radius);
            (shape, None, veil.then(|| growth.grown()))
        }
        Source::Within(align) => {
            let rect = within(entry.target, align);
            let radius = (rect.height() / 2.0, CORNER);
            (
                surface::from_rect(growth, rect, entry.target, radius),
                None,
                None,
            )
        }
    };
    Drawn {
        shape,
        order: entry.order,
        island,
        laid: entry.laid,
        finished: growth.open && growth.at_rest(),
        veil,
        alpha: smoothstep(0.0, PAPER_FADE, growth.grown()),
    }
}

/// A laid-out surface: its paper under the content (unless the chrome pass draws it), its
/// outline over it, and the content clipped to its shape while it moves.
fn clip(ctx: &egui::Context, drawn: &Drawn, layer: LayerId, paper: Option<ShapeIdx>, screen: Rect) {
    let (rect, radius) = (drawn.shape.rect, drawn.shape.radius);
    ctx.graphics_mut(|graphics| {
        let list = graphics.entry(layer);
        let from = paper.map_or(0, |paper| paper.0 + 1);
        if !drawn.finished {
            let inside = rect.shrink(1.0);
            for index in from..list.next_idx().0 {
                list.mutate_shape(ShapeIdx(index), |shape| {
                    shape.clip_rect = shape.clip_rect.intersect(inside);
                });
            }
        }
        if let Some(paper) = paper {
            let fill = PANEL.gamma_multiply(drawn.alpha);
            list.set(paper, screen, Shape::rect_filled(rect, radius, fill));
            list.add(screen, outline(rect, radius, drawn.alpha));
        }
    });
}

/// Paper whose owner no longer shows it, going back into its source with no content.
fn retracting(ctx: &egui::Context, drawn: &Drawn) {
    let painter = ctx.layer_painter(LayerId::new(drawn.order, Id::new(RETRACTING)));
    if let Some(veil) = drawn.veil {
        painter.rect_filled(ctx.content_rect(), 0.0, BACKDROP.gamma_multiply(veil));
    }
    let (rect, radius) = (drawn.shape.rect, drawn.shape.radius);
    painter.rect_filled(rect, radius, PANEL.gamma_multiply(drawn.alpha));
    painter.add(outline(rect, radius, drawn.alpha));
}

fn outline(rect: Rect, radius: f32, alpha: f32) -> Shape {
    let rule = Stroke::new(PANEL_RULE_WIDTH, PANEL_RULE.gamma_multiply(alpha));
    Shape::rect_stroke(rect, radius, rule, StrokeKind::Inside)
}

/// A square of `rect`'s height at its left end, middle or right end.
fn within(rect: Rect, align: egui::Align) -> Rect {
    let side = rect.height().min(rect.width());
    let x = match align {
        egui::Align::Min => rect.left(),
        egui::Align::Center => rect.center().x - side / 2.0,
        egui::Align::Max => rect.right() - side,
    };
    Rect::from_min_size(egui::pos2(x, rect.top()), egui::Vec2::splat(side))
}
