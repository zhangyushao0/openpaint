//! The export and reference flows: what they ask the platform for, how its answers and the
//! worker's results move them on, what the screen is told, and that one picker runs at a time.

use openpaint_app::{
    AppError, Availability, Delivery, EditorAction, ExportError, ExportIntent, ExportNotice,
    ExportScale, HostAnswer, HostRequest, ImageError, ImageFormat, ImageSink, ImageSource,
    PortError, ReferenceImage, ReferenceIntent, ReferenceNotice, Title, TransferJob, Transfers,
};
use openpaint_core::{Artwork, CanvasSize};

struct Sink;

impl ImageSink for Sink {
    fn write(self: Box<Self>, _bytes: &[u8]) -> Result<(), PortError> {
        Ok(())
    }
}

struct Source;

impl ImageSource for Source {
    fn read(self: Box<Self>, _limit: u64) -> Result<Vec<u8>, PortError> {
        Ok(Vec::new())
    }
}

fn artwork() -> Artwork {
    Artwork::new(CanvasSize::new(64, 48).expect("size"))
}

fn start(transfers: &mut Transfers, format: ImageFormat, delivery: Delivery) {
    start_at(transfers, format, delivery, ExportScale::One).expect("export starts");
}

fn start_at(
    transfers: &mut Transfers,
    format: ImageFormat,
    delivery: Delivery,
    scale: ExportScale,
) -> Result<(), AppError> {
    let artwork = artwork();
    let title = Title::new("Harbour").expect("title");
    transfers.export(
        &ExportIntent::Start {
            format,
            delivery,
            scale,
        },
        Some((&artwork, &title)),
    )
}

#[test]
fn the_scale_travels_to_the_job_and_jpeg_goes_up_to_two_x() {
    let mut transfers = Transfers::default();
    let jpeg = start_at(
        &mut transfers,
        ImageFormat::Jpeg,
        Delivery::Save,
        ExportScale::Four,
    );
    assert!(jpeg.is_err(), "no 4x JPEG");
    assert_eq!(transfers.request(), None, "nothing asked");
    assert!(!ExportScale::Four.fits(ImageFormat::Jpeg));
    assert!(ExportScale::Two.fits(ImageFormat::Jpeg) && ExportScale::Four.fits(ImageFormat::Png));
    start_at(
        &mut transfers,
        ImageFormat::Png,
        Delivery::Save,
        ExportScale::Four,
    )
    .expect("4x PNG");
    let Some(TransferJob::Export(job)) = transfers.answer(HostAnswer::File(Box::new(Sink))) else {
        panic!("a file starts the export");
    };
    assert_eq!((job.scale, job.scale.factor()), (ExportScale::Four, 4));
    assert_eq!(ExportScale::default(), ExportScale::One, "1x unless chosen");
}

fn image() -> ReferenceImage {
    ReferenceImage::fitted([1, 1], &[200, 40, 40, 255]).expect("image")
}

#[test]
fn saving_asks_for_a_file_renders_it_and_reports_the_name() {
    let mut transfers = Transfers::default();
    start(&mut transfers, ImageFormat::Png, Delivery::Save);
    assert_eq!(
        transfers.request(),
        Some(HostRequest::SaveFile {
            name: "Harbour.png".into(),
            format: ImageFormat::Png
        })
    );
    assert_eq!(transfers.request(), None, "asked once");
    let projection = transfers.projection();
    assert_eq!(projection.available, Availability::Running);
    assert!(matches!(projection.export, ExportNotice::Working { .. }));
    let Some(TransferJob::Export(job)) = transfers.answer(HostAnswer::File(Box::new(Sink))) else {
        panic!("a file starts the export");
    };
    assert_eq!(job.format, ImageFormat::Png);
    assert!(job.artwork.canvas() == artwork().canvas());
    let again = ExportIntent::Start {
        format: ImageFormat::Jpeg,
        delivery: Delivery::Save,
        scale: ExportScale::One,
    };
    let refused = transfers.export(&again, Some((&artwork(), &Title::untitled())));
    assert_eq!(
        refused,
        Err(AppError::Unavailable {
            action: EditorAction::Export,
            availability: Availability::Running
        })
    );
    transfers.exported(Ok(()));
    let saved = ExportNotice::Saved {
        name: "Harbour.png".into(),
    };
    assert_eq!(transfers.projection().export, saved);
    assert_eq!(transfers.projection().available, Availability::Available);
    transfers.interrupt();
    assert_eq!(transfers.projection().export, ExportNotice::None);
}

#[test]
fn cancelling_the_dialog_ends_the_export_without_a_word() {
    let mut transfers = Transfers::default();
    start(&mut transfers, ImageFormat::Jpeg, Delivery::Save);
    assert!(transfers.waiting());
    assert!(transfers.answer(HostAnswer::Cancelled).is_none());
    assert_eq!(transfers.projection().export, ExportNotice::None);
    assert_eq!(transfers.projection().available, Availability::Available);
    assert!(!transfers.waiting());
    assert!(
        transfers.answer(HostAnswer::Cancelled).is_none(),
        "nobody waits"
    );
}

