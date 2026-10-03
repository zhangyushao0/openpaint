//! `Intent`: everything the UI can ask of the session. Each family lives in its own module with
//! its handler and tests.

use crate::export::ExportIntent;
use crate::fill::FillIntent;
use crate::gallery::GalleryIntent;
use crate::history::HistoryIntent;
use crate::layers::LayerIntent;
use crate::reference::ReferenceIntent;
use crate::storage::StorageIntent;
use crate::stroke::StrokeIntent;
use crate::toolbox::{BrushIntent, ToolIntent};
use crate::view::ViewIntent;

#[derive(Debug, Clone, PartialEq)]
pub enum Intent {
    Stroke(StrokeIntent),
    View(ViewIntent),
    History(HistoryIntent),
    Storage(StorageIntent),
    Layer(LayerIntent),
    Fill(FillIntent),
    Gallery(GalleryIntent),
    Brush(BrushIntent),
    Tool(ToolIntent),
    Export(ExportIntent),
    Reference(ReferenceIntent),
}
