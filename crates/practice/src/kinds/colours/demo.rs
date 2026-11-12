//! A colour's demonstration (`demos/demo.rs`): the wheel's ring to the hue's family, then lighter
//! or darker, then purer or greyer, the frame filled, then compared.

use crate::demos::demo::{Demo, DemoLayout, Drawn, Marker, On, Script, band, rect};
use crate::kinds::colours::colour_match::{Lightness, Purity, Tone};
use crate::maths::colour::{from_hsv, hsv};
use crate::sheets::exercise::Family;

/// The example's seed: never an exercise's but by a chance of one in 2^64.
const COLOUR_SEED: u64 = 0xDE30_5EED_0004;

/// A single colour.
pub(crate) fn colour() -> Demo {
    let tone = Tone {
        purity: Purity::Muted,
        lightness: Lightness::Middle,
    };
    let original = tone.colour(COLOUR_SEED);
    let [h, s, v] = hsv(original);
    let mut script = Script::new();
    let whole = rect([0.0, 0.0], [1.0, 1.0]);
    script.given(On::Original, Marker::Given, band(whole.clone(), original));
    script.given(
        On::Wheel,
        Marker::Given,
        Drawn::Pick([(h + 140.0) % 360.0, 0.85, 0.9]),
    );
    // V大's order: the hue first, then lightness, then purity. A first pick by eye a little off,
    // as a hand makes it: its hue a step round, a little darker and purer.
    let hue = (h + 12.0) % 360.0;
    let picks = [
        [hue, 0.85, 0.9],
        [hue, 0.85, v - 0.06],
        [hue, s + 0.07, v - 0.06],
    ];
    for pick in picks {
        script.step();
        script.draw(On::Wheel, Marker::Ink, Drawn::Pick(pick));
    }
    let picked = from_hsv(picks[2]);
    script.step();
    script.draw(On::Copy, Marker::Ink, band(whole, picked));
    let patch = rect([0.25, 0.25], [0.75, 0.75]);
    let compared = vec![
        (On::Copy, Marker::Given, band(patch.clone(), original)),
        (On::Original, Marker::Given, band(patch, picked)),
        (On::Wheel, Marker::Answer, Drawn::Ringed([h, s, v])),
    ];
    script.compare(Family::Colour, DemoLayout::Wheel, compared)
}
