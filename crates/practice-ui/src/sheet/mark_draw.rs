//! A practice mark drawn over its pair of frames (S109): whatever kind made it, the screen draws
//! the vocabulary (`openpaint_practice::SheetMark`): lines in their pens (an original's in ink, a
//! given printed grey, the answer in see-through red as thick as the Draw pen, an original laid
//! over the copy at half strength, what is off in the marking red, construction faint, a
//! division's levels in V大's colours, hatching in a value's grey), rings, dots, a narrow corner's
//! arc, a shaded face, the horizon, swatches, patches, cells, outlines over the paper's halo,
//! pictures laid as strongly as asked, a pack's picture with its credit, a key shift's tendency,
//! a gamut mask and the swatches beside a frame (`mark_beside.rs`).
//! Lines run as their mark's clip says: as they lie, cut at the sheet's edge (`At`), kept off the
//! original's frame, or kept inside their own frame.

use egui::{Align2, Color32, FontId, Pos2, Rect, Shape, Stroke};
use openpaint_practice::{Clip, Figure, Frame, In, LinePen, Place, SheetMark, Source, within};
use openpaint_practice_app::PracticeProjection;

use openpaint_ui::design::{DIVIDER, INK, INK_SECONDARY, PANEL, PANEL_RULE_WIDTH};

use super::mark_beside as beside;
use super::mark_lines::{
    At, DASH, FAINT, GAP, HALO, MARK, RING, SEE_THROUGH, faint, horizon, ring,
};
use super::marks::{SheetPictures, colour, pictured, quad, value_colour};
use crate::palette::{CORRECTION, PRINTED};
use crate::text::{self as t, Words};

/// Widths in points: an original's line, the overlay's (the Draw pen's), and points to a canvas
/// pixel.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Widths {
    pub(crate) line: f32,
    pub(crate) overlay: f32,
    pub(crate) scale: f32,
}

/// How strongly an original laid over the copy shows; hatching's share of the pen's width.
const LAID: f32 = 0.5;
const HATCH: f32 = 0.6;
/// An outline's line over the paper's halo, in points.
const OUTLINE: f32 = 2.0;
const OUTLINE_HALO: f32 = 4.0;
/// A narrow corner's arc's radius; a middle's dot and an object's; an object's corner's ring; in
/// points. A given dot's radius in canvas pixels of a frame, and the least it shows at.
const ARC: f32 = 22.0;
const DOT: f32 = 3.0;
const SMALL_DOT: f32 = 2.0;
const FOUND: f32 = 4.5;
const GIVEN: f32 = 4.5;
const LEAST_GIVEN: f32 = 2.0;
/// How strongly a face is shaded, printed.
const SHADE: f32 = 0.22;
/// The tendency's disc beside the copy's frame: its radius and its gap from the frame, its name's
/// size and gap under it, canvas pixels of a lesson's frame.
const DISC: f32 = 22.0;
const DISC_GAP: f32 = 18.0;
const NAME_SIZE: f32 = 11.0;
const NAME_GAP: f32 = 6.0;

/// `mark` of exercise `index` over its pair.
pub(crate) fn draw(
    painter: &egui::Painter,
    (practice, index): (&PracticeProjection, usize),
    mark: &SheetMark,
    (at, widths): (At<'_>, Widths),
    pictures: &SheetPictures,
) {
    let pair = practice.sheet.pairs[index];
    let frame = match mark.on {
        In::Original => pair.original,
        In::Copy => pair.copy,
    };
    let at = match mark.clip {
        Clip::KeptOff => at.keeping_off(&pair.original),
        _ => at,
    };
    let inside;
    let painter = if mark.clip == Clip::Inside {
        let rect = Rect::from_points(&frame.corners().map(|p| at.of(p)));
        inside = painter.with_clip_rect(rect.intersect(painter.clip_rect()));
        &inside
    } else {
        painter
    };
    let laid = mark.clip == Clip::Laid;
    let map = |p: [f64; 2]| {
        let canvas = frame.to_canvas(p);
        at.of(if laid {
            canvas.map(|v| f64::from(v as f32))
        } else {
            canvas
        })
    };
    let on = Drawing {
        painter,
        frame: &frame,
        at,
        map: &map,
        widths,
    };
    on.figure(&mark.drawn, mark.clip, (practice, index), pictures);
}

/// What a mark is drawn with.
struct Drawing<'a> {
    painter: &'a egui::Painter,
    frame: &'a Frame,
    at: At<'a>,
    map: &'a dyn Fn([f64; 2]) -> Pos2,
    widths: Widths,
}

