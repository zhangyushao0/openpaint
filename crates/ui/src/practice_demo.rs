//! A How card's demonstration (S92): under the card's title a small example of the kind (for a
//! copying kind the original and the copy side by side; a box, an ellipse or a point in space
//! alone, as large as the card lets it, a box's vanishing points beside it; a colour's two frames
//! and a little wheel; a value scale's one frame cropped to its rows beside the wheel, of as many
//! cells as the row in hand, S97) where V大's way draws itself step by step
//! (`openpaint_practice::Demo`), and under it the numbered steps, the one being drawn in ink and the
//! others in pencil grey, every one in ink once it is over. It plays whenever How opens, once
//! through on the frame clock, then stands still on its last frame; Again plays it from the start.
//! Frames are wanted only while it moves (D97). Nothing of it touches the sheet.

use std::fmt;
use std::sync::Arc;

use egui::{Rect, Sense, TextureHandle, Vec2};
use openpaint_practice::{Demo, Drawn, Family, Layout, Marker, On, Rgb8, ValueMasses};

use crate::components::{Alpha, card_step, keep_moving, picture_texture};
use crate::design::{INK, PENCIL, SPACE};
use crate::practice_demo_marks::{Frames, paint};
use crate::text::Words;

/// The example's height (a lone one's taller: a box with its points, a trick's face), and between
/// its frames, in points.
const HEIGHT: f32 = 176.0;
const LONE_HEIGHT: f32 = 224.0;
const GAP: f32 = 2.0 * SPACE;
/// Between a fitted example and its paper's edge, in points.
const FIT_MARGIN: f32 = 1.5 * SPACE;
/// A value scale example's wheel, a share of the example's height (S97).
const ROW_WHEEL: f32 = 0.7;
/// A combination's picture's side, in pixels.
const PICTURE: u32 = 128;
/// An object's picture's side, in pixels (its frame some 176 points, at up to 2 pixels a point).
const OBJECT_PICTURE: u32 = 360;

/// A landscape's picture's width, in pixels (its frame some 176 points, S95).
const MASSES_PICTURE: u32 = 360;

/// Which picture of an example a texture is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PictureKey {
    /// A combination's, by its areas' colours, its edges soft where the picture's are or all hard
    /// (S100).
    Areas(Vec<Rgb8>, bool),
    /// The example object's (S94).
    Object,
    /// The example landscape's (S95).
    Masses,
    /// Its value masses (S96): the hand's copy's or the answer's, the values shown.
    Values { copy: bool, shown: [bool; 3] },
}

/// The demonstrations made so far and the one playing.
#[derive(Default)]
pub(crate) struct Demonstrations {
    made: Vec<(HowOf, Arc<Demo>)>,
    playing: Option<Playing>,
    /// The examples' pictures.
    pictures: Vec<(PictureKey, TextureHandle)>,
}

impl fmt::Debug for Demonstrations {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "demonstrations: {} made", self.made.len())
    }
}

/// Which How a card shows: its family's, a value scale's for the cells of the row in hand (S97:
/// its example and steps of that count; every other family's the same whatever is in hand).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HowOf {
    pub(crate) family: Family,
    cells: usize,
}

impl HowOf {
    /// `family`'s How with a value scale of `cells` in hand.
    pub(crate) fn new(family: Family, cells: usize) -> Self {
        let cells = if family == Family::Scale { cells } else { 0 };
        Self { family, cells }
    }

    /// The cells a value scale's How shows.
    pub(crate) const fn cells(self) -> usize {
        self.cells
    }
}

/// The demonstration on the card: which, since when it plays, and the pass it last showed in.
struct Playing {
    of: HowOf,
    demo: Arc<Demo>,
    start: f64,
    pass: u64,
}

impl Demonstrations {
    /// `of`'s demonstration under the card's title, playing since How opened (a card not shown in
    /// the pass before has just opened), and its numbered `steps`. A kind without one (the colour
    /// copy) shows nothing.
    pub(crate) fn show(&mut self, ui: &mut egui::Ui, of: HowOf, steps: &[String]) {
        let ctx = ui.ctx().clone();
        let (now, pass) = (ctx.input(|input| input.time), ctx.cumulative_pass_nr());
        let still = (self.playing.as_ref()).is_some_and(|p| p.of == of && p.pass + 1 >= pass);
        if !still {
            let Some(demo) = self.made(of) else {
                self.playing = None;
                return;
            };
            self.playing = Some(Playing {
                of,
                demo,
                start: now,
                pass,
            });
        }
        let Some(playing) = self.playing.as_mut() else {
            return;
        };
        playing.pass = pass;
        let demo = Arc::clone(&playing.demo);
        let seconds = now - playing.start;
        self.example(ui, &demo, seconds);
        let (lit, over) = (demo.step_at(seconds), seconds >= demo.length);
        for (index, words) in steps.iter().enumerate() {
            let ink = if over || index == lit { INK } else { PENCIL };
            card_step(ui, index + 1, Words::made(words), ink);
        }
        if !over {
            keep_moving(&ctx);
        }
    }

