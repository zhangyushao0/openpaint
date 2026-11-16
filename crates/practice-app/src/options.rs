//! The practice options on the home (the path, the practice options and the practice book): a
//! lesson of the path (S122) or a long colour copy of a chosen picture, each opened at once; a
//! copy renamed or removed (S83); a pack imported and a picture of the user's
//! own added at a tier (S80), each through a picker that takes its turn with the gallery's own
//! dialogs (`Gallery::choose`); one of the user's pictures removed or moved to another tier; the
//! practice book's sheets read (S89). Like the gallery, this decides what to ask of the store and
//! what the user is told; the runtime turns each request into the storage worker's job.

use std::collections::VecDeque;

use openpaint_app::{
    AppError, EditorAction, FileProblem, Gallery, HostRequest, ImageSource, PortError, ProjectId,
    Title, problem,
};
use openpaint_practice::{CopyRef, ItemRef, Pack, PictureKind, Tier};

use crate::path::Start;

/// Most bytes a practice pack may have to be imported (read whole, then checked).
pub const MAX_PACK_BYTES: u64 = 512 << 20;

/// The picker's tag for a pack to import; a picture's is its tier's code (1 to 4).
const PACK: u32 = 0;

/// What the user can ask of the practice options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OptionsIntent {
    /// A lesson started and opened at once (S122: the home's and the path's Start, Review, a
    /// skill's tile), onto the day's sheet where it takes it; one left unfinished is
    /// continued by opening its sheet (`PathProjection::open`).
    Lesson(Start),
    /// A new long colour copy of the picture chosen, opened at once (the practice options' New
    /// copy; one begun is opened by its sheet).
    Copy(ItemRef),
    /// A colour copy's name: its sheet's title.
    RenameCopy { sheet: ProjectId, title: Title },
    /// A colour copy removed with its sheet, for good; asked only after the user confirmed it.
    RemoveCopy(ProjectId),
    /// "Import practice pack…": a pack file the user chooses, kept beside the library.
    ImportPack,
    /// "Add my picture…": a picture the user chooses, added to their own at the tier chosen first.
    AddPicture(Tier),
    /// One of the user's own pictures deleted, once the user confirmed it (`PictureShown`).
    RemovePicture(ItemRef),
    /// One of the user's own pictures moved to a tier.
    PictureTier(ItemRef, Tier),
    /// The practice book reads the sheets of these practice records (by id) to show them.
    ReadSheets(Vec<u64>),
    /// The practice book's progress fits each skill's curve (S147).
    Refit,
    /// The practice book reads these copies' cards (a re-test's pair beside its curve).
    ReadCards(Vec<CopyRef>),
    /// Closes the notice.
    Dismiss,
}

/// A call the practice options need from the store, in the order it must run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OptionsRequest {
    /// A lesson started (`PracticeLog::start`) and its sheet, which becomes the open one (S122).
    Lesson(Start),
    /// A new long colour copy of the picture given and its sheet, which becomes the open one.
    Copy(ItemRef),
    /// A colour copy's sheet titled anew.
    RenameCopy(ProjectId, Title),
    /// A colour copy removed, its record and its sheet.
    RemoveCopy(ProjectId),
    /// One of the user's own pictures (by its item id) removed from their pack.
    RemovePicture(String),
    /// One of the user's own pictures (by its item id) moved to a tier.
    PictureTier(String, Tier),
    /// The sheets of the practice records given (by id) read for the practice book.
    ReadSheets(Vec<u64>),
    /// Each skill's curve fitted for the practice book.
    Refit,
    /// These copies' cards read for the practice book.
    ReadCards(Vec<CopyRef>),
}

impl OptionsRequest {
    /// Whether it makes a sheet that becomes the open one (a lesson, a copy): it holds the
    /// gallery until the sheet opens, and failing it is a sheet not made.
    #[must_use]
    pub const fn opens(&self) -> bool {
        matches!(self, Self::Lesson(_) | Self::Copy(_))
    }
}

/// A file the user chose for the practice options, for the store to read.
pub enum OptionsFile {
    /// To keep as a practice pack.
    Pack(Box<dyn ImageSource>),
    /// To add to the user's own pictures at the tier, its outline derived.
    Picture(Box<dyn ImageSource>, Tier),
}

/// What the practice options tell the user, with an OK to close it; the gallery's own notice
/// shows first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionsNotice {
    /// The chosen practice pack is kept: its pictures come up in single shapes, or (a pack of
    /// compositions) in divisions and whole copies, its crops (S101) in colour combinations.
    PackImported { compositions: bool, crops: bool },
    /// The chosen file was not kept as a practice pack; nothing changed.
    PackRefused(FileProblem),
    /// The chosen picture is among the user's own, with an outline found or none.
    PictureAdded { outline: bool },
    /// The chosen picture was not added (not a PNG or JPEG one, or unreadable); nothing changed.
    PictureRefused,
}

