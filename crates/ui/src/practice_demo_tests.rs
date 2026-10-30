use openpaint_practice::CELLS;

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
    let english = text::Language::English;
    for family in Family::ALL {
        let how = text::how(family);
        let steps = text::how_steps(family, CELLS, english);
        match Demo::of(family) {
            Some(demo) => assert_eq!(steps.len(), demo.steps.len(), "{family:?}"),
            None => assert!(steps.is_empty() && how.trains.is_none(), "{family:?}"),
        }
    }
    // A value scale's How follows the row in hand (S97's review): of five, three steps naming its
    // cells; of nine, four.
    let five = text::how_steps(Family::Scale, 5, english);
    assert_eq!(five.len(), Demo::of_scale(5).steps.len());
    assert_eq!(
        five,
        [
            "The middle cell first (the 3rd): halfway in lightness",
            "Then the middle of each half (the 2nd and 4th)",
            "Compare"
        ]
    );
    let nine = text::how_steps(Family::Scale, 9, text::Language::Chinese);
    assert_eq!(nine[0], "先填正中一格（第 5 格）：明度取两端的正中");
    assert_eq!(nine[1], "再填两半各自的正中（第 3、7 格）");
    assert_eq!(nine[2], "最后填剩下的格子（第 2、4、6、8 格）");
}

#[test]
fn a_scale_s_how_plays_the_count_of_the_row_in_hand() {
    let ctx = egui::Context::default();
    let mut demos = Demonstrations::default();
    for cells in [5, 9, 5] {
        let of = HowOf::new(Family::Scale, cells);
        let steps = text::how_steps(Family::Scale, cells, text::Language::English);
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| demos.show(ui, of, &steps));
        output.textures_delta.clear();
        let playing = demos.playing.as_ref().expect("a demonstration");
        assert_eq!(*playing.demo, Demo::of_scale(cells), "{cells}");
    }
    assert_eq!(demos.made.len(), 2, "each count made once");
    let line = HowOf::new(Family::Line, 5);
    assert_eq!(line, HowOf::new(Family::Line, 9), "other kinds alike");
}

/// One pass at `time` seconds, the card showing `family`'s demonstration when `shown` (and Again
/// tapped when `again`); whether frames are wanted after it.
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
            let steps = text::how_steps(family, CELLS, text::Language::English);
            demos.show(ui, HowOf::new(family, CELLS), &steps);
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