    /// The demonstration on the card plays again from the start.
    pub(crate) fn again(&mut self, ctx: &egui::Context) {
        if let Some(playing) = &mut self.playing {
            playing.start = ctx.input(|input| input.time);
            keep_moving(ctx);
        }
    }

    /// `of`'s demonstration, made once.
    fn made(&mut self, of: HowOf) -> Option<Arc<Demo>> {
        if let Some((_, made)) = self.made.iter().find(|(made, _)| *made == of) {
            return Some(Arc::clone(made));
        }
        let demo = match of.family {
            Family::Scale => Demo::of_scale(of.cells),
            family => Demo::of(family)?,
        };
        let made = Arc::new(demo);
        self.made.push((of, Arc::clone(&made)));
        Some(made)
    }

    /// The example `seconds` after the start, its frames laid out across the card.
    fn example(&mut self, ui: &mut egui::Ui, demo: &Demo, seconds: f64) {
        let width = ui.available_width();
        let height = match demo.layout {
            Layout::Wheel => (width - 2.0 * GAP) / 3.0,
            Layout::Single | Layout::Wide => LONE_HEIGHT,
            Layout::Pair | Layout::Row => HEIGHT,
        };
        let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
        let frames = laid(demo, rect);
        for mark in &demo.marks {
            let key = match &mark.drawn {
                Drawn::Areas { colours, soft } if mark.from <= seconds => {
                    PictureKey::Areas(colours.clone(), *soft)
                }
                Drawn::Shaded => PictureKey::Object,
                Drawn::Masses(_) => PictureKey::Masses,
                Drawn::Values { copy, shown, .. } => PictureKey::Values {
                    copy: *copy,
                    shown: *shown,
                },
                _ => continue,
            };
            self.picture(ui.ctx(), demo, key);
        }
        let pictures = |key: &PictureKey| -> Option<TextureHandle> {
            (self.pictures.iter())
                .find(|(shown, _)| shown == key)
                .map(|(_, texture)| texture.clone())
        };
        paint(ui.painter(), (demo, seconds), &frames, &pictures);
    }

    /// Makes the example's picture `key` names, once: a combination's with its areas in its
    /// colours, the object's (S94), the landscape's (S95).
    fn picture(&mut self, ctx: &egui::Context, demo: &Demo, key: PictureKey) {
        if self.pictures.iter().any(|(shown, _)| *shown == key) {
            return;
        }
        let made = match &key {
            PictureKey::Areas(colours, soft) => demo.areas.clone().map(|mut areas| {
                areas.colours.clone_from(colours);
                if !soft {
                    areas.soft.fill(0.0);
                }
                ([PICTURE; 2], areas.picture(PICTURE))
            }),
            PictureKey::Object => (demo.object.as_deref())
                .map(|object| ([OBJECT_PICTURE; 2], object.picture(OBJECT_PICTURE))),
            PictureKey::Masses => (demo.masses.as_deref()).map(|m| m.picture(MASSES_PICTURE)),
            PictureKey::Values { copy, shown } => (demo.values.as_deref()).map(|maps| {
                let map = &maps[usize::from(*copy)];
                (map.size, shown_values(map, *shown))
            }),
        };
        let Some((size, pixels)) = made else {
            return;
        };
        let name = format!("demonstration-{}", self.pictures.len());
        let texture = picture_texture(ctx, name, (size, &pixels, Alpha::Straight));
        self.pictures.push((key, texture));
    }
}

/// `map`'s cells in their greys where their value is `shown` (dark, middle, light), clear
/// elsewhere (S96).
fn shown_values(map: &ValueMasses, shown: [bool; 3]) -> Vec<u8> {
    (map.picture().chunks(4).zip(&map.values))
        .flat_map(|(pixel, value)| {
            let on = shown.get(usize::from(*value)).copied().unwrap_or(false);
            [pixel[0], pixel[1], pixel[2], if on { 255 } else { 0 }]
        })
        .collect()
}

