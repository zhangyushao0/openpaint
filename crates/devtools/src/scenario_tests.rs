//! Scenario files: every step kind parses, unknown steps and fields are refused, names and
//! hashes are checked.

use super::*;

#[test]
fn every_step_kind_parses() {
    let text = r#"{
        "walkthrough": ["Open the app."],
        "screen": [640, 400],
        "density": 2,
        "steps": [
            { "tap": { "at": [10, 20] } },
            { "drag": { "from": [1, 2], "to": [3, 4], "tool": "stylus" } },
            { "stroke": { "points": [[100, 200, 0.5]] } },
            { "stroke": { "points": [[1, 2, 0.5]], "lift": [3, 4],
                          "palm": { "at": [5, 6], "lands": "during" } } },
            { "wait": { "frames": 2 } },
            "restart",
            { "checkpoint": { "name": "a", "canvas": "1f94cbfbd4b11e02" } },
            { "screenshot": { "name": "b" } },
            "storage_fails",
            "storage_works",
            { "type": "Harbour" },
            "back",
            { "system": "choose_file" },
            { "exported": { "name": "png" } }
        ]
    }"#;
    let scenario = Scenario::parse(text).expect("parses");
    assert_eq!(scenario.walkthrough, ["Open the app."]);
    assert_eq!(scenario.steps.len(), 14);
    assert_eq!(scenario.steps[12], Step::System(SystemAnswer::ChooseFile));
    let png = Step::Exported {
        name: "png".into(),
        scale: 1,
        pixels: None,
    };
    assert_eq!(scenario.steps[13], png);
    assert_eq!(scenario.steps[8], Step::StorageFails);
    assert_eq!(scenario.steps[9], Step::StorageWorks);
    assert_eq!(scenario.steps[10], Step::Type("Harbour".into()));
    assert_eq!(scenario.steps[11], Step::Back);
    assert!(matches!(
        scenario.steps[3],
        Step::Stroke { sparse: false, .. }
    ));
    assert_eq!(
        scenario.steps[0],
        Step::Tap {
            at: Spot::Point([10.0, 20.0]),
            tool: Tool::Finger
        }
    );
    assert_eq!(scenario.steps[5], Step::Restart);
    let checkpoint = Step::Checkpoint {
        name: "a".into(),
        canvas: 0x1f94_cbfb_d4b1_1e02,
    };
    assert_eq!(scenario.steps[6], checkpoint);
}

#[test]
fn finger_and_view_steps_parse() {
    let steps = r#"
        { "touch": { "from": [[1, 2], [3, 4]], "to": [[0, 2], [5, 4]], "ms": 200 } },
        { "touch": { "from": [[1, 2], [3, 4], [5, 6]] } },
        { "view": { "name": "zoomed", "zoom": 2.5, "rotation": -30 } }"#;
    let scenario = with_steps(steps).expect("parses");
    let points = |a: [f32; 2], b: [f32; 2]| vec![Spot::Point(a), Spot::Point(b)];
    let two = Step::Touch {
        from: points([1.0, 2.0], [3.0, 4.0]),
        to: Some(points([0.0, 2.0], [5.0, 4.0])),
        ms: 200,
    };
    assert_eq!(scenario.steps[0], two);
    assert!(matches!(
        &scenario.steps[1],
        Step::Touch {
            to: None,
            ms: 64,
            ..
        }
    ));
    let view = Step::View {
        name: "zoomed".into(),
        zoom: 2.5,
        rotation: -30.0,
        pan: None,
    };
    assert_eq!(scenario.steps[2], view);
    let uneven = r#"{ "touch": { "from": [[1, 2]], "to": [[1, 2], [3, 4]] } }"#;
    assert!(matches!(with_steps(uneven), Err(ScenarioError::Invalid(_))));
}

#[test]
fn an_export_step_takes_a_scale_and_a_pixel_hash() {
    let steps = r#"{ "exported": { "name": "png-4x", "scale": 4, "pixels": "00000000000000ff" } }"#;
    let scenario = with_steps(steps).expect("parses");
    let expected = Step::Exported {
        name: "png-4x".into(),
        scale: 4,
        pixels: Some(0xff),
    };
    assert_eq!(scenario.steps[0], expected);
}

fn with_steps(steps: &str) -> Result<Scenario, ScenarioError> {
    Scenario::parse(&format!(
        r#"{{ "screen": [8, 8], "density": 1, "steps": [{steps}] }}"#
    ))
}

#[test]
fn unknown_steps_and_fields_are_refused_with_a_position() {
    let error = with_steps(r#"{ "jump": {} }"#).expect_err("unknown step");
    assert!(error.to_string().contains("line 1"), "{error}");
    assert!(with_steps(r#"{ "tap": { "at": [1, 2], "force": 1 } }"#).is_err());
}

#[test]
fn hashes_are_sixteen_hex_digits() {
    assert!(with_steps(r#"{ "checkpoint": { "name": "a", "canvas": "1f94" } }"#).is_err());
    let bad = r#"{ "checkpoint": { "name": "a", "canvas": "1f94cbfbd4b11e0g" } }"#;
    assert!(with_steps(bad).is_err());
}

#[test]
fn names_are_plain_and_unique() {
    let upper = r#"{ "screenshot": { "name": "Final" } }"#;
    assert!(matches!(with_steps(upper), Err(ScenarioError::Invalid(_))));
    let twice = r#"{ "screenshot": { "name": "a" } }, { "screenshot": { "name": "a" } }"#;
    assert!(matches!(with_steps(twice), Err(ScenarioError::Invalid(_))));
    let empty = r#"{ "stroke": { "points": [] } }"#;
    assert!(matches!(with_steps(empty), Err(ScenarioError::Invalid(_))));
}

#[test]
fn a_stroke_step_takes_its_lift_palm_and_sparse_samples() {
    let text = r#"{
        "walkthrough": ["Draw."],
        "screen": [640, 400],
        "density": 2,
        "steps": [
            { "stroke": { "points": [[1, 2, 0.5]], "lift": [3, 4],
                          "palm": { "at": [5, 6], "lands": "during" }, "sparse": true, "hold_ms": 600,
                          "pen": "secondary" } },
            { "hover": { "at": [7, 8], "pen": "eraser" } },
            { "hover": { "at": [9, 10], "pen": "tertiary" } }
        ]
    }"#;
    let scenario = Scenario::parse(text).expect("parses");
    let stroke = Step::Stroke {
        points: vec![[1.0, 2.0, 0.5]],
        lift: Some([3.0, 4.0]),
        palm: Some(Palm {
            at: [5.0, 6.0],
            lands: Lands::During,
        }),
        sparse: true,
        hold_ms: 600,
        pen: Pen::Secondary,
    };
    let hover = Step::Hover {
        at: Spot::Point([7.0, 8.0]),
        pen: Pen::Eraser,
    };
    let third = Step::Hover {
        at: Spot::Point([9.0, 10.0]),
        pen: Pen::Tertiary,
    };
    assert_eq!(scenario.steps, [stroke, hover, third]);
}
