//! The gallery's Today card and its options as one UI frame sees them (`log.rs` keeps
//! the log they are made of): the stage practised and its sessions, each ready stage's kinds to
//! drill (S91), the packs kept, the long
//! colour copies with their names and pictures, and every picture a copy can be begun of (S83),
//! the packs' by tier and the user's own after them, each with its credit, its tier and how often
//! sessions and copies used it (what removing one of the user's own would leave without its
//! reference).

use std::sync::Arc;

use openpaint_practice::{Family, ItemRef, Pack, PictureKind, Sheet, Tier, lessons};

use crate::book::BookProjection;
use crate::pictures::PackShown;
use openpaint_app::ProjectId;
use openpaint_app::Title;

/// The gallery's Today card, its options and the practice book.
#[derive(Debug, Clone, PartialEq)]
pub struct TodayProjection {
    pub stage: u8,
    /// Sessions finished in the stage, and about how many it holds.
    pub done: u32,
    pub sessions: u32,
    /// The sheet of the session still open, to continue.
    pub open: Option<ProjectId>,
    /// The next stage, offered on the card once this one's sessions are done.
    pub next: Option<u8>,
    /// The stages a session can be started in, from 1 (the options offer the others).
    pub ready: u8,
    /// The packs kept, the official ones first.
    pub packs: Vec<PackShown>,
    /// Whether a pack kept holds a composition: divisions and whole copies can be drilled (S95);
    /// and one whose value masses read: value masses can be (S96).
    pub compositions: bool,
    pub masses: bool,
    /// Whether a pack kept holds a crop: colour combinations can be practised (S101).
    pub crops: bool,
    /// The long colour copies, newest first.
    pub copies: Vec<CopyShown>,
    /// The pictures a colour copy can be begun of: the official packs' by tier (easiest first,
    /// in their packs' order), then the user's own, oldest first.
    pub pictures: Arc<[PictureShown]>,
    /// The practice book, which the card's road opens (S89).
    pub book: BookProjection,
    /// Each ready stage's kinds as the practice options' tiles show them, stage by stage (S91).
    pub drills: Vec<DrillShown>,
}

/// A kind as its tile in the practice options shows it (S91).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrillShown {
    /// The stage whose row it lies under, and the kind.
    pub stage: u8,
    pub family: Family,
    /// Compared at least once anywhere (a session or a drill): drawn in ink, else in pencil.
    pub compared: bool,
    /// The sheet of its drill left unfinished, which its tile continues.
    pub open: Option<ProjectId>,
}

/// A long colour copy as the practice options list it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopyShown {
    /// Its sheet, which opens it.
    pub sheet: ProjectId,
    /// Its number among the copies.
    pub number: u32,
    /// Its name: its sheet's title ("Colour copy n" until renamed).
    pub name: Title,
    /// Its picture, while a pack kept holds it (none: removed).
    pub picture: Option<ItemRef>,
}

/// A picture of a pack kept, as the practice options show it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PictureShown {
    pub item: ItemRef,
    pub tier: Tier,
    pub title: String,
    /// Its credit as the options show it under it (an official item's), none for the user's own.
    pub credit: Option<String>,
    /// One of the user's own.
    pub personal: bool,
    pub outline: bool,
    /// The colour copies begun of it, and the sessions that showed it.
    pub copies: u32,
    pub sessions: u32,
}

/// The pictures of `packs` as the options show them, with how many lessons of `records` used
/// each.
pub(crate) fn pictures_shown(packs: &[Pack], records: &[Sheet]) -> Arc<[PictureShown]> {
    let lessons = lessons(records);
    let uses = |item: &ItemRef, copies: bool| -> u32 {
        let count = (lessons.iter())
            .filter(|on| on.is_copy() == copies)
            .filter(|on| {
                (on.lesson.exercises.iter())
                    .any(|laid| laid.exercise.picture.as_ref() == Some(item))
            })
            .count();
        u32::try_from(count).unwrap_or(u32::MAX)
    };
    let mut shown: Vec<PictureShown> = Vec::new();
    for pack in packs {
        for found in pack.items.iter().filter(|i| i.kind == PictureKind::Shape) {
            let item = ItemRef {
                pack: pack.id.clone(),
                item: found.id.clone(),
            };
            shown.push(PictureShown {
                tier: found.tier,
                title: found.title.clone(),
                credit: found
                    .credit
                    .as_ref()
                    .map(|credit| credit.attribution.clone()),
                personal: pack.personal(),
                outline: found.outline.is_some(),
                copies: uses(&item, true),
                sessions: uses(&item, false),
                item,
            });
        }
    }
    // A stable sort: the packs' order within a tier, the user's own last.
    shown.sort_by_key(|picture| (picture.personal, picture.tier));
    shown.into()
}

impl TodayProjection {
    /// The pictures of `tier` of the official packs, or the user's own (`None`).
    pub fn pictures_of(&self, tier: Option<Tier>) -> impl Iterator<Item = &PictureShown> {
        (self.pictures.iter()).filter(move |picture| match tier {
            Some(tier) => !picture.personal && picture.tier == tier,
            None => picture.personal,
        })
    }

    /// The picture `item` names, while a pack kept holds it.
    #[must_use]
    pub fn picture(&self, item: &ItemRef) -> Option<&PictureShown> {
        self.pictures.iter().find(|picture| picture.item == *item)
    }
}