/// Where the example's frames lie in `rect`: a copying kind's frames side by side; a box's, an
/// ellipse's or a point in space's example as large as the card lets it (its frame left out,
/// the paper the whole example).
fn laid(demo: &Demo, rect: Rect) -> Frames {
    let square = |left: f32, side: f32| {
        let top = rect.center().y - side / 2.0;
        Rect::from_min_size(egui::pos2(left, top), Vec2::splat(side))
    };
    match demo.layout {
        Layout::Pair => {
            let side = ((rect.width() - GAP) / 2.0).min(rect.height());
            let left = rect.center().x - side - GAP / 2.0;
            let [original, copy] = [square(left, side), square(left + side + GAP, side)];
            // An object's example cropped to its box in both frames (S94), so it reads large; a
            // composition's to its picture, the frames its shape (S99).
            if let Some(view) = demo.view {
                let [original, copy] = [(original, true), (copy, false)]
                    .map(|(frame, against)| shaped(view, frame, against));
                return Frames {
                    shown: Some([original, copy]),
                    ..Frames::of(Some(fitted(demo, original)), fitted(demo, copy), None)
                };
            }
            Frames::of(Some(original), copy, None)
        }
        Layout::Single | Layout::Wide => Frames {
            wide: Some(rect),
            ..Frames::of(None, fitted(demo, rect.shrink(FIT_MARGIN)), None)
        },
        Layout::Wheel => {
            let side = rect.height();
            let left = rect.left();
            let at = |k: f32| square(left + k * (side + GAP), side);
            Frames::of(Some(at(0.0)), at(1.0), Some(at(2.0)))
        }
        // A value scale's one frame cropped to its rows, the wheel smaller at its right (S97),
        // so the row's nine cells and their numbers read.
        Layout::Row => {
            let side = rect.height() * ROW_WHEEL;
            let wheel = square(rect.right() - side, side);
            let row = Rect::from_min_max(rect.min, egui::pos2(wheel.left() - GAP, rect.max.y));
            Frames {
                wide: Some(row),
                ..Frames::of(None, fitted(demo, row), Some(wheel))
            }
        }
    }
}

/// The largest part of `square` of the shape of `view` (a demonstration's part shown, frame
/// units), in its middle from top to bottom and against its right side if `right` (the
/// original's, against the gap) or its left (the copy's), as the sheet lays frames (S99).
fn shaped([low, high]: [[f64; 2]; 2], square: Rect, right: bool) -> Rect {
    let span = [high[0] - low[0], high[1] - low[1]].map(|v| v.max(1e-3) as f32);
    let scale = (square.width() / span[0]).min(square.height() / span[1]);
    let size = Vec2::new(span[0] * scale, span[1] * scale);
    let left = if right {
        square.right() - size.x
    } else {
        square.left()
    };
    Rect::from_min_size(egui::pos2(left, square.center().y - size.y / 2.0), size)
}

/// Where the frame lies so the part of it the example shows (`Demo::view`: a box with its
/// vanishing points, a trick's face) fills `rect`, centred.
fn fitted(demo: &Demo, rect: Rect) -> Rect {
    let Some([low, high]) = demo.view else {
        return rect;
    };
    let span = [high[0] - low[0], high[1] - low[1]].map(|v| v.max(1e-3) as f32);
    let side = (rect.width() / span[0]).min(rect.height() / span[1]);
    let middle = [(low[0] + high[0]) / 2.0, (low[1] + high[1]) / 2.0].map(|v| v as f32);
    let origin = rect.center() - Vec2::new(middle[0], middle[1]) * side;
    Rect::from_min_size(origin, Vec2::splat(side))
}

/// The frame a mark `on` lies in, if the example has it.
pub(crate) fn placed(frames: &Frames, on: On) -> Option<Rect> {
    match on {
        On::Original => frames.original,
        On::Copy => Some(frames.copy),
        On::Wheel => frames.wheel,
    }
}

/// How far `mark` is drawn `seconds` after the start, 0 to 1.
pub(crate) fn drawn(mark: &openpaint_practice::Mark, seconds: f64) -> f32 {
    if seconds < mark.from {
        0.0
    } else if mark.to <= mark.from {
        1.0
    } else {
        ((seconds - mark.from) / (mark.to - mark.from)).clamp(0.0, 1.0) as f32
    }
}

/// Whether `pen` draws its line along its length (a hand's), rather than showing it at once
/// (what the exercise gives, Compare's marks).
pub(crate) const fn by_hand(pen: Marker) -> bool {
    matches!(
        pen,
        Marker::Pencil | Marker::Construction | Marker::Air | Marker::Ink | Marker::Level(_)
    )
}

#[cfg(test)]
#[path = "practice_demo_tests.rs"]
mod tests;
