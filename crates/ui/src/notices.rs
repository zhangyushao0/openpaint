//! What the canvas tells in a pill in the top band, one thing at a time, most urgent first: a
//! failed save or open (with Retry or OK), that the display was reset (the GPU device was
//! lost and made again; with OK, and a new stroke lets it go), what a mode's sheet tells
//! (`CanvasHook::notice`, with OK), a refused or failed fill (with OK), then exporting (working,
//! saved, failed), then the reference image and a picture to insert (opening, failed). Running
//! work shows a spinner and asks nothing; a result waits for OK. The messages are the
//! user-facing form of the app's typed errors, in the pass's language (`text/notices.rs`). At fit
//! the pill lies on the desk above the canvas.

use openpaint_app::{
    AdjustIntent, EditorAction, ExportIntent, ExportNotice, FillIntent, FillRefusal, InsertIntent,
    Intent, Language, ReferenceIntent, ReferenceNotice, SessionProjection, StorageIntent,
    StorageNotice, TransformIntent, TransformRefusal,
};

use crate::components::{Place, Source, confirmation, notice, pill, status};
use crate::hooks::CanvasHook;
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
    /// What a mode's sheet tells, dismissed with OK.
    Sheet(String),
}

/// What OK or Retry asks.
enum Answer {
    Ask(Intent),
    Reset,
    Sheet,
}

/// The pill for `projection` (on a mode's sheet, `hook`), if there is something to tell, at
/// `place`, with the notice that the display was reset while `reset` holds (OK clears it); its
/// taps go to `taps`. Returns its rectangle in points.
pub(crate) fn notice_pill(
    ctx: &egui::Context,
    projection: &SessionProjection,
    (place, reset): (Place, &mut bool),
    (taps, hook): (&mut Vec<Intent>, Option<&mut (dyn CanvasHook + '_)>),
) -> Option<egui::Rect> {
    let language = t::language(ctx);
    let told = storage(projection, language)
        .or_else(|| (*reset).then_some(Told::Reset))
        .or_else(|| Some(Told::Sheet(hook.as_deref()?.notice(language)?)))
        .or_else(|| fill(projection, hook.as_deref(), language))
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
                .then_some(Answer::Ask(intent))
        }
        Told::Done(message, intent) => confirmation(ui, Words::made(&message))
            .clicked()
            .then_some(Answer::Ask(intent)),
        Told::Reset => notice(ui, t::DISPLAY_RESET, t::OK, true)
            .clicked()
            .then_some(Answer::Reset),
        Told::Sheet(message) => notice(ui, Words::made(&message), t::OK, true)
            .clicked()
            .then_some(Answer::Sheet),
    });
    match (tapped, hook) {
        (Some(Answer::Ask(intent)), _) => push_once(taps, intent),
        (Some(Answer::Reset), _) => *reset = false,
        (Some(Answer::Sheet), Some(hook)) => hook.dismiss(),
        _ => {}
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

/// A refused or failed fill, with OK; on a mode's sheet (`hook`) what it says of a fill outside
/// its clip.
fn fill(
    projection: &SessionProjection,
    hook: Option<&(dyn CanvasHook + '_)>,
    language: Language,
) -> Option<Told> {
    let fill = projection.fill();
    let outside = hook.and_then(CanvasHook::outside);
    let message = match (fill.refused, fill.failed) {
        (Some(FillRefusal::Layer(refusal)), _) => t::refused(language, Refused::Fill, refusal),
        (Some(FillRefusal::OutsideSelection), _) => outside
            .unwrap_or(t::DROP_IN_SELECTION)
            .get(language)
            .to_owned(),
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
