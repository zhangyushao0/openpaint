//! Emerge and retract (D44): how a surface grows out of its source and goes back into it. A
//! surface's growth is two critically damped springs, across and down, from 0 (its source) to
//! 1 (the finished surface); its shape in a frame follows from them. A panel grows out of its
//! island button's ink ring (the approved demo's container transform: the top edge crosses the
//! island's lower edge at once and waits under it while the panel pours down); any other
//! surface grows from the rectangle it comes from. The content shows only near the end of the
//! growth, laid out at its finished place, so the hand can use it from the first frame.

use egui::{Pos2, Rect, Vec2};

use crate::design::TOUCH_TARGET;
use crate::design::motion::{
    CLOSE, OPEN, POUR, RETRACT, RETRACT_FADE, REVEAL, Spring, lerp, smoothstep,
};

/// The ink ring's radius around a button's centre, in points: where a panel starts.
pub(crate) const RING: f32 = 0.4 * TOUCH_TARGET;
/// How much faster than the growth a panel's top edge travels, so it crosses the island's lower
/// edge in a blink, then waits at its place under the island.
const TOP_LEAD: f32 = 1.6;
/// The band under the island's lower edge, in points, that a panel's top edge steps over: there
/// the two ink lines would run a hair apart. The step is smaller than one frame of the motion.
const SLIVER: (f32, f32) = (-1.0, 5.0);

/// One surface's growth.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Growth {
    pub(crate) open: bool,
    /// Retracting because another panel was tapped: fast, content fading out at once.
    pub(crate) retracting: bool,
    /// When it last changed direction, so the most recent draws on top.
    pub(crate) since: u64,
    /// Seconds before it starts to grow.
    delay: f32,
    x: Spring,
    y: Spring,
    /// Its content's share that a retraction has not yet faded out.
    fade: f32,
}

impl Growth {
    /// Opening from its source, after `delay` seconds.
    pub(crate) const fn new(since: u64, delay: f32) -> Self {
        Self {
            open: true,
            retracting: false,
            since,
            delay,
            x: Spring::at(0.0),
            y: Spring::at(0.0),
            fade: 1.0,
        }
    }

    pub(crate) const fn close(&mut self, since: u64, retracting: bool) {
        self.open = false;
        self.retracting = retracting;
        self.since = since;
    }

    /// Grows again from wherever it is, keeping its velocity.
    pub(crate) const fn reopen(&mut self, since: u64) {
        self.open = true;
        self.retracting = false;
        self.since = since;
    }

    /// Opened but not yet growing: a later tap may take its place without any motion.
    pub(crate) fn pending(&self) -> bool {
        self.delay > 0.0 && self.x.at_rest(0.0)
    }

    pub(crate) fn step(&mut self, dt: f32) {
        let used = self.delay.min(dt);
        self.delay -= used;
        let dt = dt - used;
        if self.delay > 0.0 || dt <= 0.0 {
            return;
        }
        let target = if self.open { 1.0 } else { 0.0 };
        let (feel, pour) = match (self.open, self.retracting) {
            (true, _) => (OPEN, POUR),
            (false, true) => (RETRACT, 1.0),
            (false, false) => (CLOSE, 1.0),
        };
        self.x.step(target, feel, 1.0, dt);
        self.y.step(target, feel, pour, dt);
        let fading = dt / RETRACT_FADE;
        self.fade = if self.retracting {
            (self.fade - fading).max(0.0)
        } else {
            (self.fade + fading).min(1.0)
        };
    }

    pub(crate) fn at_rest(&self) -> bool {
        let target = if self.open { 1.0 } else { 0.0 };
        self.delay == 0.0 && self.x.at_rest(target) && self.y.at_rest(target)
    }

    /// Closed and at rest: nothing left to draw.
    pub(crate) fn gone(&self) -> bool {
        !self.open && self.at_rest()
    }

    /// Across and down, each 0 at the source and 1 finished.
    fn progress(&self) -> (f32, f32) {
        (self.x.x.max(0.0), self.y.x.max(0.0))
    }

    /// How far it has grown overall, 0 to 1.
    pub(crate) fn grown(&self) -> f32 {
        let (x, y) = self.progress();
        x.min(y).clamp(0.0, 1.0)
    }

    /// How much of the content shows.
    pub(crate) fn reveal(&self) -> f32 {
        smoothstep(REVEAL.0, REVEAL.1, self.grown()) * self.fade
    }
}

/// A surface's shape in one frame, in points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Shape {
    pub(crate) rect: Rect,
    pub(crate) radius: f32,
    /// 0 while its outline is the ink ring it grew from, 1 once it is a panel's rule.
    pub(crate) ring_to_rule: f32,
}

/// `top` moved out of the sliver band under `edge`, to its nearer side.
fn skip_sliver(top: f32, edge: f32) -> f32 {
    let (above, below) = (edge + SLIVER.0, edge + SLIVER.1);
    if top <= above || top >= below {
        top
    } else if top - above < below - top {
        above
    } else {
        below
    }
}

