//! What the canvas tells in a pill in the top band, one thing at a time, most urgent first: a
//! failed save or open (with Retry or OK), that the display was reset (the GPU device was
//! lost and made again; with OK, and a new stroke lets it go), a refused or failed fill (with
//! OK), then exporting (working, saved, failed), then the reference image and a picture to
//! insert (opening, failed). Running work shows a spinner and asks nothing; a result waits for
//! OK. The messages are the user-facing form of the app's typed errors, in the pass's language
//! (`text/notices.rs`). At fit the pill lies on the desk above the canvas.

use openpaint_app::{
    AdjustIntent, EditorAction, ExportIntent, ExportNotice, FillIntent, FillRefusal, InsertIntent,
    Intent, Language, PracticeIntent, PracticeNotice, ReferenceIntent, ReferenceNotice,
    SessionProjection, StorageIntent, StorageNotice, TransformIntent, TransformRefusal,
};

use crate::components::{Place, Source, confirmation, notice, pill, status};
use crate::taps::push_once;
use crate::text::{self as t, Refused, Text, Words};

/// One thing to tell.
enum Told {
    /// Work running in the background.
    Running(String),
    /// Something went wrong: the message, the answer's words, whether it can be given now, and
    /// what giving it asks for.
    Failed(String, Text, bool, Intent),
    /// Work that succeeded, dismissed with OK.
    Done(String, Intent),
    /// The display was reset, dismissed with OK; nothing is asked of the app.
    Reset,
}

/// The pill for `projection`, if there is something to tell, at `place`, with the notice that the
/// display was reset while `reset` holds (OK clears it); its taps go to `taps`. Returns its
/// rectangle in points.
pub(crate) fn notice_pill(
    ctx: &egui::Context,
    projection: &SessionProjection,
    (place, reset): (Place, &mut bool),
    taps: &mut Vec<Intent>,
) -> Option<egui::Rect> {
    let language = t::language(ctx);
    let told = storage(projection, language)
        .or_else(|| (*reset).then_some(Told::Reset))
        .or_else(|| practice(projection, language))
        .or_else(|| fill(projection, language))
        .or_else(|| transform(projection, language))
        .or_else(|| adjust(projection, language))
        .or_else(|| transfers(projection, language))?;
    // It comes out of its right end, beside the right island.
    let source = Source::Within(egui::Align::Max);
    let (tapped, rect) = pill(ctx, "notice", place, source, |ui| match told {
        Told::Running(message) => {
            status(ui, Words::made(&message));
            None
        }
        Told::Failed(message, action, enabled, intent) => {
            notice(ui, Words::made(&message), action, enabled)
                .clicked()
                .then_some(Some(intent))
        }
        Told::Done(message, intent) => confirmation(ui, Words::made(&message))
            .clicked()
            .then_some(Some(intent)),
        Told::Reset => notice(ui, t::DISPLAY_RESET, t::OK, true)
            .clicked()
            .then_some(None),
    });
    match tapped {
        Some(Some(intent)) => push_once(taps, intent),
        Some(None) => *reset = false,
        None => {}
    }
    Some(rect)
}

/// What a notice answered with OK says.
fn with_ok(message: String, dismiss: Intent) -> Told {
    Told::Failed(message, t::OK, true, dismiss)
}

/// A failed save (with Retry) or a failed open (with OK).
fn storage(projection: &SessionProjection, language: Language) -> Option<Told> {
    let (message, action, editor_action, intent) = match projection.storage() {
        StorageNotice::None => return None,
        StorageNotice::SaveFailed { retrying: false } => (
            t::SAVE_FAILED,
            t::RETRY,
            EditorAction::RetrySave,
            StorageIntent::RetrySave,
        ),
        StorageNotice::SaveFailed { retrying: true } => (
            t::SAVING,
            t::RETRY,
            EditorAction::RetrySave,
            StorageIntent::RetrySave,
        ),
        StorageNotice::OpenFailed => (
            t::LAST_DRAWING_FAILED,
            t::OK,
            EditorAction::DismissNotice,
            StorageIntent::Dismiss,
        ),
    };
    let enabled = projection.availability(editor_action).is_available();
    Some(Told::Failed(
        message.get(language).to_owned(),
        action,
        enabled,
        Intent::Storage(intent),
    ))
}

