//! Export (Procreate's Actions > Share): the artwork at its full size as a PNG or JPEG file,
//! saved where the user chooses or handed to another app. The version exported is the one on
//! the canvas when the user asked; drawing goes on while it is written. Export changes nothing
//! in the artwork, so it is no history step.

use openpaint_core::Artwork;

use crate::project::PortError;
use crate::title::Title;
use crate::transfer::{Delivery, ExportError, HostAnswer, HostRequest, ImageFormat, ImageSink};

/// What the user asks of exporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportIntent {
    /// Export the artwork on the canvas as `format`, to a file or to another app.
    Start {
        format: ImageFormat,
        delivery: Delivery,
    },
    /// Close the notice that the export finished or failed.
    Dismiss,
}

/// What the top bar says about exporting.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ExportNotice {
    #[default]
    None,
    /// The system's save dialog is open, or the file is being rendered and written.
    Working {
        format: ImageFormat,
        delivery: Delivery,
    },
    /// The file `name` was saved.
    Saved { name: String },
    Failed {
        format: ImageFormat,
        delivery: Delivery,
        error: ExportError,
    },
}

/// One export for the transfer worker: render `artwork`, encode it as `format`, write `sink`.
pub struct ExportJob {
    pub artwork: Artwork,
    pub format: ImageFormat,
    pub sink: Box<dyn ImageSink>,
}

impl std::fmt::Debug for ExportJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExportJob")
            .field("format", &self.format)
            .finish_non_exhaustive()
    }
}

/// The export asked for: what, as what, where to and under which name.
#[derive(Debug, Clone)]
struct Order {
    artwork: Artwork,
    name: String,
    format: ImageFormat,
    delivery: Delivery,
}

#[derive(Debug, Default)]
enum State {
    #[default]
    Idle,
    /// Waiting for the platform's file.
    Choosing(Order),
    /// On the transfer worker.
    Running(Order),
    Saved(String),
    Failed(Order, ExportError),
}

#[derive(Debug, Default)]
pub(crate) struct Export {
    state: State,
}

impl Export {
    /// Starts exporting `artwork` (titled `title`) and returns what to ask the platform for.
    /// Call only when not `busy`.
    pub(crate) fn start(
        &mut self,
        format: ImageFormat,
        delivery: Delivery,
        artwork: &Artwork,
        title: &Title,
    ) -> HostRequest {
        let name = file_name(title, format);
        let request = match delivery {
            Delivery::Save => HostRequest::SaveFile {
                name: name.clone(),
                format,
            },
            Delivery::Share => HostRequest::ShareFile {
                name: name.clone(),
                format,
            },
        };
        self.state = State::Choosing(Order {
            artwork: artwork.clone(),
            name,
            format,
            delivery,
        });
        request
    }

    pub(crate) fn waiting(&self) -> bool {
        matches!(self.state, State::Choosing(_))
    }

    /// A system picker is open or a file is being written.
    pub(crate) fn busy(&self) -> bool {
        matches!(self.state, State::Choosing(_) | State::Running(_))
    }

    /// The platform's answer while `waiting`: a file starts the job; cancelling ends the
    /// export without a word; anything else fails it.
    pub(crate) fn answer(&mut self, answer: HostAnswer) -> Option<ExportJob> {
        let State::Choosing(order) = std::mem::take(&mut self.state) else {
            return None;
        };
        match answer {
            HostAnswer::File(sink) => {
                let job = ExportJob {
                    artwork: order.artwork.clone(),
                    format: order.format,
                    sink,
                };
                self.state = State::Running(order);
                Some(job)
            }
            HostAnswer::Cancelled => None,
            HostAnswer::Failed(error) => {
                self.state = State::Failed(order, ExportError::Picker(error));
                None
            }
            HostAnswer::Image(_) => {
                let error = PortError::Device {
                    detail: "an image came back for a file to write".into(),
                };
                self.state = State::Failed(order, ExportError::Picker(error));
                None
            }
        }
    }

    /// The job finished. A saved file is reported; a shared one goes to the share sheet,
    /// which is all the user sees of it.
    pub(crate) fn finished(&mut self, result: Result<(), ExportError>) -> Option<HostRequest> {
        let State::Running(order) = std::mem::take(&mut self.state) else {
            return None;
        };
        match (result, order.delivery) {
            (Ok(()), Delivery::Save) => {
                self.state = State::Saved(order.name);
                None
            }
            (Ok(()), Delivery::Share) => Some(HostRequest::Send {
                name: order.name,
                format: order.format,
            }),
            (Err(error), _) => {
                self.state = State::Failed(order, error);
                None
            }
        }
    }

    pub(crate) fn dismiss(&mut self) {
        if matches!(self.state, State::Saved(_) | State::Failed(..)) {
            self.state = State::Idle;
        }
    }

    /// A success notice goes when the hand goes back to the canvas; a failure stays until OK.
    pub(crate) fn interrupt(&mut self) {
        if matches!(self.state, State::Saved(_)) {
            self.state = State::Idle;
        }
    }

    pub(crate) fn notice(&self) -> ExportNotice {
        match &self.state {
            State::Idle => ExportNotice::None,
            State::Choosing(order) | State::Running(order) => ExportNotice::Working {
                format: order.format,
                delivery: order.delivery,
            },
            State::Saved(name) => ExportNotice::Saved { name: name.clone() },
            State::Failed(order, error) => ExportNotice::Failed {
                format: order.format,
                delivery: order.delivery,
                error: error.clone(),
            },
        }
    }
}

/// The file name for `title` as `format`: the title with the characters file systems refuse
/// replaced by '_', then the extension.
#[must_use]
pub fn file_name(title: &Title, format: ImageFormat) -> String {
    let stem: String = title
        .as_str()
        .chars()
        .map(|c| {
            if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '_'
            } else {
                c
            }
        })
        .collect();
    let stem = stem.trim_end_matches('.');
    let stem = if stem.is_empty() { "Artwork" } else { stem };
    format!("{stem}.{}", format.extension())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_names_keep_the_title_and_drop_what_file_systems_refuse() {
        let title = |text| Title::new(text).expect("title");
        assert_eq!(
            file_name(&Title::untitled(), ImageFormat::Png),
            "Untitled Artwork.png"
        );
        assert_eq!(
            file_name(&title("Harbour: dusk/2"), ImageFormat::Jpeg),
            "Harbour_ dusk_2.jpg"
        );
        assert_eq!(file_name(&title("..."), ImageFormat::Png), "Artwork.png");
        assert_eq!(
            file_name(&title("墨 study"), ImageFormat::Png),
            "墨 study.png"
        );
    }
}
