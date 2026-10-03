//! What the top bar tells on the canvas, one thing at a time, most urgent first: a failed save
//! or open (with Retry or OK), then exporting (working, saved, failed), then the reference image
//! (opening, failed). Running work shows a spinner and asks nothing; a result waits for OK. The
//! messages are the user-facing form of the app's typed errors.

use openpaint_app::{
    Delivery, EditorAction, ExportError, ExportIntent, ExportNotice, ImageError, Intent,
    ReferenceIntent, ReferenceNotice, SessionProjection, StorageIntent, StorageNotice,
};

use crate::components::{confirmation, notice, status};

/// The notice for `projection`, if any, at the right end of the current row; its taps go to
/// `taps`.
pub(crate) fn top_bar_notice(
    ui: &mut egui::Ui,
    projection: &SessionProjection,
    taps: &mut Vec<Intent>,
) {
    if storage(ui, projection, taps) {
        return;
    }
    let Some(transfers) = projection.transfers() else {
        return;
    };
    let ok = |ui: &mut egui::Ui, message: &str, intent: Intent, taps: &mut Vec<Intent>| {
        if notice(ui, message, "OK", true).clicked() && taps.last() != Some(&intent) {
            taps.push(intent);
        }
    };
    let dismiss_export = Intent::Export(ExportIntent::Dismiss);
    match &transfers.export {
        ExportNotice::Working { format, delivery } => {
            let doing = match delivery {
                Delivery::Save => "Saving",
                Delivery::Share => "Preparing",
            };
            status(ui, &format!("{doing} {}…", format.name()));
            return;
        }
        ExportNotice::Saved { name } => {
            let message = format!("Saved “{name}”.");
            if confirmation(ui, &message).clicked() && taps.last() != Some(&dismiss_export) {
                taps.push(dismiss_export);
            }
            return;
        }
        ExportNotice::Failed {
            format,
            delivery,
            error,
        } => {
            let message = export_failed(format.name(), *delivery, error);
            return ok(ui, &message, dismiss_export, taps);
        }
        ExportNotice::None => {}
    }
    match &transfers.reference_notice {
        ReferenceNotice::Opening => status(ui, "Opening the image…"),
        ReferenceNotice::Failed(error) => {
            let message = format!("Couldn't open the image: {}", image_failed(error));
            ok(
                ui,
                &message,
                Intent::Reference(ReferenceIntent::Dismiss),
                taps,
            );
        }
        ReferenceNotice::None => {}
    }
}

/// The notice for a failed save (with Retry) or a failed open (with OK). Returns whether one
/// showed.
fn storage(ui: &mut egui::Ui, projection: &SessionProjection, taps: &mut Vec<Intent>) -> bool {
    let (message, action, editor_action, intent) = match projection.storage() {
        StorageNotice::None => return false,
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
    if notice(ui, message, action, enabled).clicked() {
        taps.push(Intent::Storage(intent));
    }
    true
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
