//! What every geometric mark over a practice sheet is drawn with (S97, S98): canvas points to the
//! screen (`At`), lines, dashes, rings and dots, the horizon labelled, and what is off (`Off`):
//! a ring, an edge run on dashed red past its point's ring, a true vertical. Whatever runs beyond
//! a frame is cut at the sheet's edge (S98: on the tablet a box's run-ons and construction ran on
//! over the grey chrome, and read as a fault), a ring off the sheet left out; a kind that keeps
//! its original beside the copy (an object's picture) keeps every mark off that frame too, as
//! S88 kept a box's (`At::keeping_off`).

use egui::{Align2, FontId, Pos2, Shape, Stroke};
use openpaint_practice::{Frame, Point, outside, within};

use openpaint_ui::design::{DIVIDER, PANEL};

use crate::palette::PRINTED;
use crate::text::{self as t, Words};

/// A ring's radius round a point or a middle, in points; a mark's line and the paper's halo under
/// it; a faint line's.
pub(crate) const RING: f32 = 7.0;
pub(crate) const MARK: f32 = 2.0;
pub(crate) const HALO: f32 = 3.0;
pub(crate) const FAINT: f32 = 1.25;
/// Dashes and gaps, in points.
pub(crate) const DASH: f32 = 7.0;
pub(crate) const GAP: f32 = 5.0;
/// How strongly the answer's red lies over the copy.
pub(crate) const SEE_THROUGH: f32 = 0.6;
/// The horizon's label's size in canvas pixels of a frame.
const LABEL: f32 = 11.0;

/// Canvas points to the screen, on a sheet of `sheet` canvas pixels; lines are cut at its edge
/// and kept off the rectangle `kept_off` (canvas pixels: an original's frame) when there is one.
#[derive(Clone, Copy)]
pub(crate) struct At<'a> {
    map: &'a dyn Fn([f64; 2]) -> Pos2,
    sheet: [f64; 2],
    kept_off: Option<[Point; 2]>,
}

impl<'a> At<'a> {
    pub(crate) fn new(map: &'a dyn Fn([f64; 2]) -> Pos2, sheet: [u32; 2]) -> Self {
        Self {
            map,
            sheet: sheet.map(f64::from),
            kept_off: None,
        }
    }

    /// Lines kept off `frame` as well: they stop at its side and run on past it.
    pub(crate) fn keeping_off(self, frame: &Frame) -> Self {
        let [x, y] = frame.origin;
        let [w, h] = frame.size;
        Self {
            kept_off: Some([[x, y], [x + w, y + h]]),
            ..self
        }
    }

    /// The canvas point `p` on the screen.
    pub(crate) fn of(&self, p: Point) -> Pos2 {
        (self.map)(p)
    }

    /// Whether the canvas point `p` lies on the sheet, off the rectangle kept off.
    pub(crate) fn holds(&self, p: Point) -> bool {
        let on = (0..2).all(|k| (0.0..=self.sheet[k]).contains(&p[k]));
        let kept = self
            .kept_off
            .is_some_and(|[low, high]| (0..2).all(|k| (low[k]..=high[k]).contains(&p[k])));
        on && !kept
    }

    /// The line through the canvas points `points` cut at the sheet's edge (and kept off the
    /// rectangle kept off), each part on the screen.
    pub(crate) fn parts(&self, points: &[Point]) -> Vec<Vec<Pos2>> {
        let on_sheet = within(points, self.sheet);
        let kept: Vec<Vec<Point>> = match self.kept_off {
            Some([low, high]) => (on_sheet.iter())
                .flat_map(|part| outside(part, low, high))
                .collect(),
            None => on_sheet,
        };
        (kept.into_iter())
            .map(|part| part.into_iter().map(|p| self.of(p)).collect())
            .collect()
    }
}

