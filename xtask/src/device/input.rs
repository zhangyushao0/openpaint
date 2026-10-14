//! `cargo device-input`: one gesture or question through the debug build's
//! `GestureInstrumentation`, after waking the screen (an asleep screen has no window to touch):
//!
//! - `pen x,y,x,y,... [pressure [tilt]] [primary|secondary|tertiary|eraser]`: one pen path, a
//!   point per 8 ms (repeat a point to hold), leaning `tilt` degrees from upright (S44), with a
//!   side button held (from the tip outwards) or the eraser end (S41, S43);
//! - `pen-many count x,y,x,y,... [pressure]`: that pen path `count` times, each turned by a
//!   golden angle more about its box's middle and moved up to 140 px, so the strokes cross one
//!   area (S48: a layer holding many strokes);
//! - `pen-away x,y,x,y,... sleep|home`: that pen path, the pen still down at its end when the
//!   screen is turned off or home is pressed (S50: the stroke kept as drawn so far); the lift
//!   after it may be refused;
//! - `finger x,y[,x,y...] [x,y[,x,y...] [ms]]`: fingers down at the first points, moving to the
//!   second over `ms` (default 64), then lifting;
//! - `hover x,y [ms] [primary|secondary|tertiary|eraser]`: the pen held above the glass for
//!   `ms` (default 1500), as `pen` with a button or the eraser end;
//! - `tap <name>`: a finger tap on the middle of the control the app last drew under that name
//!   (D48's names; `name #n` for a shared one), looked up on the device by the debug build;
//! - `drag <name> x,y [ms]`: a finger from that control's middle to `x,y` over `ms` (default
//!   64), then lifting (S52: a drop from the colour button);
//! - `find <name>`: no gesture; where that control lies, or every name on screen (S52);
//! - `state`: no gesture; what the app shows (canvas or gallery, the cards' artwork ids
//!   changed last first, where a canvas fits) and every control's name (S52, `smoke.rs`);
//! - `gradient on|off`: no gesture; the screen's precision check (S37) over everything, or off:
//!   a soft grey ramp from sRGB 100 to 120, its upper half at 8 bits, its lower at the window's
//!   ten;
//! - `lose-device`: no gesture; the GPU device destroyed under the app as a driver reset loses it
//!   (S49), so the app makes its renderer again;
//! - `bench [scene]`: no gesture; the evaluator's micro-benchmark on the device's GPU (S55), every
//!   scene or the one named, a line per scene and scale (also in the app's log).

use super::adb::{Adb, DeviceError, Result};
use super::check::wake;

const INSTRUMENTATION: &str = "dev.openpaint/.GestureInstrumentation";

pub fn input(args: &[String]) -> Result<()> {
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let extras = extras(&args)?;
    let adb = Adb::connected()?;
    wake(&adb)?;
    let output = instrument(&adb, &extras)?;
    if let Some(state) = answer(&output, "state") {
        state.split('\t').for_each(|field| println!("  {field}"));
    } else if let Some(lines) = answer(&output, "bench") {
        lines.split('\t').for_each(|line| println!("{line}"));
    } else if let Some(found) = answer(&output, "found") {
        println!("found {found}");
    } else if let Some(tapped) = answer(&output, "tapped") {
        println!("tapped {tapped}");
    } else {
        println!("injected {}", args.join(" "));
    }
    Ok(())
}

/// Runs the gesture instrumentation once with `extras`; its output, once it says the gesture
/// was injected or the question answered.
pub(super) fn instrument(adb: &Adb, extras: &[(&'static str, String)]) -> Result<String> {
    let mut words = vec!["am", "instrument", "-w", "--no-restart"];
    for (key, value) in extras {
        words.extend(["-e", key, value.as_str()]);
    }
    words.push(INSTRUMENTATION);
    let output = adb.shell(&words)?;
    if !output.contains("gesture=injected") && !output.contains("gesture=answered") {
        held_pointers(adb);
        return Err(DeviceError::Injection(output.trim().to_owned()));
    }
    Ok(output)
}

/// What the instrumentation's result holds under `key` (one line).
pub(super) fn answer<'a>(output: &'a str, key: &str) -> Option<&'a str> {
    output.lines().find_map(|line| {
        let value = line
            .strip_prefix("INSTRUMENTATION_RESULT: ")?
            .strip_prefix(key)?;
        value.strip_prefix('=').map(str::trim_end)
    })
}

/// After a refused gesture: the pointers the input dispatcher still holds per window (its
/// `TouchStates`), read-only. Injected events all come from device -1, and a gesture cut off
/// (an instrumentation ended by another) leaves its pointers there (S38).
fn held_pointers(adb: &Adb) {
    let Ok(dump) = adb.shell(&["dumpsys", "-t", "10", "input"]) else {
        return;
    };
    let held = dump
        .lines()
        .skip_while(|line| !line.trim_start().starts_with("TouchStates"))
        .take_while(|line| !line.trim_start().starts_with("Display:"));
    println!("the input dispatcher holds:");
    for line in held {
        println!("  {}", line.trim());
    }
}

