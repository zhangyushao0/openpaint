//! A division's demonstration (`demos/demo.rs`, S95), V大's way on the shared landscape beside its
//! picture (`demos/demo_landscape.rs`): the ridge's turning point found first, the two longest
//! lines red right through the picture; from them each part's frame green; shorter still blue;
//! Compare lays the picture faint under the lines.

use crate::compositions::levels::Level;
use crate::demos::demo::{Demo, DemoLayout, Drawn, Marker, On, Script};
use crate::demos::demo_landscape::Landscape;

/// A division.
pub(crate) fn division() -> Demo {
    let example = Landscape::new();
    let mut script = Script::new();
    example.given(&mut script);
    script.step();
    example.main_lines(&mut script);
    for level in [Level::Second, Level::Third] {
        script.step();
        example.level_lines(&mut script, level);
    }
    let mut compared = vec![example.laid(Marker::Faint)];
    for level in Level::ALL {
        let lines = example.lines(level).into_iter();
        compared.extend(lines.map(|line| (On::Copy, Marker::Level(level), Drawn::Line(line))));
    }
    let mut demo = script.compare(DemoLayout::Pair, compared);
    demo.view = Some(example.rect());
    demo.masses = Some(Box::new(example.masses));
    demo
}
