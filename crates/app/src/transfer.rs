//! Files in and out through the system: the image formats, the ports the platform implements (a
//! file the user chose to write, an image the user chose to read), what the app asks the
//! platform to show (`HostRequest`) and how it answers (`HostAnswer`). The pickers and the share
//! sheet are the platform's; the app never sees a path or a URI. `Transfers` holds the export and
//! reference flows and lets one system picker be open at a time.

use std::collections::VecDeque;
use std::fmt;

use openpaint_core::Artwork;

use crate::error::AppError;
use crate::export::{Export, ExportIntent, ExportJob, ExportNotice};
use crate::project::PortError;
use crate::projection::{Availability, EditorAction};
use crate::reference::{Reference, ReferenceImage, ReferenceIntent, ReferenceNotice};
use crate::title::Title;

/// A picture file format the app writes and reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    /// Lossless; every pixel exactly as on screen.
    Png,
    /// Smaller, lossy, no transparency.
    Jpeg,
}

impl ImageFormat {
    /// The file name extension, without the dot.
    #[must_use]
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
        }
    }

    /// The media type the system files and shares it under.
    #[must_use]
    pub const fn mime(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
        }
    }

    /// The name the user reads.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Png => "PNG",
            Self::Jpeg => "JPEG",
        }
    }
}

/// Where an exported image goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Delivery {
    /// Into a file the user chooses with the system's save dialog.
    Save,
    /// To another app through the system's share sheet.
    Share,
}

/// A file the platform opened for writing an export. Writing replaces what it held.
///
/// # Errors
/// `PortError::Device` when the system refuses the write (storage full, file gone).
pub trait ImageSink: Send {
    fn write(self: Box<Self>, bytes: &[u8]) -> Result<(), PortError>;
}

/// A file the user chose to read an image from.
///
/// # Errors
/// `PortError::Device` when the system refuses the read.
pub trait ImageSource: Send {
    /// The file's bytes, at most `limit + 1` of them: a longer file is cut there, so the caller
    /// can tell it is too large without reading it whole.
    fn read(self: Box<Self>, limit: u64) -> Result<Vec<u8>, PortError>;
}

/// What the app asks the platform to show or make. The runtime hands these to the host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostRequest {
    /// The system's dialog for saving a new file, suggesting `name`; answered with
    /// `HostAnswer::File`, `Cancelled` or `Failed`.
    SaveFile { name: String, format: ImageFormat },
    /// A new file named `name` in the platform's place for files it shares; answered at once
    /// with `HostAnswer::File` or `Failed`.
    ShareFile { name: String, format: ImageFormat },
    /// The system's share sheet for the file made for `ShareFile` and now written; no answer.
    Send { name: String, format: ImageFormat },
    /// The system's picker for one PNG or JPEG image; answered with `HostAnswer::Image`,
    /// `Cancelled` or `Failed`.
    OpenImage,
}

/// How the platform answered the last request that asks for an answer.
pub enum HostAnswer {
    /// The file to write: chosen in the save dialog, or made for sharing.
    File(Box<dyn ImageSink>),
    /// The image the user chose.
    Image(Box<dyn ImageSource>),
    /// The user closed the picker without choosing.
    Cancelled,
    /// The system could not show the picker or open what was chosen.
    Failed(PortError),
}

impl fmt::Debug for HostAnswer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::File(_) => f.write_str("File"),
            Self::Image(_) => f.write_str("Image"),
            Self::Cancelled => f.write_str("Cancelled"),
            Self::Failed(error) => write!(f, "Failed({error})"),
        }
    }
}

/// Work the runtime does on its transfer worker, off the UI thread.
#[derive(Debug)]
pub enum TransferJob {
    /// Render, encode and write an artwork; report with `Transfers::exported`.
    Export(ExportJob),
    /// Read and decode the chosen image; report with `Transfers::opened`.
    Open(OpenJob),
}

/// The chosen reference image, to read and decode.
pub struct OpenJob {
    pub source: Box<dyn ImageSource>,
}

impl fmt::Debug for OpenJob {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("OpenJob")
    }
}

/// What the screen shows of exporting and the reference image.
#[derive(Debug, Clone, PartialEq)]
pub struct TransferProjection {
    pub export: ExportNotice,
    /// The reference image of this session, shown or not.
    pub reference: Option<ReferenceImage>,
    /// The floating reference window is open.
    pub reference_shown: bool,
    pub reference_notice: ReferenceNotice,
    /// Whether an export or a new reference image can start: `Running` while a system picker
    /// is open or a file is being written or read.
    pub available: Availability,
}

/// The export and reference flows of the app. They belong to the app, not to one artwork: a
/// reference stays when another artwork opens; an export finishes after the canvas is left.
#[derive(Debug, Default)]
pub struct Transfers {
    export: Export,
    reference: Reference,
    requests: VecDeque<HostRequest>,
}

