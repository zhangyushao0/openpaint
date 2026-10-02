//! `Intent`: everything the UI can ask of the session. Each family lives in its own module with
//! its handler and tests.

use crate::history::HistoryIntent;
use crate::storage::StorageIntent;
use crate::stroke::StrokeIntent;

#[derive(Debug, Clone, PartialEq)]
pub enum Intent {
    Stroke(StrokeIntent),
    History(HistoryIntent),
    Storage(StorageIntent),
}
