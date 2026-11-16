//! Practice's part of the screen between frames (`PracticeScreens`) and the hooks it fills for
//! one frame (`Hooked`): the open sheet's on the canvas, the path's on the home (S122). Its
//! intents never travel among the painting's (D112): the runtime takes them after the frame's
//! routing (`take`) and applies them after the frame's painting intents.

use openpaint_practice::ItemRef;
use openpaint_practice_app::{
    OptionsIntent, OptionsNotice, PathProjection, PictureImage, PracticeIntent, PracticeProjection,
    TimerIntent, TimerShown,
};
use openpaint_ui::{CanvasHook, HomeHook, Hooks};

use crate::home::{HomePart, PathHook};
use crate::sheet::{SheetHook, SheetPart};

/// Practice's screens: a sheet's pictures, bar and card, the home's column, the path page, the
/// practice options and book, and the taps since the last `take`.
#[derive(Debug, Default)]
pub struct PracticeScreens {
    sheet: SheetPart,
    sheet_taps: Vec<PracticeIntent>,
    home: HomePart,
    options_taps: Vec<OptionsIntent>,
    /// The Pomodoro's taps (S124): the bar's ring and End and the block's end on the sheet; the
    /// home's row and the options' chime switch on the home.
    timer_taps: Vec<TimerIntent>,
    home_timer_taps: Vec<TimerIntent>,
}

/// What practice shows in one frame: the open `sheet` on the canvas, or on the home the path
/// (its column, the path page, the practice options and book) with what the options tell; and
/// the Pomodoro's timer on either (S124).
pub type Shown<'a> = (
    Option<&'a PracticeProjection>,
    Option<(&'a PathProjection, Option<OptionsNotice>)>,
    &'a TimerShown,
);

/// Practice's taps of one frame: the sheet's, the options' and the timer's.
pub type Taken = (Vec<PracticeIntent>, Vec<OptionsIntent>, Vec<TimerIntent>);

impl PracticeScreens {
    /// The hooks practice fills in one frame for what it shows (`shown`).
    pub fn hooked<'a>(&'a mut self, (sheet, path, timer): Shown<'a>) -> Hooked<'a> {
        // On the home, what the sheet last left laid over the canvas goes back with it (S144).
        let (sheet, laid) = match sheet {
            Some(sheet) => {
                self.home.sheet_shows();
                let hook = SheetHook {
                    part: &mut self.sheet,
                    sheet,
                    taps: &mut self.sheet_taps,
                    timer,
                    timer_taps: &mut self.timer_taps,
                };
                (Some(hook), &[][..])
            }
            None => (None, self.sheet.laid()),
        };
        let home = path.map(|(path, notice)| PathHook {
            part: &mut self.home,
            path,
            notice,
            taps: &mut self.options_taps,
            timer,
            timer_taps: &mut self.home_timer_taps,
            laid,
        });
        Hooked { sheet, home }
    }

    /// Practice's taps since the last call: the sheet's, the options' and the timer's. Call after
    /// the shell's `route`; the runtime applies them after the frame's painting intents, the
    /// timer's last.
    pub fn take(&mut self) -> Taken {
        let sheet = std::mem::take(&mut self.sheet_taps);
        let options = std::mem::take(&mut self.options_taps);
        let mut timer = std::mem::take(&mut self.timer_taps);
        timer.append(&mut self.home_timer_taps);
        (sheet, options, timer)
    }

    /// The next picture the practice options want read (a pack's, by its item), once each: the
    /// rows showing first.
    pub fn pack_picture_wanted(&mut self) -> Option<ItemRef> {
        self.home.pack_picture_wanted()
    }

    /// `item`'s picture for the practice options, 8-bit sRGB with straight alpha (none: it could
    /// not be read): its rows show it from the next pass on.
    pub fn set_pack_picture(&mut self, item: ItemRef, picture: Option<PictureImage>) {
        self.home.set_pack_picture(item, picture);
    }

    /// Whether a page of practice's lies over the home: the path page or the practice book.
    #[must_use]
    pub const fn page_shows(&self) -> bool {
        self.home.page_shows()
    }
}

/// Practice's hooks for one frame.
pub struct Hooked<'a> {
    sheet: Option<SheetHook<'a>>,
    home: Option<PathHook<'a>>,
}

impl Hooked<'_> {
    /// The hooks as the shell takes them, for one call (`Shell::show`, `route`, `back`).
    pub fn hooks(&mut self) -> Hooks<'_> {
        Hooks {
            canvas: (self.sheet.as_mut()).map(|hook| hook as &mut dyn CanvasHook),
            home: (self.home.as_mut()).map(|hook| hook as &mut dyn HomeHook),
        }
    }
}
