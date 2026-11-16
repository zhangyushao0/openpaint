//! What an end asks of the learner's own eye (S147), only where the learner pauses and sees it
//! (S148): a lesson's end while no Pomodoro's block is in hand asks the blind pair a skill's lesson
//! asks once it is done in this sitting (two earlier copies, `openpaint_practice::ask`), one a
//! skill at a unit's end re-test (its copy just drawn against the start's, `retest_pairs`); in a
//! block a lesson's end shows no card and asks nothing, and End asks at the block's end what its
//! lessons did not (`block_pairs`). Each pair is answered once (left, alike, right) and then
//! revealed, shown while the lesson in hand is the one it was asked at; a finished lesson opened
//! again shows its pairs revealed if answered, else none. An answer goes to the store once
//! (`pairs_request`); the pairs asked, all shown, go to the log so a skipped one is not asked again
//! that day (`offered`); the copies' cards are read on the storage worker (`copies_request`). And
//! the extra drill a lesson's end offers when the model is sure its skill fell.

use std::sync::Arc;

use openpaint_app::Availability;
use openpaint_core::Artwork;
use openpaint_practice::{
    Answer, BlindPair, Change, CopyRef, Offer, PairAsked, Purpose, Sheet, SkillFit, Standing,
    Within, ask, block_pairs, local_day, older_left, retest_pairs, schedule,
};

use super::{PracticeSheet, unavailable};
use crate::lesson::PairShown;
use crate::pairs::{CardSource, CopyCard};
use crate::refused::Refused;

/// Pairs asked, each with its answer once given.
type Answers = Vec<(PairAsked, Option<Answer>)>;

/// What the open sheet asked of the eye and read for it.
#[derive(Debug, Default)]
pub(crate) struct Asking {
    /// The lesson in hand when the sheet opened, if it was done: its end asks nothing new.
    reopened: Option<usize>,
    /// A Pomodoro's block is in hand (S148): a lesson's end shows no card then, so asks nothing.
    in_block: bool,
    /// The pairs an end asked, with their answers.
    asked: Option<Asked>,
    /// Pairs answered, waiting for the store.
    answered: Vec<BlindPair>,
    /// The copies' cards read, and the copies the store was asked for.
    cards: Vec<Arc<CopyCard>>,
    requested: Vec<CopyRef>,
}

/// The pairs an end asked: the lesson in hand then (they show while it is in hand), the lesson
/// they are kept under (`BlindPair::lesson`: a block's end re-test's, else that one), each with
/// its answer once given.
#[derive(Debug)]
struct Asked {
    hand: usize,
    lesson: (u64, u16),
    pairs: Answers,
}

/// Every other sheet, the pairs answered and those asked today, as the log holds them when the
/// sheet opened or took a lesson, and the clock's offset.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Past {
    pub(crate) sheets: Vec<Sheet>,
    pub(crate) pairs: Vec<BlindPair>,
    pub(crate) skipped: Vec<PairAsked>,
    pub(crate) offset: i32,
}

impl Asking {
    /// For a sheet opening on `record`.
    pub(crate) fn of(record: &Sheet) -> Self {
        let done = record.finished().is_some();
        Self {
            reopened: done.then(|| record.in_hand()),
            ..Self::default()
        }
    }
}

impl PracticeSheet {
    /// `past`'s sheets with this one as it stands.
    pub(super) fn with_this(&self) -> Vec<Sheet> {
        let past = &self.beside.past;
        let others = past
            .sheets
            .iter()
            .filter(|sheet| sheet.id != self.record.id);
        others.cloned().chain([self.record.clone()]).collect()
    }

    /// Whether a Pomodoro's block is in hand (under way, or its end in front), as the log says
    /// each frame before the sheet's intents (S148): a lesson ending then asks no pair.
    pub fn set_in_block(&mut self, in_block: bool) {
        self.asking.in_block = in_block;
    }

