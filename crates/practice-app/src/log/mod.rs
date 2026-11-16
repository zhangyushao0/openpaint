//! The app's practice log (`PracticeLog`), kept across artworks like the toolbox: every
//! practice sheet's record (the day's lessons, a test's, the long colour copies) with the copies'
//! sheets' titles (their names), and the practice packs kept (their manifests: the pictures single
//! shapes and colour copies copy, `copies.rs`). What the records tell is made again as they
//! change, never on a frame: the learner and the path (`path.rs`: the units with their skills'
//! dots, the lessons offered, a lesson started), the practice book. Progress counts lessons,
//! never days; every lesson but a copy's is the learner's, in the order of sheet and lesson
//! (`lessons`); the day's lessons follow down one sheet (`day.rs`). The Pomodoro's timer and its
//! blocks are the log's too (S124, `timer.rs`, `crate::pomodoro`), and the blind pairs answered,
//! each skill's curve and the copies' cards (S147, `progress.rs`).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

use openpaint_practice::{
    BlindPair, Book, CopyRef, Learner, PERSONAL, Pack, PairAsked, Progress, Purpose, Sheet,
    local_day,
};

use crate::book::{BookProjection, BookRecord, SheetRead};
use crate::copies::{PictureShown, pictures_shown};
use crate::pairs::CopyCard;
use crate::path::PathProjection;
use crate::pomodoro::Pomodoro;
use openpaint_app::ProjectId;
use openpaint_app::Title;

mod copies;
mod day;
mod path;
mod progress;
mod timer;

pub use progress::Refit;

/// Every practice sheet the library holds, oldest first, the titles of the copies' sheets, and
/// the practice packs kept; the practice book made of them on the device's clock, with the
/// sheets read for it; the learner and the path they tell of.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PracticeLog {
    sheets: Vec<Sheet>,
    /// Each colour copy's sheet's title, by its project.
    titles: HashMap<u64, Title>,
    packs: Vec<Pack>,
    /// The packs' pictures as the options show them, made again as the log changes.
    pictures: Arc<[PictureShown]>,
    /// The device's offset from UTC in minutes, and the local day now.
    offset: i32,
    today: i64,
    /// The practice book and the records it names, made again as the log changes; the sheets
    /// read for it, by record (one is read again once its record changed).
    book: Arc<Book>,
    named: Arc<[BookRecord]>,
    read: Arc<BTreeMap<u64, Arc<SheetRead>>>,
    /// The learner the records tell of (S122, `path.rs`), made again as the log changes.
    learner: Learner,
    /// The path as the home and the path page show it, made again as the log changes.
    path: PathProjection,
    /// The kinds whose How the sheets opened since the app started told, by their facts' keys
    /// (`tell`; the records tell the rest, `path.rs`).
    told: HashSet<&'static str>,
    /// The Pomodoro's timer and its blocks (S124).
    pomodoro: Pomodoro,
    /// The blind pairs answered (S147), oldest first; those a lesson's end asked on the local day
    /// held, which none asks again that day.
    pairs: Vec<BlindPair>,
    asked_today: (i64, Vec<PairAsked>),
    /// Each skill's curve as the storage worker last fitted it for the records as they stand
    /// (`progress.rs`), and how often the records changed: a fit of older records is dropped.
    progress: Option<Arc<Progress>>,
    changes: u64,
    /// The copies' cards read for the book's re-test pairs, by copy.
    cards: Arc<BTreeMap<CopyRef, Arc<CopyCard>>>,
}

impl PracticeLog {
    /// The log of `sheets` (with the titles of the sheets `titles` names) and `packs`.
    #[must_use]
    pub fn new(mut sheets: Vec<Sheet>, titles: Vec<(ProjectId, Title)>, packs: Vec<Pack>) -> Self {
        sheets.sort_by_key(|sheet| sheet.id);
        let mut log = Self {
            sheets,
            titles: (titles.into_iter())
                .map(|(id, title)| (id.get(), title))
                .collect(),
            ..Self::default()
        };
        packs_sorted(&mut log.packs, packs);
        log.shown();
        log.booked();
        log
    }

    /// The device's clock now: `now` seconds since 1970 UTC, `offset` minutes from UTC.
    pub fn set_clock(&mut self, now: u64, offset: i32) {
        let today = local_day(now, offset);
        if offset != self.offset {
            (self.today, self.offset) = (today, offset);
            self.booked();
        } else if today != self.today {
            self.today = today;
            self.pathed();
        }
    }

    /// The sheets the store read for the book, by record.
    pub fn sheets_read(&mut self, read: Vec<(u64, SheetRead)>) {
        let mut kept = (*self.read).clone();
        kept.extend(read.into_iter().map(|(id, sheet)| (id, Arc::new(sheet))));
        self.read = Arc::new(kept);
        self.pathed();
    }

