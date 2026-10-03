//! The gallery's calls on the library: listing in change order with titles and thumbnails,
//! rename, duplicate, delete, and which artwork the next start opens.

mod common;

use common::{fresh_dir, records, untitled, write_records};
use openpaint_app::{PortError, ProjectId, ProjectStore, Thumbnail, Title};
use openpaint_core::{Artwork, CanvasPoint, CanvasSize, Color, StrokeSample, StrokeStyle};
use openpaint_store::Library;

fn blank() -> Artwork {
    Artwork::new(CanvasSize::new(64, 48).expect("size"))
}

fn draw(artwork: &Artwork, y: f32) -> Artwork {
    let style = StrokeStyle::new(4.0, 1.0, Color::BLACK).expect("style");
    let sample = |x| {
        let point = CanvasPoint::new(x, y).expect("point");
        StrokeSample::new(point, 0.5, 0.0, 0.0, 0).expect("sample")
    };
    let mut session = artwork.begin_stroke(style, sample(2.0)).expect("begin");
    session.extend(&[sample(40.0)]).expect("extend");
    session.finish().artwork
}

fn titles(library: &mut Library) -> Vec<(u64, String)> {
    let listed = library.list().expect("list");
    listed
        .into_iter()
        .map(|item| (item.id.get(), item.title.as_str().to_owned()))
        .collect()
}

fn title(text: &str) -> Title {
    Title::new(text).expect("title")
}

#[test]
fn the_gallery_lists_the_artwork_changed_last_first() {
    let dir = fresh_dir("list");
    let mut library = Library::at(&dir);
    assert_eq!(library.list(), Ok(Vec::new()));
    let one = library.create(&blank(), &title("One")).expect("create");
    let two = library.create(&blank(), &title("Two")).expect("create");
    assert_eq!(titles(&mut library), [(2, "Two".into()), (1, "One".into())]);
    library.save(one, &draw(&blank(), 10.0)).expect("save");
    assert_eq!(titles(&mut library), [(1, "One".into()), (2, "Two".into())]);
    library.rename(two, &title("Harbour")).expect("rename");
    assert_eq!(
        titles(&mut library),
        [(1, "One".into()), (2, "Harbour".into())],
        "a rename keeps the place"
    );
    assert_eq!(
        library.rename(ProjectId::new(9), &title("x")),
        Err(PortError::Missing(ProjectId::new(9)))
    );
}

#[test]
fn a_thumbnail_is_kept_with_its_project() {
    let dir = fresh_dir("thumbnail");
    let mut library = Library::at(&dir);
    let id = library.create(&blank(), &untitled()).expect("create");
    assert_eq!(library.list().expect("list")[0].thumbnail, None);
    let picture = Thumbnail::new([2, 2], (0..16).collect()).expect("thumbnail");
    library.save_thumbnail(id, &picture).expect("thumbnail");
    drop(library);
    let listed = Library::at(&dir).list().expect("list");
    assert_eq!(listed[0].thumbnail, Some(picture));
}

#[test]
fn a_duplicate_is_a_separate_copy_listed_first() {
    let dir = fresh_dir("duplicate");
    let mut library = Library::at(&dir);
    let drawn = draw(&blank(), 10.0);
    let id = library.create(&drawn, &title("Harbour")).expect("create");
    let picture = Thumbnail::new([1, 1], vec![1, 2, 3, 255]).expect("thumbnail");
    library.save_thumbnail(id, &picture).expect("thumbnail");
    let copy = library.duplicate(id).expect("duplicate");
    assert_ne!(copy, id);
    let listed = library.list().expect("list");
    assert_eq!(listed[0].id, copy);
    assert_eq!(listed[0].title, title("Harbour"));
    assert_eq!(listed[0].thumbnail, Some(picture));
    let copied = library.load(copy).expect("load").artwork;
    assert_eq!(copied.to_parts(), drawn.to_parts());
    library.save(copy, &draw(&copied, 30.0)).expect("save");
    let original = library.load(id).expect("load").artwork;
    assert_eq!(
        original.to_parts(),
        drawn.to_parts(),
        "the original is untouched"
    );
}

#[test]
fn delete_removes_every_record_and_leaves_the_others() {
    let dir = fresh_dir("delete");
    let mut library = Library::at(&dir);
    let keep = library
        .create(&draw(&blank(), 5.0), &title("Keep"))
        .expect("create");
    let gone = library
        .create(&draw(&blank(), 9.0), &title("Gone"))
        .expect("create");
    let picture = Thumbnail::new([1, 1], vec![0; 4]).expect("thumbnail");
    library.save_thumbnail(gone, &picture).expect("thumbnail");
    library.delete(gone).expect("delete");
    library.delete(gone).expect("deleting again succeeds");
    assert_eq!(titles(&mut library), [(keep.get(), "Keep".into())]);
    assert_eq!(library.load(gone), Err(PortError::Missing(gone)));
    assert_eq!(library.open_last(), Ok(None), "the deleted one was open");
    drop(library);
    let prefix = gone.get().to_be_bytes();
    assert!(
        records(&dir)
            .iter()
            .all(|(_, key, _)| !key.starts_with(&prefix) && !key.ends_with(&prefix)),
        "no record of the deleted project is left"
    );
}

#[test]
fn the_next_start_opens_the_artwork_left_open_or_none_after_the_gallery() {
    let dir = fresh_dir("last");
    let mut library = Library::at(&dir);
    let one = library
        .create(&draw(&blank(), 5.0), &untitled())
        .expect("create");
    let two = library.create(&blank(), &untitled()).expect("create");
    library.close().expect("close");
    assert_eq!(library.open_last(), Ok(None));
    let loaded = library.load(one).expect("load");
    drop(library);
    let mut library = Library::at(&dir);
    let last = library.open_last().expect("open").expect("one is open");
    assert_eq!(last, loaded);
    library.load(two).expect("load");
    assert_eq!(library.open_last().expect("open").map(|p| p.id), Some(two));
}

#[test]
fn an_unreadable_thumbnail_is_left_out_and_an_unreadable_entry_refused() {
    let source = fresh_dir("unreadable-source");
    let mut library = Library::at(&source);
    let id = library.create(&blank(), &untitled()).expect("create");
    let picture = Thumbnail::new([1, 1], vec![0; 4]).expect("thumbnail");
    library.save_thumbnail(id, &picture).expect("thumbnail");
    drop(library);
    let stored = records(&source);
    // Both keys are 9 bytes: the thumbnail's ends in 't', the catalogue entry's starts with 'a'.
    let damaged = |part: &str, is: fn(&[u8]) -> bool| {
        let mut changed = stored.clone();
        let record = changed
            .iter_mut()
            .find(|(keyspace, key, _)| keyspace == part && key.len() == 9 && is(key));
        record.expect("the record").2.pop();
        let dir = fresh_dir(&format!("unreadable-{part}"));
        write_records(&dir, &changed);
        Library::at(&dir).list()
    };
    let listed = damaged("projects", |key| key[8] == b't').expect("listed");
    assert_eq!(listed[0].thumbnail, None);
    assert_eq!(
        damaged("library", |key| key[0] == b'a'),
        Err(PortError::Corrupt {
            what: "catalogue entry"
        })
    );
}