    /// The pairs the done lesson in hand asks, chosen once, while no block is in hand.
    pub(super) fn ask_pairs(&mut self) {
        let index = self.record.in_hand();
        let done = self.current().is_none() && self.record.finished().is_some();
        let asked = (self.asking.asked.as_ref()).is_some_and(|asked| asked.hand == index);
        if !done || asked || self.asking.in_block {
            return;
        }
        let past = &self.beside.past;
        let lesson = (self.record.id, u16::try_from(index).unwrap_or(u16::MAX));
        let stored: Vec<(PairAsked, Option<Answer>)> = (past.pairs.iter())
            .filter(|pair| pair.lesson == lesson)
            .map(|pair| {
                (
                    PairAsked {
                        older: pair.older,
                        newer: pair.newer,
                    },
                    Some(pair.answer),
                )
            })
            .collect();
        let pairs = if self.asking.reopened == Some(index) || !self.scored() {
            stored
        } else {
            self.chosen(index)
        };
        self.asking.asked = Some(Asked {
            hand: index,
            lesson,
            pairs,
        });
    }

    /// End at `now` of the block begun at `since`, its end in front (S148): what the block's end
    /// asks (`block_pairs`), beside the lesson in hand; `beside` as the log made it after the cut.
    pub(crate) fn ask_block(&mut self, since: u64, now: u64) {
        let sheets = self.with_this();
        let past = &self.beside.past;
        let within = Within {
            test: self.record.purpose == Purpose::Test,
            pictures: &self.beside.pictures,
            skipped: &past.skipped,
        };
        let today = (local_day(now, past.offset), past.offset);
        let records = (sheets.as_slice(), past.pairs.as_slice());
        let Some((lesson, pairs)) = block_pairs(since, records, today, &within) else {
            return;
        };
        self.asking.asked = Some(Asked {
            hand: self.record.in_hand(),
            lesson,
            pairs: pairs.into_iter().map(|pair| (pair, None)).collect(),
        });
    }

    /// The pairs asked beside the lesson in hand.
    fn shown_asked(&self) -> Option<&Asked> {
        let hand = self.record.in_hand();
        (self.asking.asked.as_ref()).filter(|asked| asked.hand == hand)
    }

    /// The pairs lesson `index`'s end asks, just done.
    fn chosen(&self, index: usize) -> Vec<(PairAsked, Option<Answer>)> {
        let Some(lesson) = self.record.lessons.get(index) else {
            return Vec::new();
        };
        let sheets = self.with_this();
        if let Some((_, true)) = lesson.retest() {
            let asked = retest_pairs(&self.record, index, &sheets);
            return asked.into_iter().map(|pair| (pair, None)).collect();
        }
        let past = &self.beside.past;
        let Some(skill) = lesson.skill().filter(|_| lesson.retest().is_none()) else {
            return Vec::new();
        };
        let today = local_day(lesson.finished.unwrap_or(lesson.started), past.offset);
        let within = Within {
            test: self.record.purpose == Purpose::Test,
            pictures: &self.beside.pictures,
            skipped: &past.skipped,
        };
        let asked = ask(
            &[skill],
            (&sheets, &past.pairs),
            (today, past.offset),
            &within,
        );
        asked.into_iter().map(|pair| (pair, None)).collect()
    }

    /// The pairs the end in front shows (a lesson's, or a block's), as answered so far.
    pub(super) fn pairs_shown(&self) -> Vec<PairShown> {
        let Some(asked) = self.shown_asked() else {
            return Vec::new();
        };
        let sheets = &self.beside.past.sheets;
        (asked.pairs.iter())
            .map(|(pair, answer)| {
                let laid = (pair.newer.find(sheets).map(|(_, laid)| laid))
                    .or_else(|| self.record.laid(usize::from(pair.newer.exercise)));
                PairShown {
                    asked: *pair,
                    older_left: older_left(pair.older, pair.newer),
                    answer: *answer,
                    skill: laid.and_then(|laid| openpaint_practice::Skill::of(laid.exercise.kind)),
                }
            })
            .collect()
    }

