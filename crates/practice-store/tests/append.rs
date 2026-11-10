//! A lesson appended to a stored sheet (S121b): the sheet's record and the new lesson's givens in
//! one write; the sheet of two lessons reads back whole, its givens resolving every exercise;
//! a write refused leaves the sheet and its givens as they were.

#[path = "../../store/tests/common/test_dir.rs"]
mod test_dir;

use openpaint_app::{
    EditorState, PortError, Project, ProjectId, ProjectStore, ProjectSummary, Record, SaveId,
    Space, Thumbnail, Title, VersionId,
};
use openpaint_core::{Artwork, CanvasSize};
use openpaint_practice::{Exercise, Given, Givens, Kind, Layout, LineTier, Plan, Sheet, stage_one};
use openpaint_practice_store::{GIVENS, append, printed, sheets, start};
use openpaint_store::Library;
use test_dir::TestDir;

/// 2026-10-05 12:00 UTC.
const NOON: u64 = 1_791_201_600;

/// A drill of four single lines started in `library` (finished), as stored.
fn started(library: &mut dyn ProjectStore) -> Sheet {
    let exercises = (0..4)
        .map(|seed| Exercise::generated(Kind::Line(LineTier::Arc), seed))
        .collect();
    let mut drill = Sheet::drill(
        1,
        NOON,
        Plan {
            exercises,
            warm_up: 0,
        },
    );
    let [width, height] = Layout::of_sheet(&drill).size;
    let artwork = Artwork::new(CanvasSize::new(width, height).expect("size"));
    let givens = Givens::of_lesson(&drill.lessons[0], &[]);
    let title = Title::new("Drill 1").expect("title");
    let id = start(library, (&artwork, &title), &drill, &givens).expect("started");
    drill.canvas = id.get();
    drill.lessons[0].finished = Some(NOON + 300);
    drill
}

/// `sheet` with a first session of stage 1 appended, and what it prints.
fn appended(mut sheet: Sheet) -> (Sheet, Givens) {
    let session = Sheet::session(0, 1, NOON + 600, stage_one(5, &[]));
    let lesson = session.lessons[0].clone();
    let givens = Givens::of_lesson(&lesson, &[]);
    let givens = sheet.append(lesson, givens);
    (sheet, givens)
}

#[test]
fn a_sheet_of_two_lessons_round_trips_its_givens_resolving_every_exercise() {
    let dir = TestDir::new("practice-append");
    let mut library = Library::at(&dir);
    let (sheet, givens) = appended(started(&mut library));
    append(&mut library, &sheet, &givens).expect("appended");
    drop(library);
    let mut library = Library::at(&dir);
    let read = sheets(&mut library).expect("practice");
    assert_eq!(read, std::slice::from_ref(&sheet));
    assert_eq!(library.records(GIVENS, &[]).map(|g| g.len()), Ok(2));
    let printed = printed(&mut library, &read[0]).expect("every exercise's");
    assert_eq!(printed, Given::of_sheet(&sheet, &[]));
    let layout = Layout::of_sheet(&read[0]);
    assert_eq!(
        layout.pairs[4],
        Layout::pair_at(6, None),
        "the second lesson on a new row"
    );
}

#[test]
fn an_append_refused_leaves_the_sheet_and_its_givens_as_they_were() {
    let dir = TestDir::new("practice-append-refused");
    let mut store = RefusingGivens(Library::at(&dir));
    let sheet = started(&mut store);
    let before = sheets(&mut store).expect("practice");
    let (longer, givens) = appended(sheet);
    assert!(append(&mut store, &longer, &givens).is_err());
    assert_eq!(
        sheets(&mut store).expect("practice"),
        before,
        "its record as it was"
    );
    assert_eq!(store.records(GIVENS, &[]).map(|g| g.len()), Ok(1));
    assert!(
        printed(&mut store, &before[0]).is_ok(),
        "its one lesson's givens"
    );
}

/// The library, refusing any write of records that holds a lesson's givens, as a full disk would
/// refuse the batch: what a second write of the sheet's record alone would let through.
struct RefusingGivens(Library);

impl ProjectStore for RefusingGivens {
    fn put_records(&mut self, records: &[(Space, Record)]) -> Result<(), PortError> {
        if records.iter().any(|(space, _)| *space == GIVENS) {
            return Err(PortError::Device {
                detail: "refused by the test".into(),
            });
        }
        self.0.put_records(records)
    }

    fn records(&mut self, space: Space, under: &[u8]) -> Result<Vec<Record>, PortError> {
        self.0.records(space, under)
    }

    fn create_owned(
        &mut self,
        project: (&Artwork, &Title),
        owned: &dyn Fn(ProjectId) -> Vec<(Space, Record)>,
    ) -> Result<ProjectId, PortError> {
        self.0.create_owned(project, owned)
    }

    fn remove_owned(
        &mut self,
        keys: &[(Space, Vec<u8>)],
        project: Option<ProjectId>,
    ) -> Result<(), PortError> {
        self.0.remove_owned(keys, project)
    }

    fn open_last(&mut self) -> Result<Option<Project>, PortError> {
        self.0.open_last()
    }

    fn list(&mut self) -> Result<Vec<ProjectSummary>, PortError> {
        self.0.list()
    }

    fn create(&mut self, artwork: &Artwork, title: &Title) -> Result<ProjectId, PortError> {
        self.0.create(artwork, title)
    }

    fn load(&mut self, id: ProjectId) -> Result<Project, PortError> {
        self.0.load(id)
    }

    fn save(&mut self, id: ProjectId, artwork: &Artwork) -> Result<SaveId, PortError> {
        self.0.save(id, artwork)
    }

    fn save_thumbnail(&mut self, id: ProjectId, thumbnail: &Thumbnail) -> Result<(), PortError> {
        self.0.save_thumbnail(id, thumbnail)
    }

    fn close(&mut self) -> Result<(), PortError> {
        self.0.close()
    }

    fn rename(&mut self, id: ProjectId, title: &Title) -> Result<(), PortError> {
        self.0.rename(id, title)
    }

    fn duplicate(&mut self, id: ProjectId) -> Result<ProjectId, PortError> {
        self.0.duplicate(id)
    }

    fn delete(&mut self, id: ProjectId) -> Result<(), PortError> {
        self.0.delete(id)
    }

    fn editor(&mut self) -> Result<Option<EditorState>, PortError> {
        self.0.editor()
    }

    fn save_editor(&mut self, state: &EditorState) -> Result<(), PortError> {
        self.0.save_editor(state)
    }

    fn artwork_file(&mut self, id: ProjectId) -> Result<Vec<u8>, PortError> {
        self.0.artwork_file(id)
    }

    fn import_file(&mut self, bytes: &[u8]) -> Result<ProjectId, PortError> {
        self.0.import_file(bytes)
    }

    fn keep_version(&mut self, id: ProjectId, kept_at: u64) -> Result<(), PortError> {
        self.0.keep_version(id, kept_at)
    }

    fn restore_version(
        &mut self,
        id: ProjectId,
        version: VersionId,
        title: &Title,
    ) -> Result<ProjectId, PortError> {
        self.0.restore_version(id, version, title)
    }

    fn artwork(&mut self, id: ProjectId) -> Result<Artwork, PortError> {
        self.0.artwork(id)
    }
}
