//! Selection (Procreate's Selection tool, with draweave's behaviour): while the tool is on, the
//! pen draws selections instead of strokes. Freehand: drag a lasso, or tap points for straight
//! segments; a contact touching down (or lifting) on the path's first point closes it, and so
//! does leaving the tool. Rectangle: drag from one corner to the other. Each closed shape adds
//! to the selection (draweave's default). Invert and Deselect act on the whole selection.
//!
//! The selection is tool state, not artwork (DECISIONS D42): it never enters the artwork, its
//! history or the store. While the tool is on, undo and redo walk its own steps (draweave's
//! draft cursor); off, they walk the artwork's history and the selection stays. A stroke or a
//! fill made while a selection exists changes only its pixels, and keeps that clip for good.

use std::sync::Arc;

use openpaint_core::{CanvasSize, RegionMask};

use crate::input::{Contact, ContactPhase, Device};

/// The smallest pen move, in canvas pixels, that adds a point to a freehand path.
const PATH_STEP: f32 = 1.0;
/// Most points one freehand path may hold.
const MAX_PATH_POINTS: usize = 65_536;

/// What the pen draws while the Selection tool is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectionShape {
    #[default]
    Freehand,
    Rectangle,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SelectionIntent {
    /// The Selection tool on (the pen draws selections) or off (the pen paints again, inside
    /// the selection). Turning it off closes an open freehand path.
    Tool(bool),
    Shape(SelectionShape),
    /// A pen contact while the tool is on, in canvas pixels. A freehand path closes when a
    /// contact touches down or lifts within `close` canvas pixels of its first point.
    Contact {
        contact: Contact,
        close: f32,
    },
    /// Closes the open freehand path.
    Close,
    /// Selects what was not selected; with nothing selected, the whole canvas.
    Invert,
    /// Nothing selected: the pen paints everywhere again.
    Deselect,
}

/// What the UI shows of the selection.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SelectionProjection {
    /// The Selection tool is on.
    pub tool: bool,
    pub shape: SelectionShape,
    /// The pixels selected now.
    pub mask: Option<RegionMask>,
    /// The freehand path being drawn, or the rectangle's two corners while it is dragged, in
    /// canvas pixels.
    pub path: Arc<[[f32; 2]]>,
    /// The open freehand path holds enough points to close.
    pub closable: bool,
}

#[derive(Debug)]
pub(crate) struct Selection {
    tool: bool,
    shape: SelectionShape,
    /// Every selection since the session began (`None`: nothing selected); `at` is current.
    steps: Vec<Option<RegionMask>>,
    at: usize,
    path: Vec<[f32; 2]>,
    /// Where the path's segment being drawn now began, so a cancel drops only it.
    segment: usize,
    /// The contact drawing the path.
    drawing: Option<i32>,
}

impl Default for Selection {
    fn default() -> Self {
        Self {
            tool: false,
            shape: SelectionShape::Freehand,
            steps: vec![None],
            at: 0,
            path: Vec::new(),
            segment: 0,
            drawing: None,
        }
    }
}

impl Selection {
    /// The pixels selected now.
    pub(crate) fn mask(&self) -> Option<&RegionMask> {
        self.steps[self.at].as_ref()
    }

    /// True while the Selection tool is on.
    pub(crate) const fn tool(&self) -> bool {
        self.tool
    }

    pub(crate) fn handle(&mut self, canvas: CanvasSize, intent: SelectionIntent) {
        match intent {
            SelectionIntent::Tool(on) => {
                if !on {
                    self.close(canvas);
                }
                self.tool = on;
            }
            SelectionIntent::Shape(shape) => {
                self.forget_path();
                self.shape = shape;
            }
            SelectionIntent::Contact { contact, close } if self.tool => {
                self.contact(canvas, &contact, close);
            }
            SelectionIntent::Contact { .. } => {}
            SelectionIntent::Close => self.close(canvas),
            SelectionIntent::Invert => {
                self.forget_path();
                let inverted = match self.mask() {
                    Some(mask) => mask.inverted(canvas),
                    None => Some(RegionMask::whole(canvas)),
                };
                self.push(inverted);
            }
            SelectionIntent::Deselect => {
                self.forget_path();
                if self.mask().is_some() {
                    self.push(None);
                }
            }
        }
    }

