//! Practice in the gallery (G9, D112), the ui's gallery hook (`openpaint_ui::GalleryHook`)
//! filled: the Today card first in the grid (its road; its action word starting or continuing
//! practice; its sheet opening the practice book, `book.rs` with `book_days.rs` and
//! `book_progress.rs`), the practice options grown out of it (`options.rs`, `drills.rs`,
//! `pictures.rs`, `copies.rs`) and what they tell in the gallery's notice pill after the gallery's
//! own. The options' taps are their own (`OptionsIntent`); opening a sheet asks the gallery.

mod book;
mod copies;
mod drills;
mod options;
mod pictures;

use egui::{Context, Rect, Ui};
use openpaint_app::Intent;
use openpaint_practice::ItemRef;
use openpaint_practice_app::{OptionsIntent, OptionsNotice, PictureImage, TodayProjection};
use openpaint_ui::components::carry::Opening;
use openpaint_ui::{GalleryHook, Page};

use crate::text::{self as t, Text};

/// Practice's part of the gallery between frames: the practice options, the practice book and
/// what was opened from them (the canvas grows out of it).
#[derive(Debug, Default)]
pub(crate) struct GalleryPart {
    practice: options::PracticeOptions,
    book: book::BookState,
    opening: Option<Opening>,
}

impl GalleryPart {
    /// The next picture the practice options want read (a pack's, by its item), once each: the
    /// rows showing first.
    pub(crate) fn pack_picture_wanted(&mut self) -> Option<ItemRef> {
        self.practice.pictures.next_wanted()
    }

    /// `item`'s picture for the practice options, 8-bit sRGB with straight alpha (none: it could
    /// not be read): its rows show it from the next pass on.
    pub(crate) fn set_pack_picture(&mut self, item: ItemRef, picture: Option<PictureImage>) {
        self.practice.pictures.arrive(item, picture);
    }
}

/// Practice's gallery hook for one frame: its state, the Today card's projection, the options'
/// notice and their taps.
pub(crate) struct TodayHook<'a> {
    pub(crate) part: &'a mut GalleryPart,
    pub(crate) today: &'a TodayProjection,
    pub(crate) notice: Option<OptionsNotice>,
    pub(crate) taps: &'a mut Vec<OptionsIntent>,
}

impl GalleryHook for TodayHook<'_> {
    fn island(&mut self, ctx: &Context) -> bool {
        // The practice book takes the whole screen: its own island only.
        if !self.part.book.shows() {
            return false;
        }
        self.part.book_island(ctx);
        true
    }

    fn page(&mut self, ctx: &Context, taps: &mut Vec<Intent>) -> Page {
        if !self.part.book.shows() {
            return Page::None;
        }
        self.part.book_page(ctx, self.today, (taps, self.taps));
        if self.part.book.covers() {
            Page::Covers
        } else {
            Page::Over
        }
    }

    fn card(&mut self, ui: &mut Ui, rect: Rect, enabled: bool, taps: &mut Vec<Intent>) -> Rect {
        let today = (self.today, enabled);
        self.part.today(ui, rect, today, (taps, self.taps))
    }

    fn dialogs(&mut self, ctx: &Context, taps: &mut Vec<Intent>) {
        self.part
            .practice_dialog(ctx, self.today, (taps, self.taps));
    }

    fn notice(&self) -> Option<Text> {
        self.notice.map(t::options_notice)
    }

    fn dismiss(&mut self) {
        self.taps.push(OptionsIntent::Dismiss);
    }

    fn opening(&mut self) -> Option<Opening> {
        self.part.opening.take()
    }

    fn back(&mut self) -> bool {
        self.part.practice.close() | self.part.book.close()
    }
}
