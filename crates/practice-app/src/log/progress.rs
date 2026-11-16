//! Progress and the blind pairs in the log (S147): the pairs answered, kept as the store holds them;
//! those a lesson's end asked today, which no end asks again that day (one skipped stores
//! nothing); each skill's curve, fitted on the storage worker from the records as they stand
//! (`Refit`, the book asking it when it shows) and dropped whenever they change; and the copies'
//! cards the book's re-test pairs show.

use std::collections::BTreeMap;
use std::sync::Arc;

use openpaint_practice::{BlindPair, CopyRef, PairAsked, Progress, Sheet};

use super::PracticeLog;
use crate::pairs::CopyCard;

/// The records a refit reads, and the change of the log they stand at.
#[derive(Debug, Clone, PartialEq)]
pub struct Refit {
    pub sheets: Vec<Sheet>,
    pub pairs: Vec<BlindPair>,
    pub offset: i32,
    pub stamp: u64,
}

impl Refit {
    /// Each skill's curve (`Progress::of`), on the storage worker.
    #[must_use]
    pub fn fitted(&self) -> Progress {
        Progress::of(&self.sheets, &self.pairs, self.offset)
    }
}

impl PracticeLog {
    /// The pairs the store holds, as it read them.
    pub fn set_pairs(&mut self, pairs: Vec<BlindPair>) {
        self.pairs = pairs;
        self.booked();
    }

    /// Keeps `pair`, answered and going to the store.
    pub fn keep_pair(&mut self, pair: BlindPair) {
        self.pairs.push(pair);
        self.booked();
    }

    /// The pairs answered, oldest first.
    #[must_use]
    pub fn pairs(&self) -> &[BlindPair] {
        &self.pairs
    }

    /// A lesson's end asked `asked` today: none asks them again today.
    pub fn asked(&mut self, asked: Vec<PairAsked>) {
        if self.asked_today.0 != self.today {
            self.asked_today = (self.today, Vec::new());
        }
        for pair in asked {
            if !self.asked_today.1.contains(&pair) {
                self.asked_today.1.push(pair);
            }
        }
    }

    /// The pairs asked today.
    pub(crate) fn asked_today(&self) -> Vec<PairAsked> {
        if self.asked_today.0 == self.today {
            self.asked_today.1.clone()
        } else {
            Vec::new()
        }
    }

    /// What the storage worker fits the curves from: the records as they stand.
    #[must_use]
    pub fn refit(&self) -> Refit {
        Refit {
            sheets: self.sheets.clone(),
            pairs: self.pairs.clone(),
            offset: self.offset,
            stamp: self.changes,
        }
    }

    /// The curves fitted from the records as they stood at `stamp`: kept unless they changed since.
    pub fn fitted(&mut self, stamp: u64, progress: Progress) {
        if stamp == self.changes {
            self.progress = Some(Arc::new(progress));
            self.pathed();
        }
    }

    /// Each of `copies` with its sheet's record, for its card to be read; one whose sheet is gone
    /// or never stored left out.
    #[must_use]
    pub fn copies_of(&self, copies: &[CopyRef]) -> Vec<(CopyRef, Sheet)> {
        (copies.iter())
            .filter_map(|copy| {
                let (sheet, _) = copy.find(&self.sheets)?;
                (sheet.canvas != 0).then(|| (*copy, sheet.clone()))
            })
            .collect()
    }

    /// The device's offset from UTC in minutes, as the clock last said.
    #[must_use]
    pub const fn offset(&self) -> i32 {
        self.offset
    }

    /// The store read `cards`.
    pub fn cards_read(&mut self, cards: &[Arc<CopyCard>]) {
        let mut kept: BTreeMap<CopyRef, Arc<CopyCard>> = (*self.cards).clone();
        kept.extend(cards.iter().map(|card| (card.copy, Arc::clone(card))));
        self.cards = Arc::new(kept);
        self.pathed();
    }

    /// The pairs asked at sheet `id`, which the store removed with it.
    pub(super) fn forget_pairs(&mut self, id: u64) {
        self.pairs.retain(|pair| pair.lesson.0 != id);
    }
}