    fn contact(&mut self, canvas: CanvasSize, contact: &Contact, close: f32) {
        if contact.device != Device::Pen {
            return;
        }
        let points: Vec<[f32; 2]> = contact.samples.iter().map(|s| s.position).collect();
        let Some(&last) = points.last() else {
            return;
        };
        match contact.phase {
            ContactPhase::Down => self.down(canvas, contact.id, &points, close),
            ContactPhase::Move if self.drawing == Some(contact.id) => self.extend(&points),
            ContactPhase::Up if self.drawing == Some(contact.id) => {
                self.extend(&points);
                self.drawing = None;
                let closes =
                    self.shape == SelectionShape::Rectangle || self.near_start(last, close);
                if closes {
                    self.close(canvas);
                }
            }
            ContactPhase::Cancel if self.drawing == Some(contact.id) => {
                self.drawing = None;
                self.path.truncate(self.segment);
            }
            _ => {}
        }
    }

    fn down(&mut self, canvas: CanvasSize, id: i32, points: &[[f32; 2]], close: f32) {
        let first = points[0];
        if self.shape == SelectionShape::Rectangle {
            self.path = vec![first, first];
        } else if self.near_start(first, close) {
            // A tap on the first point closes the path; the rest of this contact is no path.
            self.close(canvas);
            return;
        } else {
            self.segment = self.path.len();
            self.path.push(first);
        }
        self.drawing = Some(id);
        self.extend(&points[1..]);
    }

    fn extend(&mut self, points: &[[f32; 2]]) {
        for &point in points {
            match self.shape {
                SelectionShape::Rectangle => {
                    if let Some(corner) = self.path.get_mut(1) {
                        *corner = point;
                    }
                }
                SelectionShape::Freehand => {
                    let moved = self.path.last().is_none_or(|last| {
                        (point[0] - last[0]).hypot(point[1] - last[1]) >= PATH_STEP
                    });
                    if moved && self.path.len() < MAX_PATH_POINTS {
                        self.path.push(point);
                    }
                }
            }
        }
    }

    /// True when the open freehand path can close at `point`.
    fn near_start(&self, point: [f32; 2], close: f32) -> bool {
        self.path.len() >= 3
            && self.path[0][0].is_finite()
            && (point[0] - self.path[0][0]).hypot(point[1] - self.path[0][1]) <= close
    }

    /// Closes the path being drawn and adds its pixels to the selection; a shape holding no
    /// pixel changes nothing.
    fn close(&mut self, canvas: CanvasSize) {
        let path = std::mem::take(&mut self.path);
        (self.segment, self.drawing) = (0, None);
        let shape = match (self.shape, path.as_slice()) {
            (SelectionShape::Rectangle, [a, b]) => vec![*a, [b[0], a[1]], *b, [a[0], b[1]]],
            (SelectionShape::Freehand, points) if points.len() >= 3 => points.to_vec(),
            _ => return,
        };
        let Some(added) = RegionMask::polygon(canvas, &shape) else {
            return;
        };
        let next = match self.mask() {
            Some(mask) => mask.union(&added),
            None => added,
        };
        self.push(Some(next));
    }

    fn forget_path(&mut self) {
        self.path.clear();
        (self.segment, self.drawing) = (0, None);
    }

    fn push(&mut self, mask: Option<RegionMask>) {
        self.steps.truncate(self.at + 1);
        self.steps.push(mask);
        self.at += 1;
    }

    /// Undo while the tool is on: an open path goes first, then the selection before.
    pub(crate) const fn can_undo(&self) -> bool {
        !self.path.is_empty() || self.at > 0
    }

    pub(crate) const fn can_redo(&self) -> bool {
        self.path.is_empty() && self.at + 1 < self.steps.len()
    }

    pub(crate) fn undo(&mut self) {
        if self.path.is_empty() {
            self.at = self.at.saturating_sub(1);
        } else {
            self.forget_path();
        }
    }

    pub(crate) fn redo(&mut self) {
        if self.can_redo() {
            self.at += 1;
        }
    }

    pub(crate) fn projection(&self) -> SelectionProjection {
        SelectionProjection {
            tool: self.tool,
            shape: self.shape,
            mask: self.mask().cloned(),
            path: Arc::from(self.path.as_slice()),
            closable: self.shape == SelectionShape::Freehand && self.path.len() >= 3,
        }
    }
}