    /// The learner, the book and the path made again from the records; the curves fitted again
    /// when next asked.
    fn booked(&mut self) {
        self.progress = None;
        self.changes += 1;
        self.learner = Learner::of(&self.sheets);
        let blocks = self.pomodoro.done();
        self.book = Arc::new(Book::of(&self.sheets, blocks, self.offset));
        self.pomodoro.tallied(&self.sheets);
        self.named = (self.sheets.iter())
            .filter(|s| s.canvas != 0)
            .map(|s| BookRecord {
                id: s.id,
                sheet: ProjectId::new(s.canvas),
                rows: self.rows(s),
                copy: s.is_copy().then(|| self.copy_name(s)),
            })
            .collect();
        self.pathed();
    }

    /// The practice book as one frame sees it.
    fn book(&self) -> BookProjection {
        BookProjection {
            book: Arc::clone(&self.book),
            today: self.today,
            records: Arc::clone(&self.named),
            read: Arc::clone(&self.read),
            progress: self.progress.clone(),
            cards: Arc::clone(&self.cards),
        }
    }

    /// The packs kept now (as the store listed them).
    pub fn set_packs(&mut self, packs: Vec<Pack>) {
        packs_sorted(&mut self.packs, packs);
        self.shown();
        self.pathed();
    }

    /// Keeps `pack`, new or a newer edition of one kept.
    pub fn keep_pack(&mut self, pack: Pack) {
        let mut packs = std::mem::take(&mut self.packs);
        packs.retain(|kept| kept.id != pack.id);
        packs.push(pack);
        self.set_packs(packs);
    }

    /// The user's own pictures as the store keeps them now: `pack`, or none left.
    pub fn set_personal(&mut self, pack: Option<Pack>) {
        match pack {
            Some(pack) => self.keep_pack(pack),
            None => {
                let mut packs = std::mem::take(&mut self.packs);
                packs.retain(|kept| kept.id != PERSONAL);
                self.set_packs(packs);
            }
        }
    }

    /// The packs kept.
    #[must_use]
    pub fn packs(&self) -> &[Pack] {
        &self.packs
    }

    /// The id the next sheet begun takes.
    #[must_use]
    pub fn next_id(&self) -> u64 {
        self.sheets.last().map_or(1, |s| s.id + 1)
    }

    /// Sheet `id`, if the log holds it.
    #[must_use]
    pub fn sheet(&self, id: u64) -> Option<&Sheet> {
        self.sheets.iter().find(|s| s.id == id)
    }

    /// The test sheets (`Purpose::Test`, debug builds' chore) started at or after `since`.
    #[must_use]
    pub fn tests_since(&self, since: u64) -> Vec<Sheet> {
        (self.sheets.iter())
            .filter(|sheet| sheet.purpose == Purpose::Test && sheet.started >= since)
            .cloned()
            .collect()
    }

    /// The sheet whose canvas is project `id`.
    #[must_use]
    pub fn of_canvas(&self, id: ProjectId) -> Option<&Sheet> {
        self.sheets.iter().find(|s| s.canvas == id.get())
    }

    /// Keeps `sheet`, new or newer; its sheet is read again when the book asks.
    pub fn keep(&mut self, sheet: Sheet) {
        if self.read.contains_key(&sheet.id) {
            let mut read = (*self.read).clone();
            read.remove(&sheet.id);
            self.read = Arc::new(read);
        }
        match self.sheets.iter_mut().find(|s| s.id == sheet.id) {
            Some(kept) => *kept = sheet,
            None => {
                self.sheets.push(sheet);
                self.sheets.sort_by_key(|s| s.id);
                self.shown();
            }
        }
        self.booked();
    }

    /// The sheet of project `sheet` is titled `title` (a colour copy named or renamed).
    pub fn name(&mut self, sheet: ProjectId, title: Title) {
        self.titles.insert(sheet.get(), title);
        self.booked();
    }

    /// Forgets sheet `id`, which the store removed, with its title.
    pub fn forget(&mut self, id: u64) {
        if let Some(canvas) = self.sheet(id).map(|sheet| sheet.canvas) {
            self.titles.remove(&canvas);
        }
        self.sheets.retain(|s| s.id != id);
        self.forget_pairs(id);
        self.shown();
        self.booked();
    }

    /// The pictures as the options show them, made again.
    fn shown(&mut self) {
        self.pictures = pictures_shown(&self.packs, &self.sheets);
    }
}

/// `packs` into `kept`, the official ones first.
fn packs_sorted(kept: &mut Vec<Pack>, mut packs: Vec<Pack>) {
    packs.sort_by(|a, b| (a.personal(), &a.id).cmp(&(b.personal(), &b.id)));
    *kept = packs;
}
