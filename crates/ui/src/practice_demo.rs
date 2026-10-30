//! A How card's demonstration (S92): under the card's title a small example of the kind (for a
//! copying kind the original and the copy side by side; a box, an ellipse or a point in space
//! alone, as large as the card lets it, a box's vanishing points beside it; a colour's or a value
//! scale's two frames and a little wheel) where V大's way draws itself step by step
//! (`openpaint_practice::Demo`), and under it the numbered steps, the one being drawn in ink and the
//! others in pencil grey, every one in ink once it is over. It plays whenever How opens, once
//! through on the frame clock, then stands still on its last frame; Again plays it from the start.
//! Frames are wanted only while it moves (D97). Nothing of it touches the sheet.

use std::fmt;
use std::sync::Arc;

use egui::{Rect, Sense, TextureHandle, Vec2};
use openpaint_practice::{Demo, Drawn, Family, Layout, Marker, On, Rgb8};

use crate::components::{Alpha, card_step, keep_moving, picture_texture};
use crate::design::{INK, PENCIL, SPACE};
use crate::practice_demo_marks::{Frames, paint};
use crate::text::Text;

/// The example's height (a lone one's taller: a box with its points, a trick's face), and between
/// its frames, in points.
const HEIGHT: f32 = 176.0;
const LONE_HEIGHT: f32 = 224.0;
const GAP: f32 = 2.0 * SPACE;
/// Between a fitted example and its paper's edge, in points.
const FIT_MARGIN: f32 = 1.5 * SPACE;
/// A combination's picture's side, in pixels.
const PICTURE: u32 = 128;
/// An object's picture's side, in pixels (its frame some 176 points, at up to 2 pixels a point).
const OBJECT_PICTURE: u32 = 360;

/// The demonstrations made so far and the one playing.
#[derive(Default)]
pub(crate) struct Demonstrations {
    made: Vec<Arc<Demo>>,
    playing: Option<Playing>,
    /// A combination's pictures by their areas' colours.
    pictures: Vec<(Vec<Rgb8>, TextureHandle)>,
}

impl fmt::Debug for Demonstrations {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "demonstrations: {} made", self.made.len())
    }
}

/// The demonstration on the card: since when it plays, and the pass it last showed in.
struct Playing {
    demo: Arc<Demo>,
    start: f64,
    pass: u64,
}

