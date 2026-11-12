//! Practice on the home (S122, D112), the ui's home hook (`openpaint_ui::HomeHook`) filled: the
//! practice column beside the studio's artworks (`column.rs`: the next lesson and Start, the
//! review on the side, All units, Practice book), the path page grown out of All units over the
//! whole home (`path.rs`: the units, each skill's tile and dots, the next lesson, the review and a
//! unit's full lesson), the practice book (`book/`), the practice options grown out of their
//! button (`options.rs`, `pictures.rs`, `copies.rs`) and what they tell in the home's notice pill
//! after the home's own. The taps are practice's own (`OptionsIntent`); opening a sheet asks the
//! home, and the sheet grows out of what was tapped.

mod book;
mod column;
mod copies;
mod options;
mod path;
mod pictures;
#[cfg(test)]
mod tests;

use egui::{Context, Rect, Ui};
use openpaint_app::{GalleryIntent, Intent};
use openpaint_practice::ItemRef;
use openpaint_practice_app::{OptionsIntent, OptionsNotice, PathProjection, PictureImage, Start};
use openpaint_ui::components::carry::Opening;
use openpaint_ui::{HomeHook, Page};

use crate::text::{self as t, Text};

/// The first lesson's exercises and about how many minutes they take (`schedule::first`).
pub(crate) const FIRST: (u8, u8) = (3, 2);

/// Practice's part of the home between frames: the path page, the practice book, the practice
/// options and what was opened from them (the canvas grows out of it).
#[derive(Debug, Default)]
pub(crate) struct HomePart {
    practice: options::PracticeOptions,
    book: book::BookState,
    path: path::PathPage,
    opening: Option<Opening>,
}

/// The home's taps and the practice options' taps.
type Taps<'a> = (&'a mut Vec<Intent>, &'a mut Vec<OptionsIntent>);

impl HomePart {
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

    /// A lesson started as `start` from what lies at `from` (the sheet grows out of it): the
    /// next lesson while today's is unfinished opens its sheet, which continues it.
    fn start(&mut self, start: Start, from: Rect, path: &PathProjection, (taps, asked): Taps<'_>) {
        let open = (path.open.map(|open| open.sheet)).filter(|_| start == Start::Next);
        match open {
            Some(sheet) => taps.push(Intent::Gallery(GalleryIntent::Open(sheet))),
            None => asked.push(OptionsIntent::Lesson(start)),
        }
        self.opening = Some(Opening {
            from,
            id: open,
            picture: None,
        });
    }
}

/// Practice's home hook for one frame: its state, the path, the options' notice and their taps.
pub(crate) struct PathHook<'a> {
    pub(crate) part: &'a mut HomePart,
    pub(crate) path: &'a PathProjection,
    pub(crate) notice: Option<OptionsNotice>,
    pub(crate) taps: &'a mut Vec<OptionsIntent>,
}

impl HomeHook for PathHook<'_> {
    fn page(&mut self, ctx: &Context, taps: &mut Vec<Intent>) -> Page {
        let part = &mut *self.part;
        let covers = if part.book.shows() {
            // The practice book takes the whole screen: its own island only.
            part.book_island(ctx);
            part.book_page(ctx, self.path, (taps, self.taps));
            part.book.covers()
        } else if part.path.shows() {
            part.path_island(ctx);
            part.path_page(ctx, self.path, (taps, self.taps));
            part.path.covers()
        } else {
            return Page::None;
        };
        if covers { Page::Covers } else { Page::Over }
    }

    fn column(&mut self, ui: &mut Ui, rect: Rect, enabled: bool, taps: &mut Vec<Intent>) -> Rect {
        self.part
            .column(ui, rect, (self.path, enabled), (taps, self.taps))
    }

    fn dialogs(&mut self, ctx: &Context, taps: &mut Vec<Intent>) {
        self.part.practice_dialog(ctx, self.path, (taps, self.taps));
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
        self.part.practice.close() | self.part.book.close() | self.part.path.close()
    }
}
