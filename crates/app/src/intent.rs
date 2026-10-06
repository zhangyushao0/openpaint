//! `Intent`: everything the UI can ask of the session. Each family lives in its own module with
//! its handler and tests.

use crate::adjust::AdjustIntent;
use crate::clipboard::ClipboardAction;
use crate::export::ExportIntent;
use crate::fill::FillIntent;
use crate::gallery::GalleryIntent;
use crate::history::HistoryIntent;
use crate::insert::InsertIntent;
use crate::layers::LayerIntent;
use crate::reference::ReferenceIntent;
use crate::selection::SelectionIntent;
use crate::storage::StorageIntent;
use crate::stroke::StrokeIntent;
use crate::tool_intent::{BrushIntent, ToolIntent};
use crate::transform::TransformIntent;
use crate::view::ViewIntent;

#[derive(Debug, Clone, PartialEq)]
pub enum Intent {
    Stroke(StrokeIntent),
    View(ViewIntent),
    History(HistoryIntent),
    Storage(StorageIntent),
    Layer(LayerIntent),
    Fill(FillIntent),
    Selection(SelectionIntent),
    Transform(TransformIntent),
    Adjust(AdjustIntent),
    Gallery(GalleryIntent),
    Brush(BrushIntent),
    Tool(ToolIntent),
    Export(ExportIntent),
    Reference(ReferenceIntent),
    Insert(InsertIntent),
    /// The copy menu (S40); the runtime hands it to the session with the app's clipboard.
    Clipboard(ClipboardAction),
}
