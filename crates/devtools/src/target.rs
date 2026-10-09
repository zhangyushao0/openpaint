//! Where a step acts: a control by its name, as the UI laid it out in the last frame
//! (`ControlMap`), or a point on the canvas. Steps name the controls they touch, so a layout
//! change never touches a scenario file; only places on the canvas are window pixels.
//!
//! ```json
//! { "tap": { "at": "Undo" } }
//! { "drag": { "from": "Opacity", "to": { "on": "Opacity", "fraction": [0.25, 0.5] } } }
//! { "tap": { "at": { "on": "Saturation and value", "offset": [-120, 180] } } }
//! { "press": { "path": [{ "at": "Colour", "ms": 0 }, [1200, 900, 400]] } }
//! ```

use openpaint_runtime::{Control, ControlMap};
use serde::Deserialize;

use crate::error::ScenarioError;

/// A place on the screen.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum Spot {
    /// Window pixels `[x, y]`: a place on the canvas, where no control lies.
    Point([f32; 2]),
    /// The middle of the control of this name.
    Named(String),
    /// A place on a control.
    On(Place),
}

/// A place on the control `on`: `fraction` of its width and height from its top-left corner
/// (default the middle), then `offset` pixels further.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Place {
    pub on: String,
    #[serde(default = "middle")]
    pub fraction: [f32; 2],
    #[serde(default)]
    pub offset: [f32; 2],
}

/// One key of a press: `[x, y, ms]` on the canvas, or a spot at `ms` from touching down.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum Key {
    Point([f32; 3]),
    At { at: Spot, ms: f32 },
}

impl Key {
    /// Milliseconds from touching down.
    #[must_use]
    pub fn ms(&self) -> f32 {
        match self {
            Self::Point([_, _, ms]) | Self::At { ms, .. } => *ms,
        }
    }
}

fn middle() -> [f32; 2] {
    [0.5, 0.5]
}

/// `spot` in window pixels, from the controls laid out now.
pub(crate) fn resolve(map: &ControlMap, spot: &Spot) -> Result<[f32; 2], ScenarioError> {
    let (name, fraction, offset) = match spot {
        Spot::Point(point) => return Ok(*point),
        Spot::Named(name) => (name, middle(), [0.0, 0.0]),
        Spot::On(place) => (&place.on, place.fraction, place.offset),
    };
    let control = map.find(name).map_err(|reason| ScenarioError::Control {
        name: name.clone(),
        reason,
        available: map.names(),
    })?;
    let [x, y] = control.at(fraction);
    Ok([x + offset[0], y + offset[1]])
}

/// Every name of `shown` on screen, in reading order (by the top of its rectangle, then its
/// left), and none of `absent`; else what is wrong, with every name on screen.
pub(crate) fn expect(
    map: &ControlMap,
    (shown, absent): (&[String], &[String]),
) -> Result<(), ScenarioError> {
    let unexpected = |what: String| ScenarioError::Unexpected {
        what,
        available: map.names(),
    };
    let mut last: Option<(&String, [f32; 2])> = None;
    for name in shown {
        let control = map.find(name).map_err(|reason| ScenarioError::Control {
            name: name.clone(),
            reason,
            available: map.names(),
        })?;
        let [left, top, ..] = control.rect;
        if let Some((before, [above, after])) = last
            && (top, left) <= (above, after)
        {
            return Err(unexpected(format!("{name:?} shows before {before:?}")));
        }
        last = Some((name, [top, left]));
    }
    match absent.iter().find(|name| map.find(name).is_ok()) {
        Some(name) => Err(unexpected(format!("{name:?} shows"))),
        None => Ok(()),
    }
}

/// For the window point `at` in a step: the control under it, as the place the step should
/// name instead.
pub(crate) fn named_instead(map: &ControlMap, at: [f32; 2]) -> Option<String> {
    let (name, control) = map.under(at)?;
    Some(format!("{at:?} is on {}", place(&name, control, at)))
}

/// The spot that names `at` on `control`: its name alone at its middle, else with the offset.
fn place(name: &str, control: &Control, [x, y]: [f32; 2]) -> String {
    let [cx, cy] = control.at(middle());
    let offset = [x - cx, y - cy];
    if offset == [0.0, 0.0] {
        format!("{name:?}")
    } else {
        format!(r#"{{ "on": {name:?}, "offset": {offset:?} }}"#)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spots_and_keys_parse_as_points_names_or_places() {
        let spots: Vec<Spot> = serde_json::from_str(
            r#"[[1, 2], "Undo", { "on": "Opacity", "fraction": [0.25, 0.5] },
                { "on": "Colour wheel", "offset": [-3.5, 4] }]"#,
        )
        .expect("spots");
        assert_eq!(spots[0], Spot::Point([1.0, 2.0]));
        assert_eq!(spots[1], Spot::Named("Undo".into()));
        let Spot::On(place) = &spots[3] else {
            panic!("a place")
        };
        assert_eq!((place.fraction, place.offset), ([0.5, 0.5], [-3.5, 4.0]));
        let keys: Vec<Key> =
            serde_json::from_str(r#"[[1, 2, 0], { "at": "Colour", "ms": 40 }]"#).expect("keys");
        assert_eq!(keys[1].ms(), 40.0);
        assert!(serde_json::from_str::<Spot>(r#"{ "on": "Undo", "push": 1 }"#).is_err());
    }
}
