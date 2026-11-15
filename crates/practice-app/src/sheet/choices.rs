//! What a card asks the user to choose before Compare (S112, `openpaint_practice::Choices`): the
//! chips of its row toggled and the picture of its row chosen, as the shown exercise's original
//! lays its card out (`Original::task`); fixed once the exercise was first compared (D99: the eye
//! before the answer), and as its first Compare kept them when the sheet shows it again. And the
//! view a comparison's card shows of its views (`CardItem::Views`).

use openpaint_practice::{CardItem, Choices};

use super::{PracticeSheet, unavailable};
use crate::refused::Refused;
use openpaint_app::Availability;

impl PracticeSheet {
    /// An exercise came to show: its choices as its first Compare kept them, else none yet; its
    /// comparison's first view.
    pub(super) fn choices_shown(&mut self) {
        let first = self.current().and_then(|index| self.record.first(index));
        let kept = first.and_then(|first| Choices::kept(&first.metrics));
        self.painting_state.choices = kept.unwrap_or_default();
        self.painting_state.view = 0;
    }

    /// What the shown exercise's card asks while it is made (`Original::task`).
    pub(super) fn task(&self) -> Vec<CardItem> {
        let original = self.current().and_then(|index| self.original(index));
        original.map(|original| original.task()).unwrap_or_default()
    }

    /// How many choices the task card's row of `row` holds (none: no such row), while the shown
    /// exercise was not compared yet.
    fn choosable(&self, row: fn(&CardItem) -> Option<usize>) -> Option<usize> {
        let index = self.current()?;
        if self.record.first(index).is_some() {
            return None;
        }
        self.task().iter().find_map(row)
    }

    /// Chip `chip` of the card's row of chips turned.
    pub(super) fn toggle(&mut self, chip: u8) -> Result<(), Refused> {
        let chips = |item: &CardItem| match item {
            CardItem::Toggles(chips) => Some(chips.len()),
            _ => None,
        };
        if usize::from(chip) >= self.choosable(chips).unwrap_or(0) {
            return Err(unavailable(Availability::NothingLeft));
        }
        self.painting_state.choices = self.painting_state.choices.toggled(chip);
        Ok(())
    }

    /// Picture `shape` of the card's row of pictures chosen.
    pub(super) fn choose_shape(&mut self, shape: u8) -> Result<(), Refused> {
        let shapes = |item: &CardItem| match item {
            CardItem::Shapes(shapes) => Some(shapes.len()),
            _ => None,
        };
        if usize::from(shape) >= self.choosable(shapes).unwrap_or(0) {
            return Err(unavailable(Availability::NothingLeft));
        }
        self.painting_state.choices.shape = Some(shape);
        Ok(())
    }

    /// View `view` of the shown comparison's views shown.
    pub(super) fn choose_view(&mut self, view: u8) -> Result<(), Refused> {
        let card = self.overlay().map(|(_, card, _)| card).unwrap_or_default();
        let views = card.iter().find_map(|item| match item {
            CardItem::Views(views) => Some(views.len()),
            _ => None,
        });
        if usize::from(view) >= views.unwrap_or(0) {
            return Err(unavailable(Availability::NothingLeft));
        }
        self.painting_state.view = usize::from(view);
        Ok(())
    }
}