/// A panel between the ring around its `button` (the centre) and the finished `panel`, under an
/// island whose lower edge is `island_bottom`.
pub(crate) fn from_button(growth: &Growth, button: Pos2, panel: Rect, island_bottom: f32) -> Shape {
    let (px, py) = growth.progress();
    let centre_x = lerp(button.x, panel.center().x, px);
    let half_x = lerp(RING, panel.width() / 2.0, px);
    let lead = 1.0 - (1.0 - (TOP_LEAD * py).min(1.0)).powi(2);
    let top = skip_sliver(lerp(button.y - RING, panel.top(), lead), island_bottom);
    let bottom = lerp(button.y + RING, panel.bottom(), py).max(top);
    let rect = Rect::from_x_y_ranges(centre_x - half_x..=centre_x + half_x, top..=bottom);
    let grown = growth.grown();
    Shape {
        rect,
        radius: rounded(lerp(RING, crate::design::CORNER, grown), rect),
        ring_to_rule: smoothstep(0.0, 0.5, grown),
    }
}

/// Any other surface between its `source` and its finished `target`, its corners from
/// `radius.0` to `radius.1`.
pub(crate) fn from_rect(growth: &Growth, source: Rect, target: Rect, radius: (f32, f32)) -> Shape {
    let (px, py) = growth.progress();
    let rect = Rect::from_x_y_ranges(
        lerp(source.left(), target.left(), px)..=lerp(source.right(), target.right(), px),
        lerp(source.top(), target.top(), py)..=lerp(source.bottom(), target.bottom(), py),
    );
    let rect = Rect::from_min_size(rect.min, rect.size().max(Vec2::ZERO));
    Shape {
        rect,
        radius: rounded(lerp(radius.0, radius.1, growth.grown()), rect),
        ring_to_rule: 1.0,
    }
}

fn rounded(radius: f32, rect: Rect) -> f32 {
    radius
        .min(rect.width() / 2.0)
        .min(rect.height() / 2.0)
        .max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(growth: &mut Growth, seconds: f32) {
        for _ in 0..(seconds * 120.0) as usize {
            growth.step(1.0 / 120.0);
        }
    }

    #[test]
    fn a_panel_grows_out_of_its_button_and_finishes_as_the_panel() {
        let button = Pos2::new(1500.0, 32.0);
        let panel = Rect::from_min_size(Pos2::new(1300.0, 64.0), Vec2::new(320.0, 400.0));
        let mut growth = Growth::new(1, 0.0);
        let start = from_button(&growth, button, panel, 56.0);
        assert_eq!(start.rect.center().x, button.x);
        assert!(
            (start.radius - RING).abs() < 1e-3,
            "it starts as the ink ring"
        );
        assert_eq!(growth.reveal(), 0.0);
        run(&mut growth, 0.06);
        let early = from_button(&growth, button, panel, 56.0);
        assert!(
            early.rect.top() <= 55.0 || early.rect.top() >= 61.0,
            "no sliver"
        );
        assert_eq!(growth.reveal(), 0.0, "content waits for the shape");
        run(&mut growth, 1.0);
        assert!(growth.at_rest());
        let done = from_button(&growth, button, panel, 56.0);
        assert_eq!(done.rect, panel);
        assert_eq!(growth.reveal(), 1.0);
    }

    #[test]
    fn a_switch_retracts_fast_and_its_content_goes_at_once() {
        let mut old = Growth::new(1, 0.0);
        run(&mut old, 1.0);
        old.close(2, true);
        let mut tapped = Growth::new(2, crate::design::motion::SWITCH_DELAY);
        assert!(tapped.pending());
        run(&mut old, 0.1);
        run(&mut tapped, 0.1);
        assert_eq!(old.reveal(), 0.0, "faded within {RETRACT_FADE} s");
        assert!(old.grown() < tapped.grown() || old.grown() < 0.2);
        run(&mut old, 0.3);
        assert!(old.gone());
        assert!(!tapped.at_rest(), "the tapped panel finishes last");
    }

    #[test]
    fn a_reopened_surface_goes_on_from_where_it_was() {
        let mut growth = Growth::new(1, 0.0);
        run(&mut growth, 1.0);
        growth.close(2, false);
        run(&mut growth, 0.05);
        let part = growth.grown();
        assert!(part > 0.0 && part < 1.0);
        growth.reopen(3);
        growth.step(0.0);
        assert_eq!(growth.grown(), part);
        let source = Rect::from_min_size(Pos2::ZERO, Vec2::splat(10.0));
        let target = Rect::from_min_size(Pos2::new(100.0, 100.0), Vec2::splat(200.0));
        run(&mut growth, 1.0);
        assert_eq!(from_rect(&growth, source, target, (5.0, 10.0)).rect, target);
    }
}
