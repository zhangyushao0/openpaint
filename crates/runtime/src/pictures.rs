//! The layer panel's pictures (S38): each layer alone, and each mask's grey, evaluated by the
//! one evaluator at canvas size and reduced on the GPU (`Core::layer_thumbnail`) on a worker of
//! their own, so the frame never waits for them. Derived, never stored. Only while the panel is
//! open, one picture at a time, the topmost out of date first; a picture is out of date when
//! the layer's paint (or mask) in the version shown is not the very one it was made of, so it
//! changes after a stroke lifts or an edit is made, never while the pen moves (the stroke
//! being drawn is not in the version).

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use openpaint_core::{Artwork, Layer, LayerId};
use openpaint_render::{Core, RgbaImage};
use openpaint_ui::Shell;

use crate::canvas::Canvas;
use crate::error::RuntimeError;
use crate::worker::Queue;

/// A row's picture: a layer's paint (`false`) or its mask (`true`).
pub(crate) type PictureKey = (LayerId, bool);

struct Job {
    artwork: Artwork,
    key: PictureKey,
    max: [u32; 2],
}

struct Done {
    key: PictureKey,
    /// The layer as the picture shows it.
    layer: Option<Layer>,
    picture: Option<RgbaImage>,
}

/// Which content each picture handed to the panel was made of, per open canvas.
#[derive(Debug, Default)]
pub(crate) struct Drawn {
    made_of: HashMap<PictureKey, Layer>,
}

impl Drawn {
    /// The topmost picture of `artwork` out of date, or missing; forgets layers that went.
    fn stale(&mut self, artwork: &Artwork) -> Option<PictureKey> {
        self.made_of.retain(|&(id, mask), _| {
            artwork
                .layer(id)
                .is_some_and(|layer| !mask || layer.mask().is_some())
        });
        artwork.layers().iter().rev().find_map(|layer| {
            let id = layer.id();
            if !self.shows((id, false), layer) {
                return Some((id, false));
            }
            (layer.mask().is_some() && !self.shows((id, true), layer)).then_some((id, true))
        })
    }

    /// Whether the picture `key` was made of what `layer` holds now.
    fn shows(&self, key: PictureKey, layer: &Layer) -> bool {
        self.made_of
            .get(&key)
            .is_some_and(|made| same(key, made, layer))
    }
}

/// The paint, or the mask's grey, of `a` and `b` is the very same.
fn same((_, mask): PictureKey, a: &Layer, b: &Layer) -> bool {
    if mask {
        a.mask().zip(b.mask()).is_some_and(|(a, b)| a.same_grey(b))
    } else {
        a.same_paint(b)
    }
}

pub(crate) struct PictureWorker {
    worker: Queue<Job, Done>,
}

impl PictureWorker {
    pub(crate) fn start(core: Arc<Core>) -> Result<Self, RuntimeError> {
        let worker = Queue::spawn("openpaint-pictures", move |job| picture(&core, job))?;
        Ok(Self { worker })
    }

    /// True while a picture is being made or waits to be taken.
    pub(crate) fn busy(&self) -> bool {
        self.worker.busy()
    }

    /// One frame's pictures on `canvas`: those made go to the panel when they still show the
    /// layer as it is; while the panel is open and none is being made, the next one out of date
    /// is asked for.
    pub(crate) fn step(&mut self, canvas: &mut Canvas, shell: &mut Shell) {
        while let Some(done) = self.worker.take() {
            let current = canvas.session().artwork().layer(done.key.0);
            let Some(layer) = done
                .layer
                .filter(|made| current.is_some_and(|layer| same(done.key, made, layer)))
            else {
                continue;
            };
            if let Some(picture) = done.picture {
                shell.set_layer_picture(done.key, picture);
            }
            canvas.pictures.made_of.insert(done.key, layer);
        }
        if !shell.layers_open() || self.worker.busy() {
            return;
        }
        let artwork = canvas.session().artwork().clone();
        if let Some(key) = canvas.pictures.stale(&artwork) {
            let max = shell.layer_picture_pixels();
            self.worker.submit(Job { artwork, key, max });
        }
    }
}

/// The picture of the job's layer (or mask, inverted when the mask is), on the worker.
fn picture(core: &Core, job: Job) -> Done {
    let started = Instant::now();
    let (id, mask) = job.key;
    let layer = job.artwork.layer(id).cloned();
    let made = core
        .layer_thumbnail(&job.artwork, job.key, job.max)
        .inspect_err(|error| log::error!("no picture of {id:?}: {error}"))
        .ok();
    let inverted = layer
        .as_ref()
        .and_then(Layer::mask)
        .is_some_and(|m| m.inverted());
    let picture = made.and_then(|picture| legible(picture, mask.then_some(inverted)));
    log::info!(
        "picture of {id:?} (mask {mask}) in {} ms",
        started.elapsed().as_millis()
    );
    Done {
        key: job.key,
        layer,
        picture,
    }
}

