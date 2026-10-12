//! The gallery's OpenPaint files and kept versions without a store: what the system is asked
//! for, the jobs its answers start, one dialog at a time, and what the user is told.

use openpaint_app::{
    FileJob, FileProblem, Gallery, GalleryIntent, GalleryNotice, HostAnswer, HostRequest,
    ImageSink, ImageSource, LibraryAnswer, LibraryRequest, PortError, ProjectId, ProjectSummary,
    Title, VersionId,
};

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

fn harbour() -> Gallery {
    Gallery::listed(vec![ProjectSummary {
        id: ProjectId::new(4),
        title: Title::new("Harbour: dusk").expect("title"),
        thumbnail: None,
        versions: Vec::new(),
        needs_repair: false,
    }])
}

#[test]
fn saving_asks_for_a_place_named_after_the_title_and_writes_there() {
    let mut gallery = harbour();
    let id = ProjectId::new(4);
    gallery.dispatch(GalleryIntent::SaveFile(id)).expect("save");
    let name = "Harbour_ dusk.openpaint".to_owned();
    assert_eq!(
        gallery.host_request(),
        Some(HostRequest::SaveArtwork { name })
    );
    assert_eq!(gallery.host_request(), None);
    assert!(gallery.waiting());
    assert!(
        gallery.dispatch(GalleryIntent::OpenFile).is_err(),
        "one dialog at a time"
    );
    let job = gallery.host_answer(HostAnswer::File(Box::new(Sink)));
    assert!(matches!(job, Some(FileJob::Save { id: saved, .. }) if saved == id));
    assert!(!gallery.waiting());
    assert!(
        gallery.dispatch(GalleryIntent::SaveFile(id)).is_err(),
        "while writing"
    );
    assert_eq!(gallery.request(), None, "no library call of its own");
    gallery.answer(LibraryAnswer::FileSaved(Ok(())));
    assert_eq!(gallery.projection().notice(), GalleryNotice::FileSaved);
    gallery
        .dispatch(GalleryIntent::SaveFile(id))
        .expect("again");
    gallery.host_request();
    assert!(gallery.host_answer(HostAnswer::Cancelled).is_none());
    assert!(!gallery.waiting(), "cancelled");
    gallery
        .dispatch(GalleryIntent::SaveFile(id))
        .expect("once more");
    gallery.host_request();
    gallery.host_answer(HostAnswer::File(Box::new(Sink)));
    gallery.answer(LibraryAnswer::FileSaved(Err(PortError::Stopped)));
    assert_eq!(gallery.projection().notice(), GalleryNotice::FileSaveFailed);
}

#[test]
fn opening_a_file_adds_an_artwork_or_says_why_not() {
    let mut gallery = harbour();
    assert!(
        gallery.host_answer(HostAnswer::Cancelled).is_none(),
        "nobody waits"
    );
    let refusals = [
        (PortError::NotArtworkFile, FileProblem::NotOpenPaint),
        (PortError::Corrupt { what: "file" }, FileProblem::Damaged),
        (
            PortError::NewerSchema {
                found: 15,
                newest: 14,
            },
            FileProblem::Newer,
        ),
        (
            PortError::Device {
                detail: "gone".into(),
            },
            FileProblem::Unreadable,
        ),
    ];
    for (error, problem) in refusals {
        gallery.dispatch(GalleryIntent::OpenFile).expect("open");
        assert_eq!(gallery.host_request(), Some(HostRequest::OpenArtwork));
        let job = gallery.host_answer(HostAnswer::Image(Box::new(Source)));
        assert!(matches!(job, Some(FileJob::Open { .. })));
        gallery.answer(LibraryAnswer::FileOpened(Err(error)));
        assert_eq!(
            gallery.projection().notice(),
            GalleryNotice::FileRefused(problem)
        );
        assert_eq!(gallery.request(), None);
    }
    gallery.dispatch(GalleryIntent::OpenFile).expect("open");
    gallery.host_request();
    gallery.host_answer(HostAnswer::Image(Box::new(Source)));
    gallery.answer(LibraryAnswer::FileOpened(Ok(())));
    assert_eq!(
        gallery.request(),
        Some(LibraryRequest::List),
        "the new card shows"
    );
    gallery.dispatch(GalleryIntent::OpenFile).expect("open");
    gallery.host_request();
    let failed = PortError::Device {
        detail: "no picker".into(),
    };
    assert!(gallery.host_answer(HostAnswer::Failed(failed)).is_none());
    let unreadable = GalleryNotice::FileRefused(FileProblem::Unreadable);
    assert_eq!(gallery.projection().notice(), unreadable);
}

#[test]
fn a_version_is_restored_as_a_copy_and_listed() {
    let mut gallery = harbour();
    let (id, version) = (ProjectId::new(4), VersionId::new(3));
    let title = Title::new("Harbour: dusk (5 Oct 2026 12:00)").expect("title");
    let restore = GalleryIntent::Restore {
        id,
        version,
        title: title.clone(),
    };
    gallery.dispatch(restore).expect("restore");
    let requests: Vec<_> = std::iter::from_fn(|| gallery.request()).collect();
    assert_eq!(
        requests,
        [
            LibraryRequest::Restore(id, version, title),
            LibraryRequest::List
        ]
    );
    gallery.answer(LibraryAnswer::Restored(Ok(())));
    assert_eq!(gallery.projection().notice(), GalleryNotice::None);
    gallery.answer(LibraryAnswer::Restored(Err(PortError::Missing(id))));
    assert_eq!(gallery.projection().notice(), GalleryNotice::RestoreFailed);
}
