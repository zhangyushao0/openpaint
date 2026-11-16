//! Light on form's scene rules that need the light (S153), read off a coarse survey of the model
//! (`SURVEY` cells across the frame, each its middle as the light paints it): some of each family
//! seen on the form; its cast shadow seen, a share of the frame at least, and inside the frame; at
//! 熟练 and 稳 a round form's terminator crossing its visible side; and a pin standing on the ground
//! with its points inside the frame, clear of the form and its shadow.

use crate::geometry::light::Pin;
use crate::kinds::light_on_form::answer::{Mass, Shading};
use crate::kinds::light_on_form::scene::{Form, LightTask, Tones};
use crate::maths::path::{Point, distance};
use crate::maths::random::Random;

/// The survey's cells across the frame.
const SURVEY: usize = 64;
/// A pin's height (world units); its points inside the frame by this; it and its shadow at least
/// this clear of the form and its shadow.
const PIN: [f64; 2] = [0.4, 0.6];
const PIN_INSIDE: f64 = 0.04;
const CLEAR: f64 = 0.05;
/// The cast shadow inside the frame by this.
const SHADOW_INSIDE: f64 = 0.03;
/// The light family's share of the form seen, at least and at most; the cast shadow seen, a share
/// of the frame at least; a round form's turn either side of its terminator seen.
const FAMILIES: [f64; 2] = [0.12, 0.88];
const CAST_LEAST: f64 = 0.01;
const CROSSED: f64 = 0.1;

/// The model looked at on a coarse grid: each cell's middle as the light paints it.
pub(crate) struct Survey {
    cells: Vec<Option<Shading>>,
}

impl Survey {
    pub(crate) fn of(task: &LightTask) -> Self {
        let middle = |k: usize| (k as f64 + 0.5) / SURVEY as f64;
        let cells = (0..SURVEY * SURVEY)
            .map(|at| task.shading([middle(at % SURVEY), middle(at / SURVEY)]))
            .collect();
        Self { cells }
    }

    /// Whether no cell within `reach` of `p` holds the form or its cast shadow.
    fn clear(&self, p: Point, reach: f64) -> bool {
        let cells = (reach * SURVEY as f64).ceil() as isize + 1;
        let [cx, cy] = p.map(|v| (v * SURVEY as f64).floor() as isize);
        let side = SURVEY as isize;
        (-cells..=cells).all(|dy| {
            (-cells..=cells).all(|dx| {
                let [x, y] = [cx + dx, cy + dy];
                if !(0..side).contains(&x) || !(0..side).contains(&y) {
                    return true;
                }
                let middle = [x, y].map(|v| (v as f64 + 0.5) / SURVEY as f64);
                let held = self.cells[(y * side + x) as usize].as_ref();
                distance(middle, p) > reach || held.is_none_or(|s| !(s.on_form || s.cast()))
            })
        })
    }

    /// Whether the families, the cast shadow and the terminator keep their rules.
    pub(crate) fn keeps(&self, task: &LightTask) -> bool {
        let form: Vec<&Shading> = self.cells.iter().flatten().filter(|s| s.on_form).collect();
        let lit = form.iter().filter(|s| s.family == Mass::Light).count();
        let share = lit as f64 / form.len().max(1) as f64;
        let edge = (SHADOW_INSIDE * SURVEY as f64).ceil() as usize;
        let rim = |at: usize| {
            let (x, y) = (at % SURVEY, at / SURVEY);
            x < edge || y < edge || x >= SURVEY - edge || y >= SURVEY - edge
        };
        let cast = |at: &usize| self.cells[*at].as_ref().is_some_and(Shading::cast);
        let casts = (0..self.cells.len()).filter(cast).count();
        let inside = !(0..self.cells.len()).filter(cast).any(rim);
        let round = || form.iter().filter(|s| s.round);
        let crossed = round().any(|s| s.turned > CROSSED) && round().any(|s| s.turned < -CROSSED);
        let turning = matches!(task.lit.form, Form::Cylinder | Form::Cone | Form::Sphere);
        let terminator = task.lit.tones == Tones::Two || !turning || crossed;
        (FAMILIES[0]..=FAMILIES[1]).contains(&share)
            && casts as f64 >= CAST_LEAST * self.cells.len() as f64
            && inside
            && terminator
    }

    /// A pin laid from `random` on the ground `task`'s frame shows: its points inside the frame,
    /// clear of the form and its shadow; none else.
    pub(crate) fn pin(&self, task: &LightTask, random: &mut Random) -> Option<Pin> {
        let at = [random.range(0.08, 0.92), random.range(0.45, 0.94)];
        let pin = Pin::at(task.on_ground(at)?, random.range(PIN[0], PIN[1]));
        self.stands(task, &pin).then_some(pin)
    }

    /// Whether `pin` stands with its points inside `task`'s frame, it and its shadow clear of the
    /// form and its shadow.
    pub(crate) fn stands(&self, task: &LightTask, pin: &Pin) -> bool {
        let Some(tip) = pin.tip(&task.light) else {
            return false;
        };
        let [foot, top, tip] = [pin.foot, pin.top, tip].map(|p| task.view.at(p));
        let inside = |p: Point| {
            p.iter()
                .all(|v| (PIN_INSIDE..=1.0 - PIN_INSIDE).contains(v))
        };
        let clear = |[a, b]: [Point; 2]| {
            let steps = (distance(a, b) / 0.01).ceil().max(1.0) as usize;
            (0..=steps).all(|k| {
                let s = k as f64 / steps as f64;
                self.clear([a[0] + (b[0] - a[0]) * s, a[1] + (b[1] - a[1]) * s], CLEAR)
            })
        };
        let kept = [foot, top, tip].into_iter().all(inside);
        kept && clear([foot, top]) && clear([foot, tip])
    }
}

impl LightTask {
    /// Whether it keeps every rule: the form's box in the frame and the horizon above it, the
    /// families, the cast shadow and the terminator as the survey reads them, and the pin.
    #[cfg(test)]
    pub(crate) fn keeps(&self) -> bool {
        let survey = Survey::of(self);
        self.inside() && survey.keeps(self) && survey.stands(self, &self.pin)
    }
}
