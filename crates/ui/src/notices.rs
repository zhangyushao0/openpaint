//! What the canvas tells in a pill in the top band, one thing at a time, most urgent first: a
//! failed save or open (with Retry or OK), a refused or failed fill (with OK), then exporting
//! (working, saved, failed), then the reference image (opening, failed). Running work shows a
//! spinner and asks nothing; a result waits for OK. The messages are the user-facing form of the
//! app's typed errors. At fit the pill lies on the desk above the canvas.

use openpaint_app::{
    Delivery, EditorAction, ExportError, ExportIntent, ExportNotice, FillIntent, FillRefusal,
    ImageError, Intent, ReferenceIntent, ReferenceNotice, SessionProjection, StorageIntent,
    StorageNotice,
};

use crate::components::{Place, confirmation, notice, pill, status};

/// One thing to tell.
enum Told {
    /// Work running in the background.
    Running(String),
    /// Something went wrong: the message, the answer's label, whether it can be given now, and
    /// what giving it asks for.
    Failed(String, &'static str, bool, Intent),
    /// Work that succeeded, dismissed with OK.
    Done(String, Intent),
}

/// The pill for `projection`, if there is something to tell, at `place`; its taps go to
/// `taps`. Returns its rectangle in points.
pub(crate) fn notice_pill(
    ctx: &egui::Context,
    projection: &SessionProjection,
    place: Place,
    taps: &mut Vec<Intent>,
) -> Option<egui::Rect> {
    let told = storage(projection)
        .or_else(|| fill(projection))
        .or_else(|| transfers(projection))?;
    let (tapped, rect) = pill(ctx, "notice", place, |ui| match told {
        Told::Running(message) => {
            status(ui, &message);
            None
        }
        Told::Failed(message, action, enabled, intent) => notice(ui, &message, action, enabled)
            .clicked()
            .then_some(intent),
        Told::Done(message, intent) => confirmation(ui, &message).clicked().then_some(intent),
    });
    // egui may run a pass twice; one tap is sent once.
    if let Some(intent) = tapped
        && taps.last() != Some(&intent)
    {
        taps.push(intent);
    }
    Some(rect)
}

/// A failed save (with Retry) or a failed open (with OK).
fn storage(projection: &SessionProjection) -> Option<Told> {
    let (message, action, editor_action, intent) = match projection.storage() {
        StorageNotice::None => return None,
        StorageNotice::SaveFailed { retrying: false } => (
            "Couldn't save. Your drawing is still here.",
            "Retry",
            EditorAction::RetrySave,
            StorageIntent::RetrySave,
        ),
        StorageNotice::SaveFailed { retrying: true } => (
            "Saving…",
            "Retry",
            EditorAction::RetrySave,
            StorageIntent::RetrySave,
        ),
        StorageNotice::OpenFailed => (
            "Couldn't open the last drawing. This is a new one.",
            "OK",
            EditorAction::DismissNotice,
            StorageIntent::Dismiss,
        ),
    };
    let enabled = projection.availability(editor_action).is_available();
    Some(Told::Failed(
        message.to_owned(),
        action,
        enabled,
        Intent::Storage(intent),
    ))
}

/// A refused or failed fill, with OK.
fn fill(projection: &SessionProjection) -> Option<Told> {
    let fill = projection.fill();
    let message = match (fill.refused, fill.failed) {
        (Some(FillRefusal::Hidden), _) => "Can't fill: the layer is hidden.",
        (Some(FillRefusal::Locked), _) => "Can't fill: the layer is locked.",
        (None, true) => "Couldn't fill there. Try again.",
        (None, false) => return None,
    };
    let dismiss = Intent::Fill(FillIntent::Cancel);
    Some(Told::Failed(message.to_owned(), "OK", true, dismiss))
}

/// Exporting, then the reference image.
fn transfers(projection: &SessionProjection) -> Option<Told> {
    let transfers = projection.transfers()?;
    let dismiss_export = Intent::Export(ExportIntent::Dismiss);
    match &transfers.export {
        ExportNotice::Working { format, delivery } => {
            let doing = match delivery {
                Delivery::Save => "Saving",
                Delivery::Share => "Preparing",
            };
            return Some(Told::Running(format!("{doing} {}…", format.name())));
        }
        ExportNotice::Saved { name } => {
            return Some(Told::Done(format!("Saved “{name}”."), dismiss_export));
        }
        ExportNotice::Failed {
            format,
            delivery,
            error,
        } => {
            let message = export_failed(format.name(), *delivery, error);
            return Some(Told::Failed(message, "OK", true, dismiss_export));
        }
        ExportNotice::None => {}
    }
    match &transfers.reference_notice {
        ReferenceNotice::Opening => Some(Told::Running("Opening the image…".to_owned())),
        ReferenceNotice::Failed(error) => Some(Told::Failed(
            format!("Couldn't open the image: {}", image_failed(error)),
            "OK",
            true,
            Intent::Reference(ReferenceIntent::Dismiss),
        )),
        ReferenceNotice::None => None,
    }
}

fn export_failed(format: &str, delivery: Delivery, error: &ExportError) -> String {
    match (error, delivery) {
        (ExportError::Picker(_), Delivery::Save) => {
            "Couldn't open the save dialog. Nothing was saved.".to_owned()
        }
        (ExportError::Picker(_) | ExportError::Write(_), Delivery::Share) => {
            format!("Couldn't prepare the {format} to share.")
        }
        (ExportError::Render, _) => "Couldn't render the artwork. Nothing was saved.".to_owned(),
        (ExportError::Encode, _) => format!("Couldn't make the {format} file."),
        (ExportError::Write(_), Delivery::Save) => {
            format!("Couldn't write the {format} file. Is there space left?")
        }
    }
}

fn image_failed(error: &ImageError) -> &'static str {
    match error {
        ImageError::Picker(_) => "the picker failed.",
        ImageError::Read(_) => "the file could not be read.",
        ImageError::Unsupported => "only PNG and JPEG images can be used.",
        ImageError::Damaged => "the file is damaged.",
        ImageError::TooLarge => "it is larger than 50 megapixels.",
    }
}