/// Why a practice sheet took no contact, with OK.
fn practice(projection: &SessionProjection, language: Language) -> Option<Told> {
    let message = match projection.practice()?.notice? {
        PracticeNotice::DrawInFrame => t::DRAW_IN_FRAME.get(language).to_owned(),
        PracticeNotice::DrawInCell => t::DRAW_IN_CELL.get(language).to_owned(),
        PracticeNotice::PickAfterCompare => t::PICK_AFTER_COMPARE.get(language).to_owned(),
        PracticeNotice::OriginalLocked => t::ORIGINAL_LOCKED.get(language).to_owned(),
        PracticeNotice::CannotFill(refusal) => t::refused(language, Refused::Fill, refusal),
        PracticeNotice::Done => t::SESSION_DONE.get(language).to_owned(),
    };
    Some(with_ok(message, Intent::Practice(PracticeIntent::Dismiss)))
}

/// A refused or failed fill, with OK.
fn fill(projection: &SessionProjection, language: Language) -> Option<Told> {
    let fill = projection.fill();
    let message = match (fill.refused, fill.failed) {
        (Some(FillRefusal::Layer(refusal)), _) => t::refused(language, Refused::Fill, refusal),
        // On a practice sheet the empty frame is what a fill stays inside.
        (Some(FillRefusal::OutsideSelection), _) if projection.practice().is_some() => {
            t::DROP_IN_FRAME.get(language).to_owned()
        }
        (Some(FillRefusal::OutsideSelection), _) => t::DROP_IN_SELECTION.get(language).to_owned(),
        (None, true) => t::FILL_FAILED.get(language).to_owned(),
        (None, false) => return None,
    };
    Some(with_ok(message, Intent::Fill(FillIntent::Cancel)))
}

/// Why the Transform tool did not turn on, with OK.
fn transform(projection: &SessionProjection, language: Language) -> Option<Told> {
    let refusal = projection.transform().refused?;
    let message = match (refusal, t::transform_refused(refusal)) {
        (_, Some(text)) => text.get(language).to_owned(),
        (TransformRefusal::Layer(refusal), None) => {
            t::refused(language, Refused::Transform, refusal)
        }
        (_, None) => return None,
    };
    Some(with_ok(
        message,
        Intent::Transform(TransformIntent::Tool(false)),
    ))
}

/// Why an adjustment did not start, with OK.
fn adjust(projection: &SessionProjection, language: Language) -> Option<Told> {
    let message = t::refused(language, Refused::Adjust, projection.adjust().refused?);
    Some(with_ok(message, Intent::Adjust(AdjustIntent::Dismiss)))
}

/// Exporting, then the reference image.
fn transfers(projection: &SessionProjection, language: Language) -> Option<Told> {
    let transfers = projection.transfers()?;
    let dismiss_export = Intent::Export(ExportIntent::Dismiss);
    match &transfers.export {
        ExportNotice::Working { format, delivery } => {
            let message = t::exporting(language, *delivery, format.name());
            return Some(Told::Running(message));
        }
        ExportNotice::Saved { name } => {
            return Some(Told::Done(t::exported(language, name), dismiss_export));
        }
        ExportNotice::Failed {
            format,
            delivery,
            error,
        } => {
            let message = t::export_failed(language, (format.name(), *delivery), error);
            return Some(with_ok(message, dismiss_export));
        }
        ExportNotice::None => {}
    }
    let reference = Intent::Reference(ReferenceIntent::Dismiss);
    let insert = Intent::Insert(InsertIntent::Dismiss);
    image(&transfers.reference_notice, (language, false), reference)
        .or_else(|| image(&transfers.insert_notice, (language, true), insert))
}

/// Opening a reference image or a picture to insert, or why it failed.
fn image(
    notice: &ReferenceNotice,
    (language, insert): (Language, bool),
    dismiss: Intent,
) -> Option<Told> {
    match notice {
        ReferenceNotice::Opening => Some(Told::Running(t::OPENING_IMAGE.get(language).to_owned())),
        ReferenceNotice::Failed(error) => {
            Some(with_ok(t::image_failed(language, insert, error), dismiss))
        }
        ReferenceNotice::None => None,
    }
}
