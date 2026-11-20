//! The video's side of the replay's screen at the tablet's size: the export panel's choices and
//! what each asks for, their greying, and the pills of the video being made, saved and failed.

use openpaint_app::{
    Availability, Delivery, Language, ReplayIntent, VideoError, VideoLength, VideoNotice, VideoSize,
};

use super::screen_tests::{PPP, Screen, asked, has, nothing_clipped, open, replay};
use crate::design::SPACE;

#[test]
fn export_video_opens_its_panel_and_its_choices_ask_for_what_they_say() {
    let projection = open(replay());
    let mut screen = Screen::new(Language::English);
    screen.settle(&projection);
    assert!(screen.tap(&projection, "Export video…").is_empty());
    let texts = screen.settle(&projection);
    for name in [
        "30 s highlight",
        "Full",
        "Segment",
        "1080p",
        "Original",
        "Stop on the finished picture",
        "Strokes and time",
        "Save to album",
        "Share…",
    ] {
        assert!(screen.map.find(name).is_ok(), "{name}");
    }
    for shown in [
        "Export video",
        "Original 2880 × 1800",
        "3:21",
        "0:52 – 1:40",
    ] {
        assert!(has(&texts, shown), "{shown} in {texts:?}");
    }
    nothing_clipped(&screen.map, &texts);
    // A switch's words end well before its switch (five spaces wide, two in from the row's end).
    for name in ["Stop on the finished picture", "Strokes and time"] {
        let row = screen.map.find(name).expect("its row").rect[2] / PPP;
        let (_, words) = (texts.iter().find(|(text, _)| text == name)).expect("its words");
        assert!(
            words.max.x <= row - 8.0 * SPACE,
            "{name}: {} by {row}",
            words.max.x
        );
    }
    let tap = |screen: &mut Screen, name| screen.tap(&projection, name);
    let length = ReplayIntent::Length(VideoLength::Segment);
    assert_eq!(tap(&mut screen, "Segment"), asked(length));
    assert!(
        tap(&mut screen, "30 s highlight").is_empty(),
        "chosen already"
    );
    let size = ReplayIntent::Size(VideoSize::Canvas);
    assert_eq!(tap(&mut screen, "Original"), asked(size));
    assert_eq!(
        tap(&mut screen, "Stop on the finished picture"),
        asked(ReplayIntent::Hold(false))
    );
    assert_eq!(
        tap(&mut screen, "Strokes and time"),
        asked(ReplayIntent::Caption(false))
    );
    let save = ReplayIntent::Export(Delivery::Save);
    assert_eq!(tap(&mut screen, "Save to album"), asked(save));
    assert!(
        screen.map.find("Save to album").is_err(),
        "the panel closed"
    );
}

#[test]
fn the_canvas_s_own_size_beyond_the_encoder_and_the_line_without_the_hold_are_greyed() {
    let mut greyed = replay();
    greyed.export.sizes[1] = None;
    greyed.export.hold = false;
    let projection = open(greyed);
    let mut screen = Screen::new(Language::Chinese);
    screen.settle(&projection);
    screen.tap(&projection, "Export video…");
    let texts = screen.settle(&projection);
    assert!(has(&texts, "原始 2880 × 1800") && has(&texts, "笔数与时长"));
    nothing_clipped(&screen.map, &texts);
    assert!(screen.tap(&projection, "Original").is_empty());
    assert!(screen.tap(&projection, "Strokes and time").is_empty());
    let hold = ReplayIntent::Hold(true);
    assert_eq!(
        screen.tap(&projection, "Stop on the finished picture"),
        asked(hold)
    );
}

#[test]
fn the_video_s_pills_tell_its_progress_its_end_and_its_failure() {
    let mut working = replay();
    working.video = VideoNotice::Working { percent: 62 };
    working.controls = Availability::Running;
    let projection = open(working);
    for (language, words) in [
        (Language::English, ["Exporting video", "62 %"]),
        (Language::Chinese, ["正在导出视频", "62 %"]),
    ] {
        let mut screen = Screen::new(language);
        let texts = screen.settle(&projection);
        assert!(words.iter().all(|word| has(&texts, word)), "{texts:?}");
        nothing_clipped(&screen.map, &texts);
        for greyed in ["Back", "Pause", "Export video…", "1×"] {
            assert!(screen.tap(&projection, greyed).is_empty(), "{greyed} waits");
        }
        assert!(screen.map.find("30 s highlight").is_err(), "no panel");
        assert_eq!(
            screen.tap(&projection, "Cancel"),
            asked(ReplayIntent::Cancel)
        );
    }
    let mut saved = replay();
    saved.video = VideoNotice::Saved { seconds: 48 };
    let projection = open(saved);
    let mut screen = Screen::new(Language::Chinese);
    let texts = screen.settle(&projection);
    assert!(has(&texts, "视频已存到相册 · 48 秒"), "{texts:?}");
    nothing_clipped(&screen.map, &texts);
    assert_eq!(
        screen.tap(&projection, "Share…"),
        asked(ReplayIntent::ShareSaved)
    );
    assert_eq!(screen.tap(&projection, "OK"), asked(ReplayIntent::Dismiss));
    let mut failed = replay();
    failed.video = VideoNotice::Failed(VideoError::Render);
    let projection = open(failed);
    let mut screen = Screen::new(Language::English);
    let texts = screen.settle(&projection);
    assert!(has(&texts, "Couldn't export the video"), "{texts:?}");
    assert_eq!(screen.tap(&projection, "OK"), asked(ReplayIntent::Dismiss));
}
