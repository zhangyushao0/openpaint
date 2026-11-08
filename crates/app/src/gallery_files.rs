//! The gallery's files (DECISIONS D62): "Save as OpenPaint file…" on a card asks the system's
//! save dialog for a place and the library writes the artwork there; "Open OpenPaint file…"
//! under "+" asks the system's picker for a file and the library adds it as a new artwork. A
//! mode's own pickers take the same turn (`Gallery::choose`), their answers handed back to it by
//! tag. One system dialog at a time; the library's worker does the reading and writing.

use std::collections::VecDeque;
use std::fmt;

use crate::export::artwork_file_name;
use crate::gallery::{FileProblem, GalleryNotice};
use crate::project::{PortError, ProjectId};
use crate::title::Title;
use crate::transfer::{HostAnswer, HostRequest, ImageSink, ImageSource};

/// Most bytes an OpenPaint file may have to be opened (the file is read whole).
pub const MAX_ARTWORK_FILE_BYTES: u64 = 512 << 20;

/// Work for the library's worker once the system answered.
pub enum FileJob {
    /// Write project `id` as an OpenPaint file into `sink`.
    Save {
        id: ProjectId,
        sink: Box<dyn ImageSink>,
    },
    /// Read `source` and add it as a new artwork.
    Open { source: Box<dyn ImageSource> },
    /// A mode's picker asked with `tag` answered: the file chosen, or none when the system could
    /// not show the picker or open what was chosen. The mode calls `Gallery::file_done` once its
    /// work with the file is over.
    Mode {
        tag: u32,
        source: Option<Box<dyn ImageSource>>,
    },
}

impl fmt::Debug for FileJob {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Save { id, .. } => write!(f, "Save({})", id.get()),
            Self::Open { .. } => f.write_str("Open"),
            Self::Mode { tag, source } => write!(f, "Mode({tag}, {})", source.is_some()),
        }
    }
}

/// What a system dialog was opened for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Choice {
    Save(ProjectId),
    Open,
    /// A mode's, by its tag.
    Mode(u32),
}

#[derive(Debug, Default)]
enum State {
    #[default]
    Idle,
    /// A system dialog is open.
    Choosing(Choice),
    /// The library's worker reads or writes the file.
    Working,
}

#[derive(Debug, Default)]
pub(crate) struct Files {
    state: State,
    requests: VecDeque<HostRequest>,
}

impl Files {
    /// A dialog is open or a file is being read or written.
    pub(crate) fn busy(&self) -> bool {
        !matches!(self.state, State::Idle)
    }

    /// Asks for the save dialog for project `id`, titled `title`.
    pub(crate) fn save(&mut self, id: ProjectId, title: &Title) {
        let name = artwork_file_name(title);
        self.ask(HostRequest::SaveArtwork { name }, Choice::Save(id));
    }

    /// Asks for the picker of a file to open.
    pub(crate) fn open(&mut self) {
        self.ask(HostRequest::OpenArtwork, Choice::Open);
    }

    /// Asks for a mode's picker `request`, answered by `tag`.
    pub(crate) fn choose(&mut self, request: HostRequest, tag: u32) {
        self.ask(request, Choice::Mode(tag));
    }

    fn ask(&mut self, request: HostRequest, choice: Choice) {
        self.requests.push_back(request);
        self.state = State::Choosing(choice);
    }

    pub(crate) fn request(&mut self) -> Option<HostRequest> {
        self.requests.pop_front()
    }

    pub(crate) fn waiting(&self) -> bool {
        matches!(self.state, State::Choosing(_))
    }

    /// The system's answer while `waiting`: the job it starts, or what the user is told.
    /// Cancelling ends without a word.
    pub(crate) fn answer(&mut self, answer: HostAnswer) -> Result<Option<FileJob>, GalleryNotice> {
        let State::Choosing(choice) = std::mem::take(&mut self.state) else {
            return Ok(None);
        };
        let job = match (choice, answer) {
            (_, HostAnswer::Cancelled) => return Ok(None),
            (Choice::Save(id), HostAnswer::File(sink)) => FileJob::Save { id, sink },
            (Choice::Open, HostAnswer::Image(source)) => FileJob::Open { source },
            (Choice::Mode(tag), HostAnswer::Image(source)) => FileJob::Mode {
                tag,
                source: Some(source),
            },
            (Choice::Mode(tag), _) => return Ok(Some(FileJob::Mode { tag, source: None })),
            (Choice::Save(_), _) => return Err(GalleryNotice::FileSaveFailed),
            (Choice::Open, _) => return Err(GalleryNotice::FileRefused(FileProblem::Unreadable)),
        };
        self.state = State::Working;
        Ok(Some(job))
    }

    /// The library's worker finished the job.
    pub(crate) fn finished(&mut self) {
        self.state = State::Idle;
    }
}

/// What the user is told about a file that could not be opened.
#[must_use]
pub const fn problem(error: &PortError) -> FileProblem {
    match error {
        PortError::NotArtworkFile => FileProblem::NotOpenPaint,
        PortError::OtherFormat { .. } => FileProblem::OtherFormat,
        PortError::Corrupt { .. } | PortError::Invalid(_) => FileProblem::Damaged,
        PortError::Device { .. } | PortError::Missing(_) | PortError::Stopped => {
            FileProblem::Unreadable
        }
    }
}