/// The picture as its row shows it. Reduced by box means, a line a few pixels wide covers a
/// tenth of a picture's pixel and would all but vanish, so each pixel's coverage `a` shows as
/// its cube root (0 and 1 stay; its colour stays): on paint the alpha, on a mask the darkness of
/// its grey (inverted when the mask is, as it shows).
fn legible(picture: RgbaImage, mask: Option<bool>) -> Option<RgbaImage> {
    let size = picture.size();
    let mut pixels = picture.into_pixels();
    let stronger = |coverage: f32| coverage.cbrt();
    for pixel in pixels.as_chunks_mut::<4>().0 {
        match mask {
            Some(inverted) => {
                let grey = f32::from(if inverted { 255 - pixel[0] } else { pixel[0] });
                let shown = 255.0 * (1.0 - stronger(1.0 - grey / 255.0));
                let value = shown.round() as u8;
                pixel[..3].fill(value);
            }
            None if pixel[3] > 0 => {
                let alpha = f32::from(pixel[3]) / 255.0;
                let gain = stronger(alpha) / alpha;
                for channel in pixel.iter_mut() {
                    *channel = (f32::from(*channel) * gain).round().min(255.0) as u8;
                }
            }
            None => {}
        }
    }
    RgbaImage::from_rgba(size, pixels)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use openpaint_core::{
        ArtworkCommand, BrushRecipe, CanvasPoint, CanvasSize, Color, LayerCommand, MaskCommand,
        StrokeSample, StrokeStyle,
    };

    use super::*;

    fn sample(x: f32, ms: u32) -> StrokeSample {
        let at = CanvasPoint::new(x, 10.0).expect("point");
        StrokeSample::new(at, 1.0, 0.0, 0.0, ms).expect("sample")
    }

    /// Marks every picture of `artwork` as made, as the worker's answers would.
    fn made(drawn: &mut Drawn, artwork: &Artwork) {
        while let Some(key) = drawn.stale(artwork) {
            let layer = artwork.layer(key.0).expect("layer").clone();
            drawn.made_of.insert(key, layer);
        }
    }

    #[test]
    fn a_picture_is_out_of_date_after_a_stroke_lifts_never_while_it_is_drawn() {
        let blank = Artwork::new(CanvasSize::new(64, 48).expect("size"));
        let two = blank
            .apply(ArtworkCommand::Layer(LayerCommand::Create))
            .expect("add")
            .artwork;
        let mut drawn = Drawn::default();
        let top = two.active();
        assert_eq!(drawn.stale(&two), Some((top, false)), "the top row first");
        made(&mut drawn, &two);
        assert_eq!(drawn.stale(&two), None);
        let style = StrokeStyle::new(Arc::new(BrushRecipe::plain()), 4.0, 1.0, Color::BLACK)
            .expect("style");
        let mut stroke = two.begin_stroke(style, sample(4.0, 0)).expect("begin");
        stroke.extend(&[sample(30.0, 20)]).expect("extend");
        assert_eq!(
            drawn.stale(&two),
            None,
            "the version shown keeps its pictures"
        );
        let lifted = stroke.finish().artwork;
        assert_eq!(
            drawn.stale(&lifted),
            Some((top, false)),
            "the stroke lifted"
        );
        made(&mut drawn, &lifted);
        let masked = lifted
            .apply(ArtworkCommand::Mask(MaskCommand::Add(top)))
            .expect("mask")
            .artwork;
        assert_eq!(
            drawn.stale(&masked),
            Some((top, true)),
            "only the mask is new"
        );
        made(&mut drawn, &masked);
        let deleted = masked
            .apply(ArtworkCommand::Layer(LayerCommand::Delete(top)))
            .expect("delete")
            .artwork;
        assert_eq!(drawn.stale(&deleted), None);
        assert_eq!(
            drawn.made_of.len(),
            1,
            "the deleted layer's pictures are forgotten"
        );
        let hidden = LayerCommand::SetVisible {
            layer: deleted.active(),
            visible: false,
        };
        let props = deleted
            .apply(ArtworkCommand::Layer(hidden))
            .expect("hide")
            .artwork;
        assert_eq!(drawn.stale(&props), None, "a property is not the paint");
    }

    #[test]
    fn faint_coverage_shows_stronger_and_an_inverted_mask_the_opposite_grey() {
        let image = |pixels: Vec<u8>| RgbaImage::from_rgba([2, 1], pixels).expect("image");
        // A tenth of black over transparent, and nothing.
        let paint = legible(image(vec![0, 0, 0, 26, 0, 0, 0, 0]), None).expect("paint");
        assert_eq!(paint.pixels(), [0, 0, 0, 119, 0, 0, 0, 0]);
        // Premultiplied red at a quarter: its colour stays red, stronger.
        let red = legible(image(vec![64, 0, 0, 64, 255, 0, 0, 255]), None).expect("red");
        assert_eq!(red.pixels(), [161, 0, 0, 161, 255, 0, 0, 255]);
        // A mask's grey 230 (a little black) shows darker; inverted, 25 shows as that too.
        let grey = legible(
            image(vec![230, 230, 230, 255, 255, 255, 255, 255]),
            Some(false),
        );
        assert_eq!(
            grey.expect("grey").pixels(),
            [137, 137, 137, 255, 255, 255, 255, 255]
        );
        let flipped = legible(image(vec![25, 25, 25, 255, 0, 0, 0, 255]), Some(true));
        assert_eq!(
            flipped.expect("flipped").pixels()[..4],
            [137, 137, 137, 255]
        );
    }
}