const INPUT_USAGE: &str = "cargo device-input \
                           pen x,y,... [pressure [tilt degrees]] [primary|secondary|tertiary|eraser] \
                           | pen-many count x,y,... [pressure] \
                           | pen-away x,y,... sleep|home \
                           | finger x,y[,...] [x,y[,...] [ms]] \
                           | hover x,y [ms] [primary|secondary|tertiary|eraser] \
                           | tap <control name> | drag <control name> x,y [ms] \
                           | find <control name> | state | gradient on|off | lose-device \
                           | bench [scene]";

/// A control's name for the device's shell, which reads the line: in single quotes, so none
/// inside.
fn quoted(name: &str) -> Result<String> {
    if name.is_empty() || name.chars().any(|c| c == '\'' || c.is_control()) {
        return Err(DeviceError::Usage(INPUT_USAGE));
    }
    Ok(format!("'{name}'"))
}

/// Only digits, points, commas and a sign reach the device's shell.
fn numbers(arg: &str) -> bool {
    arg.bytes()
        .all(|b| b.is_ascii_digit() || b".,-".contains(&b))
}

/// The instrumentation's `-e` arguments for a `device-input` command line.
pub(super) fn extras(args: &[&str]) -> Result<Vec<(&'static str, String)>> {
    let one = |key, value: String| Ok(vec![(key, value)]);
    match args {
        ["tap", name] => return one("tap", quoted(name)?),
        ["find", name] => return one("find", quoted(name)?),
        ["state"] => return one("state", "1".to_owned()),
        ["gradient", state @ ("on" | "off")] => return one("gradient", (*state).to_owned()),
        ["lose-device"] => return one("lose", "device".to_owned()),
        ["bench"] => return one("bench", "all".to_owned()),
        ["bench", scene] => return one("bench", quoted(scene)?),
        ["drag", name, to, rest @ ..]
            if rest.len() <= 1 && args[2..].iter().all(|a| numbers(a)) =>
        {
            let mut extras = vec![("drag", quoted(name)?), ("to", (*to).to_owned())];
            extras.extend(rest.iter().map(|ms| ("ms", (*ms).to_owned())));
            return Ok(extras);
        }
        _ => {}
    }
    // S50: the pen still down at the path's end when the screen goes off or home is pressed.
    if let ["pen-away", path, away @ ("sleep" | "home")] = args {
        let mut extras = gesture_extras(&["pen", path])?;
        extras.push(("away", (*away).to_owned()));
        return Ok(extras);
    }
    // A pen or hover line may end with the pen end or side button it uses (S41).
    let (words, end) = match args {
        [
            "pen" | "hover",
            ..,
            end @ ("primary" | "secondary" | "tertiary" | "eraser"),
        ] => (&args[..args.len() - 1], Some(*end)),
        _ => (args, None),
    };
    let mut extras = gesture_extras(words)?;
    match end {
        Some("eraser") => {
            extras.retain(|(key, _)| *key != "tool");
            extras.insert(0, ("tool", "eraser".to_owned()));
        }
        Some(button) => extras.push(("button", button.to_owned())),
        None => {}
    }
    Ok(extras)
}