/// The line from `a` to `b` (frame units) of `frame`, cut at the sheet's edge.
pub(crate) fn segment(
    painter: &egui::Painter,
    frame: &Frame,
    [a, b]: [Point; 2],
    at: At<'_>,
    stroke: Stroke,
) {
    line(painter, frame, &[a, b], at, stroke);
}

/// The line through `points` (frame units) of `frame`, cut at the sheet's edge.
pub(crate) fn line(
    painter: &egui::Painter,
    frame: &Frame,
    points: &[Point],
    at: At<'_>,
    stroke: Stroke,
) {
    let canvas: Vec<Point> = points.iter().map(|p| frame.to_canvas(*p)).collect();
    for part in at.parts(&canvas) {
        painter.add(Shape::line(part, stroke));
    }
}

/// A ring round `p` (frame units of `frame`) over the paper's halo, in `colour`; none off the
/// sheet.
pub(crate) fn ring(
    painter: &egui::Painter,
    (frame, p): (&Frame, Point),
    at: At<'_>,
    colour: egui::Color32,
) {
    let canvas = frame.to_canvas(p);
    if !at.holds(canvas) {
        return;
    }
    let centre = at.of(canvas);
    painter.circle_stroke(centre, RING, Stroke::new(MARK + HALO, PANEL));
    painter.circle_stroke(centre, RING, Stroke::new(MARK, colour));
}

/// A faint line's stroke: the way to see an answer, laid on request (Construction).
pub(crate) fn faint() -> Stroke {
    Stroke::new(FAINT, DIVIDER)
}

/// The horizon at `height` across `frame`, printed grey and labelled small at its left; `scale`
/// points to a canvas pixel.
pub(crate) fn horizon(
    painter: &egui::Painter,
    (frame, height): (&Frame, f64),
    at: At<'_>,
    scale: f32,
) {
    let ends = [[0.0, height], [1.0, height]];
    segment(painter, frame, ends, at, Stroke::new(FAINT, PRINTED));
    let label = Words::from(t::HORIZON).shown(painter.ctx());
    let place = at.of(frame.to_canvas([0.01, height - 0.012]));
    let font = FontId::proportional((LABEL * scale).max(6.0));
    painter.text(place, Align2::LEFT_BOTTOM, label, font, PRINTED);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_beyond_the_sheet_stops_at_its_edge_and_a_ring_off_it_is_left_out() {
        let map = |p: [f64; 2]| Pos2::new(p[0] as f32, p[1] as f32);
        let at = At::new(&map, [1000, 600]);
        // A run-on from a frame down past the sheet's bottom, towards a far vertical point.
        let parts = at.parts(&[[500.0, 300.0], [700.0, 1500.0]]);
        assert_eq!(parts.len(), 1);
        let end = parts[0][parts[0].len() - 1];
        assert!(
            (end.y - 600.0).abs() < 1e-3 && (end.x - 550.0).abs() < 1e-3,
            "{end:?}"
        );
        // The horizon run on across: from edge to edge.
        let across = at.parts(&[[-2000.0, 200.0], [3000.0, 200.0]]);
        assert_eq!(
            across[0],
            vec![Pos2::new(0.0, 200.0), Pos2::new(1000.0, 200.0)]
        );
        assert!(at.holds([999.0, 599.0]) && !at.holds([1200.0, 300.0]));
        // Kept off an original's frame: cut where it crosses it, on past it; no ring in it.
        let frame = Frame::square([100.0, 100.0], 200.0);
        let kept = at.keeping_off(&frame);
        let parts = kept.parts(&[[600.0, 200.0], [0.0, 200.0]]);
        let xs: Vec<Vec<f32>> = (parts.iter())
            .map(|p| p.iter().map(|q| q.x).collect())
            .collect();
        assert_eq!(xs, vec![vec![600.0, 300.0], vec![100.0, 0.0]]);
        assert!(!kept.holds([200.0, 200.0]) && kept.holds([50.0, 200.0]));
    }
}