/// The practice options' flow while the gallery is shown.
#[derive(Debug, Default)]
pub struct Options {
    requests: VecDeque<OptionsRequest>,
    notice: Option<OptionsNotice>,
}

impl Options {
    /// Applies one intent beside `gallery`, whose turn the options take: nothing while an
    /// artwork is being opened, a picker only when no other dialog or file is under way. A new
    /// lesson or copy holds the gallery until its sheet opens (`Gallery::opening`).
    ///
    /// # Errors
    /// `Unavailable` while an artwork is being opened, and for a picker while a system dialog is
    /// open or a file is read or written.
    pub fn dispatch(
        &mut self,
        intent: OptionsIntent,
        gallery: &mut Gallery,
    ) -> Result<(), AppError> {
        let request = match intent {
            OptionsIntent::Dismiss => {
                self.notice = None;
                return Ok(());
            }
            OptionsIntent::ImportPack => return gallery.choose(HostRequest::OpenFile, PACK),
            OptionsIntent::AddPicture(tier) => {
                return gallery.choose(HostRequest::OpenImage, u32::from(tier.code()));
            }
            OptionsIntent::Lesson(start) => OptionsRequest::Lesson(start),
            OptionsIntent::Copy(picture) => OptionsRequest::Copy(picture),
            OptionsIntent::RenameCopy { sheet, title } => OptionsRequest::RenameCopy(sheet, title),
            OptionsIntent::RemoveCopy(sheet) => OptionsRequest::RemoveCopy(sheet),
            OptionsIntent::RemovePicture(picture) => OptionsRequest::RemovePicture(picture.item),
            OptionsIntent::PictureTier(picture, tier) => {
                OptionsRequest::PictureTier(picture.item, tier)
            }
            OptionsIntent::ReadSheets(records) => OptionsRequest::ReadSheets(records),
            OptionsIntent::Refit => OptionsRequest::Refit,
            OptionsIntent::ReadCards(copies) => OptionsRequest::ReadCards(copies),
        };
        let availability = gallery.availability();
        if !availability.is_available() {
            return Err(AppError::Unavailable {
                action: EditorAction::Gallery,
                availability,
            });
        }
        if request.opens() {
            gallery.opening();
        }
        self.requests.push_back(request);
        Ok(())
    }

    /// The next call for the store, oldest first.
    pub fn request(&mut self) -> Option<OptionsRequest> {
        self.requests.pop_front()
    }

    /// The picker asked with `tag` answered (`FileJob::Mode`): the file for the store to read,
    /// or none, the notice saying why when the system could not give it.
    pub fn chosen(
        &mut self,
        tag: u32,
        source: Option<Box<dyn ImageSource>>,
    ) -> Option<OptionsFile> {
        let tier = (tag != PACK).then(|| u8::try_from(tag).ok().and_then(Tier::of_code));
        match (tier, source) {
            (None, Some(source)) => Some(OptionsFile::Pack(source)),
            (Some(Some(tier)), Some(source)) => Some(OptionsFile::Picture(source, tier)),
            (None, None) => {
                self.notice = Some(OptionsNotice::PackRefused(FileProblem::Unreadable));
                None
            }
            (Some(_), _) => {
                self.notice = Some(OptionsNotice::PictureRefused);
                None
            }
        }
    }

    /// The store kept the chosen pack, or why not.
    pub fn imported(&mut self, result: &Result<Pack, PortError>) {
        self.notice = Some(match result {
            Ok(pack) => imported(pack),
            Err(error) => OptionsNotice::PackRefused(problem(error)),
        });
    }

    /// The store added the chosen picture (the personal pack as it is now, the picture its last
    /// item), or why not.
    pub fn added(&mut self, result: &Result<Pack, PortError>) {
        self.notice = Some(match result {
            Ok(pack) => OptionsNotice::PictureAdded {
                outline: pack.items.last().is_some_and(|i| i.outline.is_some()),
            },
            Err(_) => OptionsNotice::PictureRefused,
        });
    }

    /// What to tell the user, once the gallery has nothing to.
    #[must_use]
    pub const fn notice(&self) -> Option<OptionsNotice> {
        self.notice
    }
}

/// What the notice says of `pack` imported: where its pictures come up (S95: a pack of
/// compositions in divisions and whole copies; S101: crops in colour combinations).
fn imported(pack: &Pack) -> OptionsNotice {
    let holds = |kind| pack.items.iter().any(|item| item.kind == kind);
    OptionsNotice::PackImported {
        compositions: holds(PictureKind::Composition),
        crops: holds(PictureKind::Crop),
    }
}