#[test]
fn sharing_writes_the_file_then_opens_the_share_sheet() {
    let mut transfers = Transfers::default();
    start(&mut transfers, ImageFormat::Jpeg, Delivery::Share);
    let name = "Harbour.jpg".to_owned();
    let format = ImageFormat::Jpeg;
    assert_eq!(
        transfers.request(),
        Some(HostRequest::ShareFile {
            name: name.clone(),
            format
        })
    );
    assert!(transfers.answer(HostAnswer::File(Box::new(Sink))).is_some());
    transfers.exported(Ok(()));
    assert_eq!(
        transfers.request(),
        Some(HostRequest::Send { name, format })
    );
    assert_eq!(transfers.projection().export, ExportNotice::None);
}

#[test]
fn failures_stay_until_dismissed() {
    let mut transfers = Transfers::default();
    start(&mut transfers, ImageFormat::Png, Delivery::Save);
    transfers.answer(HostAnswer::File(Box::new(Sink)));
    let full = PortError::Device {
        detail: "no space left".into(),
    };
    transfers.exported(Err(ExportError::Write(full.clone())));
    let failed = ExportNotice::Failed {
        format: ImageFormat::Png,
        delivery: Delivery::Save,
        error: ExportError::Write(full.clone()),
    };
    assert_eq!(transfers.projection().export, failed);
    transfers.interrupt();
    assert_eq!(
        transfers.projection().export,
        failed,
        "a failure waits for OK"
    );
    transfers
        .export(&ExportIntent::Dismiss, None)
        .expect("dismiss");
    assert_eq!(transfers.projection().export, ExportNotice::None);
    start(&mut transfers, ImageFormat::Png, Delivery::Save);
    transfers.answer(HostAnswer::Failed(full.clone()));
    assert!(matches!(
        transfers.projection().export,
        ExportNotice::Failed {
            error: ExportError::Picker(_),
            ..
        }
    ));
    let none = transfers.export(
        &ExportIntent::Start {
            format: ImageFormat::Png,
            delivery: Delivery::Save,
            scale: ExportScale::One,
        },
        None,
    );
    assert!(none.is_err(), "nothing to export without an artwork");
}

#[test]
fn the_reference_opens_hides_and_shows_again_without_asking() {
    let mut transfers = Transfers::default();
    transfers
        .reference(&ReferenceIntent::Show(true))
        .expect("show");
    assert_eq!(transfers.request(), Some(HostRequest::OpenImage));
    assert_eq!(
        transfers.projection().reference_notice,
        ReferenceNotice::Opening
    );
    let job = transfers.answer(HostAnswer::Image(Box::new(Source)));
    assert!(matches!(job, Some(TransferJob::Open(_))));
    transfers.opened(Ok(image()));
    let projection = transfers.projection();
    assert!(projection.reference_shown);
    let shown = projection.reference.expect("image");
    transfers
        .reference(&ReferenceIntent::Show(false))
        .expect("hide");
    assert!(!transfers.projection().reference_shown);
    transfers
        .reference(&ReferenceIntent::Show(true))
        .expect("show");
    assert_eq!(transfers.request(), None, "the image of this session");
    assert_eq!(transfers.projection().reference, Some(shown));
    assert!(transfers.projection().reference_shown);
}

#[test]
fn a_failed_or_cancelled_choice_keeps_the_image_before() {
    let mut transfers = Transfers::default();
    transfers
        .reference(&ReferenceIntent::Choose)
        .expect("choose");
    transfers.answer(HostAnswer::Image(Box::new(Source)));
    transfers.opened(Ok(image()));
    let before = transfers.projection().reference;
    transfers
        .reference(&ReferenceIntent::Choose)
        .expect("choose");
    transfers.answer(HostAnswer::Cancelled);
    assert_eq!(transfers.projection().reference, before);
    assert_eq!(
        transfers.projection().reference_notice,
        ReferenceNotice::None
    );
    transfers
        .reference(&ReferenceIntent::Choose)
        .expect("choose");
    transfers.answer(HostAnswer::Image(Box::new(Source)));
    transfers.opened(Err(ImageError::Unsupported));
    let projection = transfers.projection();
    assert_eq!(projection.reference, before);
    assert_eq!(
        projection.reference_notice,
        ReferenceNotice::Failed(ImageError::Unsupported)
    );
    transfers
        .reference(&ReferenceIntent::Dismiss)
        .expect("dismiss");
    assert_eq!(
        transfers.projection().reference_notice,
        ReferenceNotice::None
    );
}

#[test]
fn one_system_picker_at_a_time() {
    let mut transfers = Transfers::default();
    start(&mut transfers, ImageFormat::Png, Delivery::Save);
    let refused = transfers.reference(&ReferenceIntent::Choose);
    assert!(refused.is_err(), "the save dialog is open");
    transfers
        .reference(&ReferenceIntent::Show(false))
        .expect("hiding is always possible");
    let job = transfers.answer(HostAnswer::Image(Box::new(Source)));
    assert!(job.is_none(), "an image answers no save dialog");
    assert!(matches!(
        transfers.projection().export,
        ExportNotice::Failed { .. }
    ));
}