impl Transfers {
    /// Starts or dismisses an export of `open` (the artwork on the canvas and its title).
    ///
    /// # Errors
    /// `Unavailable` while another export or a system picker runs, or without an artwork.
    pub fn export(
        &mut self,
        intent: &ExportIntent,
        open: Option<(&Artwork, &Title)>,
    ) -> Result<(), AppError> {
        match *intent {
            ExportIntent::Start { format, delivery } => {
                self.check(EditorAction::Export)?;
                let Some((artwork, title)) = open else {
                    return Err(AppError::Unavailable {
                        action: EditorAction::Export,
                        availability: Availability::NothingLeft,
                    });
                };
                let request = self.export.start(format, delivery, artwork, title);
                self.requests.push_back(request);
            }
            ExportIntent::Dismiss => self.export.dismiss(),
        }
        Ok(())
    }

    /// Shows, hides or replaces the reference image, or closes its notice.
    ///
    /// # Errors
    /// `Unavailable` when choosing a new image while a system picker or a transfer runs.
    pub fn reference(&mut self, intent: &ReferenceIntent) -> Result<(), AppError> {
        let choosing = match intent {
            ReferenceIntent::Choose => true,
            ReferenceIntent::Show(true) => self.reference.image().is_none(),
            ReferenceIntent::Show(false) | ReferenceIntent::Dismiss => false,
        };
        if choosing {
            self.check(EditorAction::ChooseReference)?;
        }
        if let Some(request) = self.reference.dispatch(intent) {
            self.requests.push_back(request);
        }
        Ok(())
    }

    /// The next thing to ask of the platform, once.
    pub fn request(&mut self) -> Option<HostRequest> {
        self.requests.pop_front()
    }

    /// True while a request waits for the platform's answer.
    #[must_use]
    pub fn waiting(&self) -> bool {
        self.export.waiting() || self.reference.waiting()
    }

    /// The platform's answer to the picker that is open; the work it starts, if any. An answer
    /// nobody waits for (`waiting` is false) is dropped, closing what it holds.
    pub fn answer(&mut self, answer: HostAnswer) -> Option<TransferJob> {
        if self.export.waiting() {
            self.export.answer(answer).map(TransferJob::Export)
        } else if self.reference.waiting() {
            self.reference.answer(answer).map(TransferJob::Open)
        } else {
            None
        }
    }

    /// The export job finished with `result`; a shared file goes to the share sheet next.
    pub fn exported(&mut self, result: Result<(), ExportError>) {
        if let Some(request) = self.export.finished(result) {
            self.requests.push_back(request);
        }
    }

    /// The reference image was read and decoded, or could not be.
    pub fn opened(&mut self, result: Result<ReferenceImage, ImageError>) {
        self.reference.opened(result);
    }

    /// The hand went back to the canvas: a notice that only reported success goes away.
    pub fn interrupt(&mut self) {
        self.export.interrupt();
    }

    #[must_use]
    pub fn projection(&self) -> TransferProjection {
        TransferProjection {
            export: self.export.notice(),
            reference: self.reference.image().cloned(),
            reference_shown: self.reference.shown(),
            reference_notice: self.reference.notice(),
            available: self.availability(),
        }
    }

    fn availability(&self) -> Availability {
        if self.export.busy() || self.reference.busy() {
            Availability::Running
        } else {
            Availability::Available
        }
    }

    fn check(&self, action: EditorAction) -> Result<(), AppError> {
        let availability = self.availability();
        if availability.is_available() {
            Ok(())
        } else {
            Err(AppError::Unavailable {
                action,
                availability,
            })
        }
    }
}

/// Why an export did not reach its file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportError {
    /// The system's save dialog could not be shown, or the file it gave could not be opened.
    Picker(PortError),
    /// The artwork could not be rendered (the GPU refused; the runtime logs why).
    Render,
    /// The pixels could not be encoded.
    Encode,
    /// The chosen file could not be written.
    Write(PortError),
}

/// Why an image could not be used as the reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageError {
    /// The system's picker could not be shown, or the file it gave could not be opened.
    Picker(PortError),
    /// The file could not be read.
    Read(PortError),
    /// Not a PNG or JPEG file.
    Unsupported,
    /// A PNG or JPEG file that breaks its format.
    Damaged,
    /// More pixels or bytes than the app reads (`MAX_IMAGE_PIXELS`, `MAX_IMAGE_BYTES`).
    TooLarge,
}

impl fmt::Display for ExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Picker(error) => write!(f, "the save dialog failed: {error}"),
            Self::Render => f.write_str("the artwork could not be rendered"),
            Self::Encode => f.write_str("the image could not be encoded"),
            Self::Write(error) => write!(f, "the file could not be written: {error}"),
        }
    }
}

impl fmt::Display for ImageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Picker(error) => write!(f, "the picker failed: {error}"),
            Self::Read(error) => write!(f, "the file could not be read: {error}"),
            Self::Unsupported => f.write_str("not a PNG or JPEG file"),
            Self::Damaged => f.write_str("a damaged image file"),
            Self::TooLarge => f.write_str("the image is too large"),
        }
    }
}

impl std::error::Error for ExportError {}
impl std::error::Error for ImageError {}
