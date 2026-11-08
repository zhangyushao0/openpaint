//! Practice's part of the screen between frames (`PracticeScreens`) and the hooks it fills for
//! one frame (`Hooked`): the open sheet's on the canvas, the Today card's in the gallery. Its
//! intents never travel among the painting's (D112): the runtime takes them after the frame's
//! routing (`take`) and applies them after the frame's painting intents.

use openpaint_practice::ItemRef;
use openpaint_practice_app::{
    OptionsIntent, OptionsNotice, PictureImage, PracticeIntent, PracticeProjection, TodayProjection,
};
use openpaint_ui::{CanvasHook, GalleryHook, Hooks};

use crate::gallery::{GalleryPart, TodayHook};
use crate::sheet::{SheetHook, SheetPart};

/// Practice's screens: a sheet's pictures, bar and card, the practice options and book, and the
/// taps since the last `take`.
#[derive(Debug, Default)]
pub struct PracticeScreens {
    sheet: SheetPart,
    sheet_taps: Vec<PracticeIntent>,
    gallery: GalleryPart,
    options_taps: Vec<OptionsIntent>,
}

/// What practice shows in one frame: the open `sheet` on the canvas, or in the gallery the
/// Today card, its options and the practice book (`today`) with what the options tell.
pub type Shown<'a> = (
    Option<&'a PracticeProjection>,
    Option<(&'a TodayProjection, Option<OptionsNotice>)>,
);

impl PracticeScreens {
    /// The hooks practice fills in one frame for what it shows (`shown`).
    pub fn hooked<'a>(&'a mut self, (sheet, today): Shown<'a>) -> Hooked<'a> {
        let sheet = sheet.map(|sheet| SheetHook {
            part: &mut self.sheet,
            sheet,
            taps: &mut self.sheet_taps,
        });
        let today = today.map(|(today, notice)| TodayHook {
            part: &mut self.gallery,
            today,
            notice,
            taps: &mut self.options_taps,
        });
        Hooked { sheet, today }
    }

    /// Practice's taps since the last call: the sheet's and the options'. Call after the shell's
    /// `route`; the runtime applies them after the frame's painting intents.
    pub fn take(&mut self) -> (Vec<PracticeIntent>, Vec<OptionsIntent>) {
        let sheet = std::mem::take(&mut self.sheet_taps);
        (sheet, std::mem::take(&mut self.options_taps))
    }

    /// The next picture the practice options want read (a pack's, by its item), once each: the
    /// rows showing first.
    pub fn pack_picture_wanted(&mut self) -> Option<ItemRef> {
        self.gallery.pack_picture_wanted()
    }

    /// `item`'s picture for the practice options, 8-bit sRGB with straight alpha (none: it could
    /// not be read): its rows show it from the next pass on.
    pub fn set_pack_picture(&mut self, item: ItemRef, picture: Option<PictureImage>) {
        self.gallery.set_pack_picture(item, picture);
    }
}

/// Practice's hooks for one frame.
pub struct Hooked<'a> {
    sheet: Option<SheetHook<'a>>,
    today: Option<TodayHook<'a>>,
}

impl Hooked<'_> {
    /// The hooks as the shell takes them, for one call (`Shell::show`, `route`, `back`).
    pub fn hooks(&mut self) -> Hooks<'_> {
        Hooks {
            canvas: (self.sheet.as_mut()).map(|hook| hook as &mut dyn CanvasHook),
            gallery: (self.today.as_mut()).map(|hook| hook as &mut dyn GalleryHook),
        }
    }
}