impl Drawing<'_> {
    fn figure(
        &self,
        figure: &Figure,
        clip: Clip,
        (practice, index): (&PracticeProjection, usize),
        pictures: &SheetPictures,
    ) {
        let (painter, map) = (self.painter, self.map);
        match figure {
            Figure::Line {
                points,
                pen,
                dashed,
            } => self.line(points, *pen, (*dashed, clip)),
            Figure::Ring(p) => ring(painter, (self.frame, *p), self.at, CORRECTION),
            Figure::Found(p) => {
                painter.circle_stroke(map(*p), FOUND, Stroke::new(MARK + 1.0, PANEL));
                painter.circle_stroke(map(*p), FOUND, Stroke::new(MARK, INK));
            }
            Figure::Dot { at, small } => {
                let radius = if *small { SMALL_DOT } else { DOT };
                painter.circle_filled(map(*at), radius, INK_SECONDARY);
            }
            Figure::Given { at, pen } => {
                let radius = (GIVEN * self.widths.scale).max(LEAST_GIVEN);
                painter.circle_filled(map(*at), radius, self.pen(*pen).color);
            }
            Figure::Counted { at, radius, pen } => {
                let centre = map(*at);
                let radius = centre.distance(map([at[0] + radius, at[1]]));
                painter.circle(centre, radius, PANEL, self.pen(*pen));
            }
            Figure::Narrow { at, legs } => self.narrow(*at, *legs),
            Figure::Shade(face) => {
                let face: Vec<Pos2> = face.iter().map(|p| map(*p)).collect();
                let shade = PRINTED.gamma_multiply(SHADE);
                painter.add(Shape::convex_polygon(face, shade, Stroke::NONE));
            }
            Figure::Horizon(height) => {
                horizon(painter, (self.frame, *height), self.at, self.widths.scale);
            }
            Figure::Swatch(rgb) => {
                let corners = self.frame.corners().map(|p| self.at.of(p)).to_vec();
                painter.add(Shape::convex_polygon(corners, colour(*rgb), Stroke::NONE));
            }
            Figure::Fill {
                corners,
                colour: rgb,
            } => {
                let corners = corners.iter().map(|p| map(*p)).collect();
                painter.add(Shape::convex_polygon(corners, colour(*rgb), Stroke::NONE));
            }
            Figure::Patch(rgb) => self.patch(*rgb),
            _ => self.painted(figure, (practice, index), pictures),
        }
    }

    /// The figures that paint: cells, outlines, pictures, the tendency.
    fn painted(
        &self,
        figure: &Figure,
        (practice, index): (&PracticeProjection, usize),
        pictures: &SheetPictures,
    ) {
        let painter = self.painter;
        match figure {
            Figure::Cell { cell, rect } => {
                let in_hand = practice.current == Some(index) && practice.cell == Some(*cell);
                let line = if in_hand {
                    Stroke::new(PANEL_RULE_WIDTH, INK)
                } else {
                    Stroke::new(1.0, DIVIDER)
                };
                painter.add(Shape::closed_line(self.rect(*rect), line));
            }
            Figure::Rule(rect) => {
                painter.add(Shape::closed_line(
                    self.rect(*rect),
                    Stroke::new(1.0, DIVIDER),
                ));
            }
            Figure::Uneven(rect) => {
                let both = self.rect(*rect);
                let halo = Stroke::new(OUTLINE + OUTLINE_HALO, PANEL);
                painter.add(Shape::closed_line(both.clone(), halo));
                painter.add(Shape::closed_line(both, Stroke::new(OUTLINE, CORRECTION)));
            }
            Figure::Haloed { lines, red } => {
                let (width, ink) = if *red {
                    (self.widths.overlay.max(OUTLINE), CORRECTION)
                } else {
                    (OUTLINE, INK)
                };
                let halo = Stroke::new(width + OUTLINE_HALO, PANEL);
                for stroke in [halo, Stroke::new(width, ink)] {
                    for line in lines {
                        let points = line.iter().map(|p| (self.map)(*p)).collect();
                        painter.add(Shape::line(points, stroke));
                    }
                }
            }
            Figure::Picture {
                source,
                place,
                shown,
            } => self.picture(*source, *place, *shown, (practice, index), pictures),
            Figure::Pictured => {
                let on = (self.frame, self.widths.scale);
                pictured((self.painter, self.at), (practice, index), on, pictures);
            }
            Figure::Tendency(rgb) => self.tendency(*rgb),
            Figure::Gamut {
                at,
                radius,
                corners,
            } => beside::gamut(painter, (self.frame, self.at), (*at, *radius), corners),
            Figure::Beside { at, colour, name } => {
                let on = (self.frame, self.at, self.widths.scale);
                beside::swatch(painter, on, (*at, *colour, *name));
            }
            _ => {}
        }
    }

    /// `pen`'s stroke.
    fn pen(&self, pen: LinePen) -> Stroke {
        let Widths { line, overlay, .. } = self.widths;
        match pen {
            LinePen::Ink => Stroke::new(line, INK),
            LinePen::Printed => Stroke::new(line, PRINTED),
            LinePen::PrintedHalf => Stroke::new(line / 2.0, PRINTED),
            LinePen::Answer => Stroke::new(overlay, CORRECTION.gamma_multiply(SEE_THROUGH)),
            LinePen::Laid => Stroke::new(overlay, CORRECTION.gamma_multiply(LAID)),
            LinePen::Off => Stroke::new(MARK, CORRECTION),
            LinePen::Faint => faint(),
            LinePen::Pencil => Stroke::new(FAINT, INK_SECONDARY),
            LinePen::Level(level) => {
                let [r, g, b] = level.colour();
                Stroke::new(overlay, Color32::from_rgb(r, g, b))
            }
            LinePen::Hatch(value) => Stroke::new(overlay * HATCH, value_colour(value)),
        }
    }

    /// The line through `points` in `pen`, dashed or whole, run as `clip` says: a division's
    /// levels round-joined and cut at their frame's edges, hatching a segment at a time.
    fn line(&self, points: &[[f64; 2]], pen: LinePen, (dashed, clip): (bool, Clip)) {
        let (painter, map, stroke) = (self.painter, self.map, self.pen(pen));
        match (pen, clip) {
            (LinePen::Level(_), _) => {
                for part in within(points, self.frame.extent()) {
                    let part: Vec<Pos2> = part.iter().map(|p| map(*p)).collect();
                    for segment in part.windows(2) {
                        painter.line_segment([segment[0], segment[1]], stroke);
                    }
                    for point in part {
                        painter.circle_filled(point, stroke.width / 2.0, stroke.color);
                    }
                }
            }
            (LinePen::Hatch(_), _) => {
                for segment in points.windows(2) {
                    painter.line_segment([map(segment[0]), map(segment[1])], stroke);
                }
            }
            (_, Clip::Free | Clip::Laid) => {
                let points = points.iter().map(|p| map(*p)).collect();
                painter.add(Shape::line(points, stroke));
            }
            _ => {
                let canvas: Vec<[f64; 2]> =
                    points.iter().map(|p| self.frame.to_canvas(*p)).collect();
                for part in self.at.parts(&canvas) {
                    if dashed {
                        painter.extend(Shape::dashed_line(&part, stroke, DASH, GAP));
                    } else {
                        painter.add(Shape::line(part, stroke));
                    }
                }
            }
        }
    }

    /// `rect` (top left, bottom right, frame units) as its four corners on the screen.
    fn rect(&self, [[left, top], [right, bottom]]: [[f64; 2]; 2]) -> Vec<Pos2> {
        [[left, top], [right, top], [right, bottom], [left, bottom]]
            .into_iter()
            .map(|p| (self.map)(p))
            .collect()
    }

    /// A front corner under 90 degrees at `p`: an arc between its `legs` and a ring, red.
    fn narrow(&self, p: [f64; 2], legs: [[f64; 2]; 2]) {
        let centre = (self.map)(p);
        let heading = |leg: [f64; 2]| {
            let far = (self.map)([p[0] + leg[0] * 0.05, p[1] + leg[1] * 0.05]);
            (far.y - centre.y).atan2(far.x - centre.x)
        };
        let (from, to) = (heading(legs[0]), heading(legs[1]));
        let mut sweep = to - from;
        while sweep > std::f32::consts::PI {
            sweep -= std::f32::consts::TAU;
        }
        while sweep < -std::f32::consts::PI {
            sweep += std::f32::consts::TAU;
        }
        let arc: Vec<Pos2> = (0..=16)
            .map(|k| {
                let angle = from + sweep * k as f32 / 16.0;
                centre + ARC * egui::vec2(angle.cos(), angle.sin())
            })
            .collect();
        let painter = self.painter;
        painter.add(Shape::line(arc.clone(), Stroke::new(MARK + HALO, PANEL)));
        painter.add(Shape::line(arc, Stroke::new(MARK, CORRECTION)));
        painter.circle_stroke(centre, RING, Stroke::new(MARK + HALO, PANEL));
        painter.circle_stroke(centre, RING, Stroke::new(MARK, CORRECTION));
    }

    /// A colour compared: a patch in the middle of the frame.
    fn patch(&self, rgb: [u8; 3]) {
        let [[x, y], _, [right, bottom], _] = self.frame.corners();
        let (inset, side) = ((right - x) / 4.0, (bottom - y) / 2.0);
        let [x, y] = [x + inset, y + inset];
        let corners = [[x, y], [x + side, y], [x + side, y + side], [x, y + side]];
        let corners: Vec<Pos2> = corners.into_iter().map(|p| self.at.of(p)).collect();
        (self.painter).add(Shape::convex_polygon(corners, colour(rgb), Stroke::NONE));
    }

    /// A picture of `source` laid at `place`, `shown` strong.
    fn picture(
        &self,
        source: Source,
        place: Place,
        shown: f32,
        (practice, index): (&PracticeProjection, usize),
        pictures: &SheetPictures,
    ) {
        let pack = || {
            let item = &practice.sheet.pictures.get(index)?.as_ref()?.item;
            pictures.texture(item).flatten()
        };
        let texture = match source {
            Source::Pack => pack(),
            Source::Painted => pictures.painted(practice, index),
            Source::PaintedOrPack => pictures.painted(practice, index).or_else(pack),
            Source::Read(at) => pictures.read(usize::from(at)),
        };
        let Some(texture) = texture else {
            return;
        };
        let corners = match place {
            Place::Frame => self.frame.corners().map(|p| self.at.of(p)),
            Place::Corners(corners) => corners.map(|p| (self.map)(p)),
        };
        let tint = Color32::WHITE.gamma_multiply(shown);
        self.painter.add(quad(texture, corners, tint));
    }

    /// A key shift's tendency beside the copy's frame: a disc of its colour, named under it.
    fn tendency(&self, rgb: [u8; 3]) {
        let (painter, frame, scale) = (self.painter, self.frame, self.widths.scale);
        let x = frame.origin[0] + frame.size[0] + f64::from(DISC_GAP + DISC);
        let y = frame.origin[1] + f64::from(DISC);
        let centre = self.at.of([x, y]);
        painter.circle(centre, DISC * scale, colour(rgb), Stroke::new(1.0, INK));
        let below = self.at.of([x, y + f64::from(DISC + NAME_GAP)]);
        let name = Words::from(t::TENDENCY).shown(painter.ctx());
        let font = FontId::proportional((NAME_SIZE * scale).max(6.0));
        painter.text(below, Align2::CENTER_TOP, name, font, INK_SECONDARY);
    }
}
