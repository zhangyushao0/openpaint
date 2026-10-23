//! `bench ui`: the interface's motions as a hand makes them, each measured as the display shows
//! it (`motion.rs`). Every island's popover opened by a finger tap on its button and closed by a
//! finger tap on the canvas outside it: the colour, brush and layer panels twice (the first
//! opening makes their textures and pictures), Actions and the Adjustments menu once, the smudge's
//! and the eraser's after the tap choosing the tool; the colour and brush panels opened by the pen,
//! which then hovers; a switch from the colour panel to the brushes; the colour wheel's square
//! dragged round, the brush size slider dragged up and down; the brush editor opened from the
//! chosen brush's row, its column scrolled down and up, and closed. Between motions the frames
//! still wanted, then a pause as the hand moves on. The toolbox is left as it was found.

use std::time::{Duration, Instant};

use openpaint_app::{Intent, Tool, ToolIntent};

use super::Watch;
use super::motion::{Gesture, drag, hovering, tap};
use crate::error::RuntimeError;
use crate::frame::Runtime;
use crate::input::ToolKind;
use crate::storage::Job;

/// Between motions, a hand moving on.
const PAUSE: Duration = Duration::from_millis(400);
/// The longest the frames still wanted after a motion are waited for.
const REST: Duration = Duration::from_secs(3);
/// The pen hovering after its tap, ms.
const HOVER_MS: i64 = 500;
/// A drag on the wheel or a slider, and a scroll of the editor's column, ms.
const DRAG_MS: i64 = 1000;
const SCROLL_MS: i64 = 400;
/// Screen pixels a scroll moves the column.
const SCROLL_PX: f32 = 600.0;

/// Where a popover lies: under the left island or the right one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Left,
    Right,
}

impl Runtime {
    /// `ui`: see the module.
    pub(crate) fn bench_ui(&mut self, lines: &mut Vec<String>) -> Result<(), RuntimeError> {
        self.bench_artwork()?;
        let found = self.toolbox.state();
        // The brush tool, so its button opens the brushes.
        let mut watch = Watch::paced(self);
        let brush = Intent::Tool(ToolIntent::Use(Tool::Brush));
        self.bench_intents(vec![brush], &mut watch)?;
        self.bench_settle()?;
        self.bench_rest()?;
        lines.push("frames drawn of the vsyncs wanting one (120 Hz); ms".to_owned());
        let finger = ToolKind::Finger;
        for (label, button) in [("colour", "Colour"), ("brushes", "Brushes")] {
            for when in ["first", "again"] {
                let label = format!("{label}, {when}");
                self.bench_popover(&label, (button, Side::Right), finger, lines)?;
            }
        }
        for when in ["first", "again"] {
            let label = format!("layers, {when}");
            self.bench_popover(&label, ("Layers", Side::Right), finger, lines)?;
        }
        self.bench_popover("actions", ("Actions", Side::Left), finger, lines)?;
        self.bench_popover("adjustments", ("Adjustments", Side::Left), finger, lines)?;
        for (label, button) in [("smudge", "Smudge"), ("eraser", "Eraser")] {
            let chosen = self.bench_tap(button, finger)?;
            lines.push(format!("{label} chosen: {chosen}"));
            self.bench_popover(label, (button, Side::Right), finger, lines)?;
        }
        let chosen = self.bench_tap("Brushes", finger)?;
        lines.push(format!("brushes chosen: {chosen}"));
        let pen = ToolKind::Stylus;
        for (label, button) in [
            ("colour by the pen", "Colour"),
            ("brushes by the pen", "Brushes"),
        ] {
            self.bench_popover(label, (button, Side::Right), pen, lines)?;
        }
        self.bench_panel_switch(lines)?;
        self.bench_dragged(lines)?;
        self.bench_editor(lines)?;
        // The toolbox as it was found, stored again.
        self.toolbox.restore(&found);
        self.storage.submit(Job::SaveEditor(Box::new(found)));
        self.bench_rest()
    }

