//! Opening a dense artwork (S49) through the whole frame loop: 300 strokes crossing one place, so
//! the whole canvas's work passes what one frame takes (here with a small dispatch bound, as the
//! tablet's slower GPU gives it at the real one). The canvas fills in over frames in bounded
//! dispatches, the screen keeps answering (a tap opens the layer panel while it fills), and the
//! canvas ends equal to a cold evaluation. A test binary of its own: it sets its device's
//! dispatch bound.

mod gpu;
mod hand;

use std::f32::consts::PI;
use std::sync::Arc;

use hand::{Hand, assert_cold};
use openpaint_app::{
    EditorState, PortError, Project, ProjectId, ProjectStore, ProjectSummary, SaveId, Thumbnail,
    Title, VersionId,
};
use openpaint_core::{
    Artwork, BrushRecipe, CanvasPoint, CanvasSize, Color, StrokeSample, StrokeStyle,
};
use openpaint_render::DISPATCH_WORK;
use openpaint_runtime::Runtime;

const SIZE: [u32; 2] = [1600, 1400];
const PIXELS_PER_POINT: f32 = 2.0;
const CANVAS: [u32; 2] = [1000, 700];

/// A library whose last artwork is `0`, keeping nothing written.
struct Holding(Artwork);

impl ProjectStore for Holding {
    fn open_last(&mut self) -> Result<Option<Project>, PortError> {
        Ok(Some(Project {
            id: ProjectId::new(1),
            title: Title::untitled(),
            artwork: self.0.clone(),
            saved: SaveId::new(1).expect("a save id"),
            thumbnail: None,
        }))
    }

    fn list(&mut self) -> Result<Vec<ProjectSummary>, PortError> {
        Ok(Vec::new())
    }

    fn create(&mut self, _artwork: &Artwork, _title: &Title) -> Result<ProjectId, PortError> {
        Ok(ProjectId::new(2))
    }

    fn load(&mut self, id: ProjectId) -> Result<Project, PortError> {
        Err(PortError::Missing(id))
    }

    fn save(&mut self, _id: ProjectId, _artwork: &Artwork) -> Result<SaveId, PortError> {
        Ok(SaveId::new(2).expect("a save id"))
    }

    fn save_thumbnail(&mut self, _id: ProjectId, _thumbnail: &Thumbnail) -> Result<(), PortError> {
        Ok(())
    }

    fn close(&mut self) -> Result<(), PortError> {
        Ok(())
    }

    fn rename(&mut self, _id: ProjectId, _title: &Title) -> Result<(), PortError> {
        Ok(())
    }

    fn duplicate(&mut self, id: ProjectId) -> Result<ProjectId, PortError> {
        Err(PortError::Missing(id))
    }

    fn delete(&mut self, _id: ProjectId) -> Result<(), PortError> {
        Ok(())
    }

    fn editor(&mut self) -> Result<Option<EditorState>, PortError> {
        Ok(None)
    }

    fn save_editor(&mut self, _state: &EditorState) -> Result<(), PortError> {
        Ok(())
    }

    fn artwork_file(&mut self, id: ProjectId) -> Result<Vec<u8>, PortError> {
        Err(PortError::Missing(id))
    }

    fn import_file(&mut self, _bytes: &[u8]) -> Result<ProjectId, PortError> {
        Err(PortError::NotArtworkFile)
    }

    fn keep_version(&mut self, _id: ProjectId, _kept_at: u64) -> Result<(), PortError> {
        Ok(())
    }

    fn restore_version(
        &mut self,
        id: ProjectId,
        _version: VersionId,
        _title: &Title,
    ) -> Result<ProjectId, PortError> {
        Err(PortError::Missing(id))
    }
}

/// 300 strokes crossing the canvas's middle at every angle (as `pen-many` lays them).
fn dense() -> Artwork {
    let mut artwork = Artwork::new(CanvasSize::new(CANVAS[0], CANVAS[1]).expect("size"));
    let middle = CANVAS.map(|side| side as f32 / 2.0);
    for k in 0..300u32 {
        let angle = k as f32 * 2.399_963_2;
        let shift = [((k * 37) % 61) as f32 - 30.0, ((k * 59) % 41) as f32 - 20.0];
        let samples: Vec<StrokeSample> = (0..20u16)
            .map(|i| {
                let t = f32::from(i) / 19.0;
                let s = 360.0 * (t - 0.5);
                let at = CanvasPoint::new(
                    middle[0] + shift[0] + s * angle.cos(),
                    middle[1] + shift[1] + s * angle.sin(),
                )
                .expect("point");
                let pressure = 0.6 + 0.3 * (PI * t).sin();
                StrokeSample::new(at, pressure, 0.0, 0.0, u32::from(i) * 8).expect("sample")
            })
            .collect();
        let colour = Color::rgb(0.2 + 0.08 * (k % 7) as f32, 0.3, 0.6).expect("colour");
        let style = StrokeStyle::new(Arc::new(BrushRecipe::plain()), 40.0, 0.7, colour);
        let mut stroke = artwork
            .begin_stroke(style.expect("style"), samples[0])
            .expect("begin");
        stroke.extend(&samples[1..]).expect("extend");
        artwork = stroke.finish().artwork;
    }
    artwork
}

#[test]
fn a_dense_artwork_fills_in_over_frames_while_taps_work_and_ends_exact() {
    let core = gpu::core();
    core.set_dispatch_work(500);
    let mut runtime = Runtime::start_on(
        Arc::clone(&core),
        PIXELS_PER_POINT,
        Box::new(Holding(dense())),
    )
    .expect("the runtime starts");
    let mut frames = 0;
    while runtime.artwork().is_none() {
        runtime.frame_offscreen(SIZE).expect("frame");
        frames += 1;
        assert!(frames < 10_000, "the artwork never opened");
    }
    let mut hand = Hand::new(SIZE);
    let mut filling = 0;
    while runtime.moving() {
        if filling == 2 {
            // The screen answers while the canvas fills in.
            hand.tap(&mut runtime, "Layers");
            let names = runtime.controls().names();
            assert!(
                names.iter().any(|name| name == "Add layer"),
                "the layer panel opened while filling: {names:?}"
            );
            assert!(runtime.moving(), "still filling in after the tap");
        }
        runtime.frame_offscreen(SIZE).expect("frame");
        filling += 1;
        assert!(filling < 10_000, "the canvas never filled in");
    }
    core.set_dispatch_work(DISPATCH_WORK);
    eprintln!("the dense artwork filled in over {filling} frames");
    assert!(filling > 4, "{filling} frames");
    assert_cold(&core, &mut runtime, "the canvas filled in");
}
