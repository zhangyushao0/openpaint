//! The projects (long colour copies, drawings from life) and the pictures one can be begun of, as
//! the home and the practice options show them (`log/copies.rs` keeps them): each with its name
//! and picture, and every picture of the packs kept (S83), the packs' by tier and the user's own
//! after them, each with its credit, its tier and how often lessons, copies and drawings from life
//! used it (what removing one of the user's own would leave without its reference).

use std::sync::Arc;

use openpaint_practice::{ItemRef, OnSheet, Pack, PictureKind, Sheet, Tier, lessons};

use openpaint_app::ProjectId;
use openpaint_app::Title;

/// A project as the practice options list it: a long colour copy, or a drawing from life.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopyShown {
    /// Its sheet, which opens it.
    pub sheet: ProjectId,
    /// Its number among the projects of its sort.
    pub number: u32,
    /// Its name: its sheet's title ("Colour copy n", "Drawing from life n" until renamed).
    pub name: Title,
    /// Its picture, while a pack kept holds it (none: removed).
    pub picture: Option<ItemRef>,
    /// A drawing from life (S151), else a colour copy.
    pub from_life: bool,
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
    /// The colour copies begun of it, the lessons that showed it and the drawings from life
    /// drawn against it (S151).
    pub copies: u32,
    pub lessons: u32,
    pub drawings: u32,
}

/// What used a picture: a lesson, a colour copy or a drawing from life.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Use {
    Lesson,
    Copy,
    Life,
}

/// The pictures of `packs` as the options show them, with how many lessons, copies and drawings
/// from life of `records` used each.
pub(crate) fn pictures_shown(packs: &[Pack], records: &[Sheet]) -> Arc<[PictureShown]> {
    let lessons = lessons(records);
    let used = |on: &OnSheet<'_>| match (on.is_copy(), on.sheet.drawn_from_life()) {
        (false, _) => Use::Lesson,
        (true, false) => Use::Copy,
        (true, true) => Use::Life,
    };
    let uses = |item: &ItemRef, by: Use| -> u32 {
        let count = (lessons.iter())
            .filter(|on| used(on) == by)
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
                copies: uses(&item, Use::Copy),
                lessons: uses(&item, Use::Lesson),
                drawings: uses(&item, Use::Life),
                item,
            });
        }
    }
    // A stable sort: the packs' order within a tier, the user's own last.
    shown.sort_by_key(|picture| (picture.personal, picture.tier));
    shown.into()
}
