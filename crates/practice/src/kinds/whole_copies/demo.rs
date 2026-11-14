//! A whole copy's demonstration (`demos/demo.rs`, S95), V大's way on the shared landscape beside
//! its picture (`demos/demo_landscape.rs`): the division red and green; Draw, the division dimming
//! under the big shapes' lines in ink; Compare lays the picture over the copy, the division gone.

use crate::compositions::levels::Level;
use crate::demos::demo::{Demo, DemoLayout, Drawn, Marker, On, Script, wobbled};
use crate::demos::demo_landscape::{Landscape, STRAY, TOGETHER};
use crate::maths::path::Point;

/// A whole copy.
pub(crate) fn whole_copy() -> Demo {
    let example = Landscape::new();
    let mut script = Script::new();
    example.given(&mut script);
    script.step();
    example.main_lines(&mut script);
    example.level_lines(&mut script, Level::Second);
    script.step();
    let drawing: Vec<Vec<Point>> = (example.masses.drawing.iter())
        .map(|line| line.iter().map(|p| example.at(*p)).collect())
        .collect();
    for (turn, group) in drawing.chunks(TOGETHER).enumerate() {
        let marks = (group.iter())
            .map(|line| {
                let drawn = wobbled(line, STRAY, 2.1 + turn as f64);
                (On::Copy, Marker::Ink, Drawn::Line(drawn))
            })
            .collect();
        script.together(marks);
    }
    let mut demo = script.compare(DemoLayout::Pair, vec![example.laid(Marker::Faint)]);
    demo.dimmed = match demo.steps[..] {
        [_, draw, compare] => Some([draw, compare]),
        _ => None,
    };
    demo.view = Some(example.rect());
    demo.masses = Some(Box::new(example.masses));
    demo
}
