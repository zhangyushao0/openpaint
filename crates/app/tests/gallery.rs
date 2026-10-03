//! The gallery flow without a store: which library calls each intent asks for and in what
//! order, what the user is told when a call fails, and that nothing else starts while an
//! artwork is being opened.

use openpaint_app::{
    AppError, Availability, CanvasPreset, EditorAction, Gallery, GalleryIntent, GalleryNotice,
    LibraryAnswer, LibraryRequest, PortError, ProjectId, ProjectSummary, Thumbnail, Title,
};

fn summary(id: u64, title: &str) -> ProjectSummary {
    ProjectSummary {
        id: ProjectId::new(id),
        title: Title::new(title).expect("title"),
        thumbnail: None,
    }
}

fn requests(gallery: &mut Gallery) -> Vec<LibraryRequest> {
    std::iter::from_fn(|| gallery.request()).collect()
}

fn failure() -> PortError {
    PortError::Device {
        detail: "test".into(),
    }
}

#[test]
fn a_new_gallery_lists_the_library_first() {
    let mut gallery = Gallery::new();
    assert!(!gallery.projection().listed());
    assert_eq!(requests(&mut gallery), [LibraryRequest::List]);
    let items = vec![summary(2, "Harbour"), summary(1, "Untitled Artwork")];
    gallery.answer(LibraryAnswer::Listed(Ok(items.clone())));
    let projection = gallery.projection();
    assert!(projection.listed());
    assert_eq!(projection.items(), items.as_slice());
}

#[test]
fn changes_are_listed_again_after_they_run() {
    let mut gallery = Gallery::listed(vec![summary(1, "A")]);
    let id = ProjectId::new(1);
    let title = Title::new("Harbour").expect("title");
    for intent in [
        GalleryIntent::Rename {
            id,
            title: title.clone(),
        },
        GalleryIntent::Duplicate(id),
        GalleryIntent::Delete(id),
    ] {
        gallery.dispatch(intent).expect("offered");
    }
    assert_eq!(
        requests(&mut gallery),
        [
            LibraryRequest::Rename(id, title),
            LibraryRequest::List,
            LibraryRequest::Duplicate(id),
            LibraryRequest::List,
            LibraryRequest::Delete(id),
            LibraryRequest::List,
        ]
    );
}

#[test]
fn opening_waits_for_the_library_and_a_failure_is_told_and_dismissed() {
    let mut gallery = Gallery::listed(vec![summary(1, "A")]);
    gallery
        .dispatch(GalleryIntent::Open(ProjectId::new(1)))
        .expect("open");
    assert_eq!(gallery.projection().availability(), Availability::Running);
    let refused = gallery.dispatch(GalleryIntent::Create(CanvasPreset::Screen));
    assert_eq!(
        refused,
        Err(AppError::Unavailable {
            action: EditorAction::Gallery,
            availability: Availability::Running
        })
    );
    assert_eq!(
        requests(&mut gallery),
        [LibraryRequest::Open(ProjectId::new(1))]
    );
    gallery.answer(LibraryAnswer::OpenFailed(failure()));
    let projection = gallery.projection();
    assert_eq!(projection.notice(), GalleryNotice::OpenFailed);
    assert_eq!(projection.availability(), Availability::Available);
    gallery.dispatch(GalleryIntent::Dismiss).expect("dismiss");
    assert_eq!(gallery.projection().notice(), GalleryNotice::None);
    let square = GalleryIntent::Create(CanvasPreset::Square);
    gallery.dispatch(square).expect("create");
    assert_eq!(
        requests(&mut gallery),
        [LibraryRequest::Create(
            Title::untitled(),
            CanvasPreset::Square
        )]
    );
    assert_eq!(CanvasPreset::Square.size([2880, 1716]), [2048, 2048]);
    assert_eq!(CanvasPreset::Screen.size([2880, 1716]), [2880, 1716]);
    assert_eq!(CanvasPreset::A4Portrait.size([1, 1]), [2480, 3508]);
    gallery.answer(LibraryAnswer::CreateFailed(failure()));
    assert_eq!(gallery.projection().notice(), GalleryNotice::CreateFailed);
}

#[test]
fn a_failed_change_or_listing_is_told() {
    let mut gallery = Gallery::listed(Vec::new());
    gallery.answer(LibraryAnswer::Changed(Err(failure())));
    assert_eq!(gallery.projection().notice(), GalleryNotice::ChangeFailed);
    gallery.answer(LibraryAnswer::Listed(Err(failure())));
    assert_eq!(gallery.projection().notice(), GalleryNotice::ListFailed);
}

#[test]
fn thumbnails_check_their_size_and_compare_by_pixels() {
    assert!(Thumbnail::new([2, 1], vec![0; 7]).is_none());
    assert!(Thumbnail::new([0, 1], Vec::new()).is_none());
    let one = Thumbnail::new([2, 1], vec![9; 8]).expect("thumbnail");
    let copy = one.clone();
    assert!(one.same(&copy));
    let equal = Thumbnail::new([2, 1], vec![9; 8]).expect("thumbnail");
    assert!(!one.same(&equal));
    assert_eq!(one, equal);
    assert_ne!(one, Thumbnail::new([1, 2], vec![9; 8]).expect("thumbnail"));
}