    /// The popover of `button` on `side` opened by `tool`'s tap and closed by a finger's tap
    /// outside it: a line each.
    fn bench_popover(
        &mut self,
        label: &str,
        (button, side): (&'static str, Side),
        tool: ToolKind,
        lines: &mut Vec<String>,
    ) -> Result<(), RuntimeError> {
        let opens = self.bench_tap(button, tool)?;
        lines.push(format!("{label} opens: {opens}"));
        let closes = self.bench_away(side)?;
        lines.push(format!("{label} closes: {closes}"));
        Ok(())
    }

    /// A tap by `tool` on the control `name` (the pen hovering after it); the motion's line.
    fn bench_tap(&mut self, name: &'static str, tool: ToolKind) -> Result<String, RuntimeError> {
        let at = self.bench_control(name)?;
        let gesture = match tool {
            ToolKind::Stylus => hovering(tap(at, tool), HOVER_MS),
            _ => tap(at, tool),
        };
        self.bench_measured(gesture)
    }

    /// A finger's tap on the canvas away from a panel on `side`; the motion's line.
    fn bench_away(&mut self, side: Side) -> Result<String, RuntimeError> {
        let area = (self.shell.canvas_area()).ok_or(RuntimeError::NotNow("a canvas area"))?;
        let ([x, y], [w, h]) = (area.fit.origin, area.fit.size);
        let across = if side == Side::Right { 0.3 } else { 0.7 };
        let at = [x as f32 + across * w as f32, y as f32 + 0.85 * h as f32];
        self.bench_measured(tap(at, ToolKind::Finger))
    }

    /// `gesture` measured, then the frames it still wants and a pause; the motion's line.
    fn bench_measured(&mut self, gesture: Gesture) -> Result<String, RuntimeError> {
        let motion = self.bench_motion(gesture)?;
        self.bench_rest()?;
        Ok(motion.line())
    }

    /// The middle of the control `name` in the last frame.
    fn bench_control(&self, name: &'static str) -> Result<[f32; 2], RuntimeError> {
        let control = (self.controls().find(name)).map_err(|_| RuntimeError::NotNow(name))?;
        Ok(control.at([0.5, 0.5]))
    }

    /// Frames at the display's cadence while one is wanted (at most `REST`), then `PAUSE`.
    fn bench_rest(&mut self) -> Result<(), RuntimeError> {
        let mut watch = Watch::paced(self);
        let started = Instant::now();
        while self.bench_frame(&mut watch)? && started.elapsed() < REST {}
        std::thread::sleep(PAUSE);
        Ok(())
    }

    /// The colour panel open, then the brush button tapped: the switch's line.
    fn bench_panel_switch(&mut self, lines: &mut Vec<String>) -> Result<(), RuntimeError> {
        self.bench_tap("Colour", ToolKind::Finger)?;
        let switch = self.bench_tap("Brushes", ToolKind::Finger)?;
        lines.push(format!("colour to brushes: {switch}"));
        self.bench_away(Side::Right)?;
        Ok(())
    }

    /// The colour wheel's square dragged round inside the colour panel, and the brush size
    /// slider dragged up and down: a line each.
    fn bench_dragged(&mut self, lines: &mut Vec<String>) -> Result<(), RuntimeError> {
        self.bench_tap("Colour", ToolKind::Finger)?;
        let square = self.bench_rect("Saturation and value")?;
        let ([left, top], [width, height]) = square;
        let centre = [left + width / 2.0, top + height / 2.0];
        let round: Vec<[f32; 2]> = (0..=32)
            .map(|k| {
                let turn = k as f32 / 32.0 * std::f32::consts::TAU;
                let r = 0.3 * width.min(height);
                [centre[0] + r * turn.cos(), centre[1] + r * turn.sin()]
            })
            .collect();
        let wheel = self.bench_measured(drag(&round, DRAG_MS))?;
        lines.push(format!("colour wheel dragged: {wheel}"));
        self.bench_away(Side::Right)?;
        let ([left, top], [width, height]) = self.bench_rect("Brush size")?;
        let x = left + width / 2.0;
        let along = |share: f32| [x, top + share * height];
        let path = [along(0.5), along(0.2), along(0.8), along(0.5)];
        let slider = self.bench_measured(drag(&path, DRAG_MS))?;
        lines.push(format!("size slider dragged: {slider}"));
        Ok(())
    }

    /// The brush editor opened from the chosen brush's row, its column scrolled down and up by
    /// a finger on the words of its StreamLine row, and closed: a line each.
    fn bench_editor(&mut self, lines: &mut Vec<String>) -> Result<(), RuntimeError> {
        self.bench_tap("Brushes", ToolKind::Finger)?;
        let name = self.toolbox.projection().edited.name.clone();
        let row = (self.controls().find(&name))
            .map_err(|_| RuntimeError::NotNow("the chosen brush's row"))?;
        let opens = self.bench_measured(tap(row.at([0.5, 0.5]), ToolKind::Finger))?;
        lines.push(format!("brush editor opens: {opens}"));
        let ([left, top], [_, height]) = self.bench_rect("StreamLine")?;
        // The row's words lie left of its track and take no contact: the column scrolls.
        let words = left - 8.0 * openpaint_ui::design::SPACE * self.pixels_per_point;
        let start = [words, top + height / 2.0];
        let by = |dy: f32| [start[0], start[1] + dy];
        let down = self.bench_measured(drag(&[start, by(-SCROLL_PX)], SCROLL_MS))?;
        lines.push(format!("editor scrolled down: {down}"));
        let up = self.bench_measured(drag(&[start, by(SCROLL_PX)], SCROLL_MS))?;
        lines.push(format!("editor scrolled up: {up}"));
        let closes = self.bench_away(Side::Right)?;
        lines.push(format!("brush editor closes: {closes}"));
        Ok(())
    }

    /// The control `name`'s top-left corner and size in the last frame, screen pixels.
    fn bench_rect(&self, name: &'static str) -> Result<([f32; 2], [f32; 2]), RuntimeError> {
        let control = (self.controls().find(name)).map_err(|_| RuntimeError::NotNow(name))?;
        let [left, top, right, bottom] = control.rect;
        Ok(([left, top], [right - left, bottom - top]))
    }
}