/// The instrumentation's `-e` arguments for a pen, finger or hover line without its end.
fn gesture_extras(args: &[&str]) -> Result<Vec<(&'static str, String)>> {
    let usage = DeviceError::Usage(INPUT_USAGE);
    if !args[1.min(args.len())..].iter().all(|arg| numbers(arg)) {
        return Err(usage);
    }
    let extras: Vec<(&'static str, &str)> = match *args {
        ["pen", path] => vec![("tool", "stylus"), ("path", path)],
        ["pen", path, pressure] => vec![("tool", "stylus"), ("pressure", pressure), ("path", path)],
        // S44: the pen leaning that many degrees from upright.
        ["pen", path, pressure, tilt] => vec![
            ("tool", "stylus"),
            ("pressure", pressure),
            ("tilt", tilt),
            ("path", path),
        ],
        // S48: the path `count` times, each turned and moved (strokes crossing one area).
        ["pen-many", count, path] => vec![("tool", "stylus"), ("repeat", count), ("path", path)],
        ["pen-many", count, path, pressure] => vec![
            ("tool", "stylus"),
            ("repeat", count),
            ("pressure", pressure),
            ("path", path),
        ],
        ["finger", from] => vec![("from", from)],
        ["finger", from, to] => vec![("from", from), ("to", to)],
        ["finger", from, to, ms] => vec![("from", from), ("to", to), ("ms", ms)],
        ["hover", at] => vec![("hover", at)],
        ["hover", at, ms] => vec![("hover", at), ("ms", ms)],
        _ => return Err(usage),
    };
    Ok(extras
        .into_iter()
        .map(|(key, value)| (key, value.to_owned()))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_lines_become_instrumentation_arguments() {
        let pen = extras(&["pen", "100,200,300,400", "0.8"]).expect("pen");
        assert_eq!(
            pen,
            [
                ("tool", "stylus".to_owned()),
                ("pressure", "0.8".to_owned()),
                ("path", "100,200,300,400".to_owned())
            ]
        );
        let fingers = extras(&["finger", "1,2,3,4", "0,2,5,4", "200"]).expect("fingers");
        assert_eq!(fingers[2], ("ms", "200".to_owned()));
        let tap = extras(&["tap", "Move layer up"]).expect("tap");
        assert_eq!(tap, [("tap", "'Move layer up'".to_owned())]);
        let lose = extras(&["lose-device"]).expect("lose");
        assert_eq!(lose, [("lose", "device".to_owned())]);
        let bench = extras(&["bench"]).expect("bench");
        assert_eq!(bench, [("bench", "all".to_owned())]);
        let speckle = extras(&["bench", "speckle"]).expect("one scene");
        assert_eq!(speckle, [("bench", "'speckle'".to_owned())]);
        assert!(extras(&["tap", "it's"]).is_err());
        let gradient = extras(&["gradient", "on"]).expect("gradient");
        assert_eq!(gradient, [("gradient", "on".to_owned())]);
        assert!(extras(&["gradient", "maybe"]).is_err());
        assert!(extras(&["tap", "a\nb"]).is_err());
        assert!(extras(&["pen", "1,2;rm -rf /"]).is_err());
        let primary = extras(&["pen", "1,2,3,4", "0.5", "primary"]).expect("button");
        assert_eq!(primary.last(), Some(&("button", "primary".to_owned())));
        assert_eq!(primary[0], ("tool", "stylus".to_owned()));
        let tilted = extras(&["pen", "1,2,3,4", "0.5", "70", "primary"]).expect("tilted");
        assert_eq!(tilted[2], ("tilt", "70".to_owned()));
        assert_eq!(tilted.len(), 5, "{tilted:?}");
        let eraser = extras(&["pen", "1,2,3,4", "eraser"]).expect("eraser");
        assert_eq!(eraser[0], ("tool", "eraser".to_owned()));
        assert_eq!(eraser.len(), 2, "{eraser:?}");
        let hover = extras(&["hover", "5,6", "800", "secondary"]).expect("hover");
        assert_eq!(hover[2], ("button", "secondary".to_owned()));
        let third = extras(&["hover", "5,6", "tertiary"]).expect("third");
        assert_eq!(
            third,
            [
                ("hover", "5,6".to_owned()),
                ("button", "tertiary".to_owned())
            ]
        );
        let many = extras(&["pen-many", "300", "1,2,3,4", "0.8"]).expect("many");
        assert_eq!(many[1], ("repeat", "300".to_owned()));
        assert_eq!(many.len(), 4, "{many:?}");
        assert!(extras(&["pen-many", "1,2,3,4"]).is_err());
        let away = extras(&["pen-away", "1,2,3,4", "sleep"]).expect("away");
        assert_eq!(away[2], ("away", "sleep".to_owned()));
        assert!(extras(&["pen-away", "1,2;3", "home"]).is_err());
        assert!(extras(&["pen-away", "1,2,3,4", "power"]).is_err());
        assert!(extras(&["finger", "1,2", "primary"]).is_err());
        assert!(extras(&["walk", "1,2"]).is_err());
        assert!(extras(&[]).is_err());
    }

    #[test]
    fn drags_finds_and_the_state_question() {
        let drag = extras(&["drag", "Colour", "900,1200", "240"]).expect("drag");
        assert_eq!(
            drag,
            [
                ("drag", "'Colour'".to_owned()),
                ("to", "900,1200".to_owned()),
                ("ms", "240".to_owned())
            ]
        );
        assert_eq!(
            extras(&["drag", "Colour", "9,12"]).ok().map(|e| e.len()),
            Some(2)
        );
        assert!(extras(&["drag", "Colour", "9;12"]).is_err());
        assert!(extras(&["drag", "it's", "9,12"]).is_err());
        assert!(extras(&["drag", "Colour", "9,12", "40", "1"]).is_err());
        let find = extras(&["find", "Open artwork #1"]).expect("find");
        assert_eq!(find, [("find", "'Open artwork #1'".to_owned())]);
        assert_eq!(
            extras(&["state"]).expect("state"),
            [("state", "1".to_owned())]
        );
        let output = "INSTRUMENTATION_RESULT: found=Undo at 40,800\r\n\
                      INSTRUMENTATION_RESULT: gesture=answered\r\nINSTRUMENTATION_CODE: -1\r\n";
        assert_eq!(answer(output, "found"), Some("Undo at 40,800"));
        assert_eq!(answer(output, "gesture"), Some("answered"));
        assert_eq!(answer(output, "state"), None);
    }
}
