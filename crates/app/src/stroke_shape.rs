//! QuickShape in the stroke intent (S42, Procreate's): the pen held still on the glass for
//! `HOLD_MS` snaps the stroke drawn so far to the shape it resembles (`quickshape_fit.rs`), its
//! samples replaced by samples along the shape (`quickshape::along`). Still pressing, the hand
//! turns and sizes the shape; the lift commits it as any stroke, one step. After the lift the
//! shape can be edited by its handles (`EditShape`, `MoveHandle`): each drag replaces the
//! committed stroke and its copies in that same step, so one undo still goes back to before the
//! stroke. Any other intent ends editing (`end_shape`). A stroke that resembles nothing stays
//! as drawn.

use openpaint_core::{RegionMask, StrokeSample, StrokeSession, StrokeStyle};

use super::copies::Copies;
use super::{Drawing, Strokes};
use crate::error::AppError;
use crate::history::History;
use crate::quickshape::{Shape, along, norm, sub};
use crate::quickshape_fit::recognise;
use crate::symmetry::Mirror;

/// How long the pen holds still before the stroke snaps, in milliseconds (Procreate's half
/// second).
pub(crate) const HOLD_MS: i64 = 500;
/// A snapped shape follows the pen once it moved this far, in canvas pixels.
const FOLLOW: f32 = 0.25;

/// Whether and where the pen holds still, and the shape it snapped to.
#[derive(Debug, Default)]
pub(super) struct Hold {
    /// Where the pen came to rest and when, by the input clock.
    rest: Option<([f32; 2], i64)>,
    /// This rest was tried already.
    tried: bool,
    snap: Option<Snap>,
}

#[derive(Debug)]
struct Snap {
    /// The shape as it snapped, where the pen was then, and where the pen last placed it.
    first: Shape,
    held: [f32; 2],
    at: [f32; 2],
    shape: Shape,
    /// The stroke as drawn, whose pressure the shape's samples carry.
    drawn: Vec<StrokeSample>,
}

impl Hold {
    pub(super) const fn snapped(&self) -> bool {
        self.snap.is_some()
    }
}

/// The shape snapped last, from its lift until any other intent: what it committed, to edit.
#[derive(Debug)]
pub(super) struct Shaped {
    session: StrokeSession,
    style: StrokeStyle,
    clip: Option<RegionMask>,
    mirrors: Vec<Mirror>,
    shape: Shape,
    drawn: Vec<StrokeSample>,
    editing: bool,
}

impl Shaped {
    /// The shape `drawing` committed, if it snapped to one.
    pub(super) fn of(drawing: Drawing) -> Option<Self> {
        let snap = drawing.hold.snap?;
        Some(Self {
            session: drawing.session,
            style: drawing.style,
            clip: drawing.clip,
            mirrors: drawing.mirrors,
            shape: snap.shape,
            drawn: snap.drawn,
            editing: false,
        })
    }
}

/// The shape just snapped, which can be edited until anything else is done.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapeProjection {
    /// Its handles show and can be dragged.
    pub editing: bool,
    /// Where its handles are, in canvas pixels, in `MoveHandle`'s order.
    pub handles: Vec<[f32; 2]>,
}

impl Strokes {
    /// The pen is still drawing at `now_ms`: once it has rested within `still` for `HOLD_MS`,
    /// the stroke snaps to the shape it resembles, if any (once per rest).
    pub(super) fn hold(&mut self, now_ms: i64, still: f32) -> Result<(), AppError> {
        let Some(drawing) = self.drawing.as_mut() else {
            return Ok(());
        };
        let (pen, hold) = (drawing.pen, &mut drawing.hold);
        if hold.snap.is_some() {
            return Ok(());
        }
        let since = match hold.rest {
            Some((at, since)) if norm(sub(pen, at)) <= still => since,
            _ => {
                hold.rest = Some((pen, now_ms));
                hold.tried = false;
                now_ms
            }
        };
        if hold.tried || now_ms - since < HOLD_MS {
            return Ok(());
        }
        hold.tried = true;
        let samples = drawing.session.active().samples();
        let mut points: Vec<[f32; 2]> = samples
            .iter()
            .map(|sample| [sample.position().x(), sample.position().y()])
            .collect();
        points.push(pen);
        let Some(shape) = recognise(&points, still) else {
            return Ok(());
        };
        let drawn = samples.to_vec();
        drawing.session.reshape(&along(&shape.outline(), &drawn)?)?;
        drawing.hold.snap = Some(Snap {
            first: shape.clone(),
            held: pen,
            at: pen,
            shape,
            drawn,
        });
        self.show_copies()
    }

    /// The pen moved on after the stroke snapped: the shape turns and sizes with it.
    pub(super) fn follow_snapped(&mut self) -> Result<(), AppError> {
        let Some(drawing) = self.drawing.as_mut() else {
            return Ok(());
        };
        let pen = drawing.pen;
        let Some(snap) = drawing.hold.snap.as_mut() else {
            return Ok(());
        };
        if norm(sub(pen, snap.at)) < FOLLOW {
            return Ok(());
        }
        let shape = snap.first.dragged(snap.held, pen);
        drawing
            .session
            .reshape(&along(&shape.outline(), &snap.drawn)?)?;
        (snap.shape, snap.at) = (shape, pen);
        self.show_copies()
    }

    /// Shows the shape's handles (`on`), or ends editing it.
    pub(super) fn edit_shape(&mut self, on: bool) {
        match self.shaped.as_mut() {
            Some(shaped) if on => shaped.editing = true,
            _ => self.shaped = None,
        }
    }

    /// Drags handle `handle` of the shape being edited to `to`: the stroke and its copies are
    /// committed anew in place of the step they made.
    pub(super) fn move_handle(
        &mut self,
        history: &mut History,
        handle: usize,
        to: [f32; 2],
    ) -> Result<(), AppError> {
        let Some(shaped) = self.shaped.as_mut().filter(|shaped| shaped.editing) else {
            return Ok(());
        };
        let shape = shaped.shape.moved(handle, to);
        shaped
            .session
            .reshape(&along(&shape.outline(), &shaped.drawn)?)?;
        let copies = Copies {
            style: &shaped.style,
            clip: shaped.clip.as_ref(),
            mirrors: &shaped.mirrors,
        };
        history.amend(copies.committed(&shaped.session)?);
        shaped.shape = shape;
        self.reshown = true;
        Ok(())
    }

    /// Any other intent is done with the shape snapped last.
    pub(crate) fn end_shape(&mut self) {
        self.shaped = None;
    }

    /// The shape snapped last, while it can be edited.
    pub(crate) fn shape(&self) -> Option<ShapeProjection> {
        self.shaped.as_ref().map(|shaped| ShapeProjection {
            editing: shaped.editing,
            handles: shaped.shape.handles(),
        })
    }
}