impl Demonstrations {
    /// `family`'s demonstration under the card's title, playing since How opened (a card not
    /// shown in the pass before has just opened), and its numbered `steps`. A kind without one
    /// (the colour copy) shows nothing.
    pub(crate) fn show(&mut self, ui: &mut egui::Ui, family: Family, steps: &[Text]) {
        let ctx = ui.ctx().clone();
        let (now, pass) = (ctx.input(|input| input.time), ctx.cumulative_pass_nr());
        let still =
            (self.playing.as_ref()).is_some_and(|p| p.demo.family == family && p.pass + 1 >= pass);
        if !still {
            let Some(demo) = self.made(family) else {
                self.playing = None;
                return;
            };
            self.playing = Some(Playing {
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
            card_step(ui, index + 1, *words, ink);
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

    /// `family`'s demonstration, made once.
    fn made(&mut self, family: Family) -> Option<Arc<Demo>> {
        if let Some(made) = self.made.iter().find(|d| d.family == family) {
            return Some(Arc::clone(made));
        }
        let made = Arc::new(Demo::of(family)?);
        self.made.push(Arc::clone(&made));
        Some(made)
    }

    /// The example `seconds` after the start, its frames laid out across the card.
    fn example(&mut self, ui: &mut egui::Ui, demo: &Demo, seconds: f64) {
        let width = ui.available_width();
        let height = match demo.layout {
            Layout::Wheel => (width - 2.0 * GAP) / 3.0,
            Layout::Single | Layout::Wide => LONE_HEIGHT,
            Layout::Pair => HEIGHT,
        };
        let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
        let frames = laid(demo, rect);
        for mark in &demo.marks {
            match &mark.drawn {
                Drawn::Areas(colours) if mark.from <= seconds => {
                    self.picture(ui.ctx(), demo, colours);
                }
                Drawn::Shaded => self.shaded(ui.ctx(), demo),
                _ => {}
            }
        }
        let pictures = |colours: &[Rgb8]| -> Option<TextureHandle> {
            (self.pictures.iter())
                .find(|(shown, _)| shown == colours)
                .map(|(_, texture)| texture.clone())
        };
        paint(ui.painter(), (demo, seconds), &frames, &pictures);
    }

    /// Makes the example combination's picture with its areas in `colours`, once.
    fn picture(&mut self, ctx: &egui::Context, demo: &Demo, colours: &[Rgb8]) {
        if self.pictures.iter().any(|(shown, _)| shown == colours) {
            return;
        }
        let Some(mut areas) = demo.areas.clone() else {
            return;
        };
        areas.colours = colours.to_vec();
        let pixels = areas.picture(PICTURE);
        let name = format!("demonstration-{}", self.pictures.len());
        let texture = picture_texture(ctx, name, ([PICTURE; 2], &pixels, Alpha::Straight));
        self.pictures.push((colours.to_vec(), texture));
    }
}

impl Demonstrations {
    /// Makes the example object's picture (S94), once; it goes by no colours.
    fn shaded(&mut self, ctx: &egui::Context, demo: &Demo) {
        if self.pictures.iter().any(|(shown, _)| shown.is_empty()) {
            return;
        }
        let Some(object) = demo.object.as_deref() else {
            return;
        };
        let pixels = object.picture(OBJECT_PICTURE);
        let name = "demonstration-object".to_owned();
        let texture = picture_texture(ctx, name, ([OBJECT_PICTURE; 2], &pixels, Alpha::Straight));
        self.pictures.push((Vec::new(), texture));
    }
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
            // An object's example cropped to its box in both frames (S94), so it reads large.
            if demo.view.is_some() {
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
    }
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
        Marker::Pencil | Marker::Construction | Marker::Air | Marker::Ink
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::moving;
    use crate::text;

    #[test]
    fn an_objects_construction_is_dark_in_its_step_and_faint_from_the_next_on() {
        use crate::practice_demo_marks::toned;
        let demo = Demo::of(Family::Object).expect("an object's demonstration");
        let built = |m: &&openpaint_practice::Mark| m.pen == Marker::Construction;
        for mark in demo.marks.iter().filter(built) {
            let next = demo.steps[demo.step_at(mark.from) + 1];
            assert_eq!(toned(&demo, mark, mark.to), INK, "dark while drawn");
            assert_eq!(
                toned(&demo, mark, next + 1.0),
                PENCIL,
                "faint in the next step"
            );
            assert_eq!(
                toned(&demo, mark, demo.length),
                PENCIL,
                "faint under Compare"
            );
        }
    }

    #[test]
    fn every_kinds_card_has_a_step_for_each_step_of_its_demonstration() {
        for family in Family::ALL {
            let how = text::how(family);
            match Demo::of(family) {
                Some(demo) => assert_eq!(how.steps.len(), demo.steps.len(), "{family:?}"),
                None => assert!(how.steps.is_empty() && how.trains.is_none(), "{family:?}"),
            }
        }
    }

    /// One pass at `time` seconds, the card showing `family`'s demonstration when `shown` (and
    /// Again tapped when `again`); whether frames are wanted after it.
    fn pass(
        (ctx, demos): (&egui::Context, &mut Demonstrations),
        time: f64,
        (family, shown, again): (Family, bool, bool),
    ) -> bool {
        let input = egui::RawInput {
            time: Some(time),
            ..Default::default()
        };
        let mut wanted = false;
        let mut output = ctx.run_ui(input, |ui| {
            if shown {
                demos.show(ui, family, text::how(family).steps);
            }
            if again {
                demos.again(ui.ctx());
            }
            wanted = moving(ui.ctx());
        });
        output.textures_delta.clear();
        wanted
    }

    #[test]
    fn it_plays_once_through_then_stands_still_until_again_or_how_opens_it_again() {
        let ctx = egui::Context::default();
        let mut demos = Demonstrations::default();
        let family = Family::Centre;
        let length = Demo::of(family).map_or(0.0, |demo| demo.length);
        let mut at = |time: f64, shown: bool, again: bool| {
            pass((&ctx, &mut demos), time, (family, shown, again))
        };
        assert!(at(10.0, true, false), "it plays as How opens");
        assert!(at(10.0 + length / 2.0, true, false));
        assert!(
            !at(10.0 + length + 0.1, true, false),
            "still on its last frame"
        );
        assert!(!at(30.0, true, false), "no frames while it stands still");
        assert!(at(30.0, true, true), "Again plays it from the start");
        assert!(at(30.0 + length / 2.0, true, false));
        assert!(!at(30.0 + length + 0.1, true, false));
        let _ = at(50.0, false, false);
        assert!(at(60.0, true, false), "How opened again plays it again");
    }
}
