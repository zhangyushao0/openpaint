//! The control map: every control on screen by its accessible name and where it lies, filled by
//! the shared components as they lay themselves out in each pass. It names what the hand can
//! reach without coordinates (accessibility metadata, and the targets of scenario steps), so a
//! layout change never reaches what refers to a control by name. Screens add nothing: the name
//! is the label the component is drawn with.
//!
//! Each pass: `begin`, the components `register`, then `take`. A closing panel's controls take
//! no contact, so its layer is `mute`d for the pass.

use egui::{Id, LayerId, Rect};

/// Where the map is kept in egui's memory during a pass.
const STATE: &str = "control map";

/// One control: its name and its rectangle in screen pixels, `[left, top, right, bottom]`.
#[derive(Debug, Clone, PartialEq)]
pub struct Control {
    pub name: String,
    pub rect: [f32; 4],
}

impl Control {
    /// True when the screen point `[x, y]` (pixels) lies inside.
    #[must_use]
    pub fn contains(&self, [x, y]: [f32; 2]) -> bool {
        let [left, top, right, bottom] = self.rect;
        (left..right).contains(&x) && (top..bottom).contains(&y)
    }

    /// The point at `fraction` of its width and height from its top-left corner (`[0.5, 0.5]`
    /// is the middle), in pixels.
    #[must_use]
    pub fn at(&self, [fx, fy]: [f32; 2]) -> [f32; 2] {
        let [left, top, right, bottom] = self.rect;
        [left + fx * (right - left), top + fy * (bottom - top)]
    }
}

/// The controls of the last pass in the order they were drawn (later ones lie on top). A name
/// shared by several controls (a recent colour, a card's bin) is told apart as `name #n`, the
/// n-th drawn, from 1.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ControlMap {
    controls: Vec<Control>,
    /// Layers whose controls take no contact in this pass.
    muted: Vec<LayerId>,
}

/// Why a name finds no single control.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unresolved {
    /// No control has this name now.
    Missing,
    /// Several have it; `name #1` to `name #count` tell them apart.
    Shared { count: usize },
}

impl ControlMap {
    /// Every control, in drawing order.
    pub fn iter(&self) -> impl Iterator<Item = &Control> {
        self.controls.iter()
    }

    /// The control called `name`, or `name #n` for the n-th of those sharing a name.
    pub fn find(&self, name: &str) -> Result<&Control, Unresolved> {
        if let Some((base, index)) = name.rsplit_once(" #")
            && let Ok(index) = index.parse::<usize>()
        {
            return self
                .controls
                .iter()
                .filter(|c| c.name == base)
                .nth(index.wrapping_sub(1))
                .ok_or(Unresolved::Missing);
        }
        let mut found = self.controls.iter().filter(|c| c.name == name);
        let first = found.next();
        match (first, 1 + found.count()) {
            (Some(control), 1) => Ok(control),
            (None, _) => Err(Unresolved::Missing),
            (Some(_), count) => Err(Unresolved::Shared { count }),
        }
    }

    /// The topmost control under the screen point `at` (pixels), with how it is named: its
    /// name, or `name #n` when others share it.
    #[must_use]
    pub fn under(&self, at: [f32; 2]) -> Option<(String, &Control)> {
        let (index, control) = self
            .controls
            .iter()
            .enumerate()
            .rev()
            .find(|(_, control)| control.contains(at))?;
        Some((self.called(index), control))
    }

    /// Every name a step can use now, sorted.
    #[must_use]
    pub fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = (0..self.controls.len()).map(|i| self.called(i)).collect();
        names.sort();
        names
    }

    /// How the control at `index` is named: its name, or `name #n` when others share it.
    fn called(&self, index: usize) -> String {
        let name = &self.controls[index].name;
        let same = |c: &&Control| c.name == *name;
        if self.controls.iter().filter(same).count() == 1 {
            return name.clone();
        }
        let before = self.controls[..index].iter().filter(same).count();
        format!("{name} #{}", before + 1)
    }
}

fn state<R>(ctx: &egui::Context, f: impl FnOnce(&mut ControlMap) -> R) -> R {
    ctx.data_mut(|data| f(data.get_temp_mut_or_default::<ControlMap>(Id::new(STATE))))
}

/// Starts a pass with no controls.
pub(crate) fn begin(ctx: &egui::Context) {
    state(ctx, |map| *map = ControlMap::default());
}

/// The controls on `layer` take no contact in this pass (a closing panel).
pub(crate) fn mute(ctx: &egui::Context, layer: LayerId) {
    state(ctx, |map| map.muted.push(layer));
}

/// The control `name` lies at `rect` (points) in `ui`: kept where it shows, unless its layer is
/// muted or this is a sizing pass that shows nothing.
pub(crate) fn register(ui: &egui::Ui, name: &str, rect: Rect) {
    let shown = rect.intersect(ui.clip_rect());
    if ui.is_sizing_pass() || !shown.is_positive() {
        return;
    }
    let ppp = ui.ctx().pixels_per_point();
    let layer = ui.layer_id();
    state(ui.ctx(), |map| {
        if !map.muted.contains(&layer) {
            map.controls.push(Control {
                name: name.to_owned(),
                rect: [shown.min.x, shown.min.y, shown.max.x, shown.max.y].map(|side| side * ppp),
            });
        }
    });
}

/// The pass's map, once every component is laid out.
pub(crate) fn take(ctx: &egui::Context) -> ControlMap {
    state(ctx, std::mem::take)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(names: &[&str]) -> ControlMap {
        let controls = names
            .iter()
            .zip(0u8..)
            .map(|(name, i)| Control {
                name: (*name).to_owned(),
                rect: [f32::from(i) * 10.0, 0.0, f32::from(i) * 10.0 + 10.0, 10.0],
            })
            .collect();
        ControlMap {
            controls,
            muted: Vec::new(),
        }
    }

    #[test]
    fn names_find_one_control_and_shared_names_take_a_number() {
        let map = map(&["Undo", "Recent colour", "Recent colour"]);
        assert_eq!(map.find("Undo").map(|c| c.rect[0]), Ok(0.0));
        assert_eq!(map.find("Recent colour #2").map(|c| c.rect[0]), Ok(20.0));
        assert_eq!(
            map.find("Recent colour"),
            Err(Unresolved::Shared { count: 2 })
        );
        assert_eq!(map.find("Recent colour #3"), Err(Unresolved::Missing));
        assert_eq!(map.find("Redo"), Err(Unresolved::Missing));
        assert_eq!(
            map.names(),
            ["Recent colour #1", "Recent colour #2", "Undo"]
        );
        let (name, _) = map.under([25.0, 5.0]).expect("a control");
        assert_eq!(name, "Recent colour #2");
        assert!(map.under([35.0, 5.0]).is_none());
    }

    #[test]
    fn a_point_is_taken_from_the_rectangle_by_fraction() {
        let control = Control {
            name: "Brush size".into(),
            rect: [10.0, 100.0, 20.0, 300.0],
        };
        assert_eq!(control.at([0.5, 0.5]), [15.0, 200.0]);
        assert_eq!(control.at([0.0, 0.25]), [10.0, 150.0]);
        assert!(control.contains([10.0, 100.0]) && !control.contains([20.0, 100.0]));
    }
}