    /// The learner answered pair `at` of the end with `answer` at `now`: kept once, for the store.
    pub(super) fn answer_pair(
        &mut self,
        at: usize,
        answer: Answer,
        now: u64,
    ) -> Result<(), Refused> {
        let hand = self.record.in_hand();
        let shown = (self.asking.asked.as_mut()).filter(|asked| asked.hand == hand);
        let Some(asked) = shown else {
            return Err(unavailable(Availability::NothingLeft));
        };
        let Some((pair, held @ None)) = asked.pairs.get_mut(at) else {
            return Err(unavailable(Availability::NothingLeft));
        };
        *held = Some(answer);
        self.asking.answered.push(BlindPair {
            asked: now,
            lesson: asked.lesson,
            older: pair.older,
            newer: pair.newer,
            older_left: older_left(pair.older, pair.newer),
            answer,
        });
        Ok(())
    }

    /// The pairs answered since the store last got them.
    pub fn pairs_request(&mut self) -> Vec<BlindPair> {
        std::mem::take(&mut self.asking.answered)
    }

    /// Every pair the end in front asks, answered or not, each shown since it was asked: what the
    /// log asks no more today.
    #[must_use]
    pub fn offered(&self) -> Vec<PairAsked> {
        let asked = self
            .shown_asked()
            .into_iter()
            .flat_map(|asked| &asked.pairs);
        asked.map(|(pair, _)| *pair).collect()
    }

    /// The copies whose cards the end needs from the store, each once, when it asks pairs; a copy
    /// of this sheet (a re-test's just drawn) is read off `artwork` here and now.
    pub(crate) fn copies_request(&mut self, artwork: &Artwork) -> Option<Vec<CopyRef>> {
        let requested = &self.asking.requested;
        let wanted: Vec<CopyRef> = (self.offered().iter())
            .flat_map(|pair| [pair.older, pair.newer])
            .filter(|copy| !requested.contains(copy))
            .collect();
        if wanted.is_empty() {
            return None;
        }
        self.asking.requested.extend(&wanted);
        let (own, stored): (Vec<CopyRef>, Vec<CopyRef>) = wanted
            .into_iter()
            .partition(|copy| copy.sheet == self.record.id);
        let offset = self.beside.past.offset;
        for copy in own {
            let index = usize::from(copy.exercise);
            let picture = self.sheet.pictures.get(index).cloned().flatten();
            let item = picture.as_ref().map(|picture| &picture.item);
            let image = (self.images.iter())
                .find(|(held, _)| Some(held) == item)
                .and_then(|(_, image)| image.clone());
            let source = CardSource {
                artwork,
                record: &self.record,
                printed: self.sheet.givens.get(index).map_or(&[], Vec::as_slice),
                picture: picture.as_deref(),
                image: image.as_ref(),
            };
            self.asking
                .cards
                .extend(CopyCard::of(&source, copy, offset).map(Arc::new));
        }
        (!stored.is_empty()).then_some(stored)
    }

    /// The store read `cards`.
    pub fn copies_read(&mut self, cards: &[Arc<CopyCard>]) {
        let wanted = self.offered();
        let ours = |card: &&Arc<CopyCard>| {
            wanted
                .iter()
                .any(|pair| pair.older == card.copy || pair.newer == card.copy)
        };
        self.asking.cards.extend(cards.iter().filter(ours).cloned());
    }

    /// The copies' cards read so far.
    pub(super) fn cards(&self) -> Vec<Arc<CopyCard>> {
        self.asking.cards.clone()
    }

    /// An extra drill of the done lesson's skill when its curve is sure it fell: that skill's
    /// lesson at its level, as `standing` (the learner after the lesson) lays it; `sheets` every
    /// sheet with this one.
    pub(super) fn drill(&self, standing: &Standing<'_>, sheets: &[Sheet]) -> Option<Offer> {
        let lesson = self.record.lesson()?;
        let skill = lesson.skill().filter(|_| self.scored())?;
        let past = &self.beside.past;
        let fitted = SkillFit::of(skill, sheets, &past.pairs, past.offset)?;
        matches!(fitted.change(), Some(Change::Fell { .. }))
            .then(|| schedule::offer(skill, standing))
            .flatten()
    }
}
