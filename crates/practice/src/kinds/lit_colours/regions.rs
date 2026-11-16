//! Colour under light's regions (S154j): the scene's exact families from light on form's answer
//! (`light_on_form/answer.rs`), each a region of one colour to read: the object's light family,
//! its shadow family, the lit ground, the cast shadow; at 稳 the shadow family's reflected light
//! apart (light on form's 稳 band: the shadow facing down onto the lit ground). They are a map of
//! cells (`Areas`, `GRID` across the frame), from which their outlines are drawn.
//!
//! A copy is read as a combination's areas are (`areas_read.rs`): each region's colour the
//! median, channel by channel, of the copy's pixels over its inner part, none while most of that
//! shows the paper. The inner part keeps clear of the region's edges as a combination's core does,
//! but by half the region's depth where it is too thin for the core's six cells (a core shadow, a
//! cast shadow seen low), so every region reads.

use crate::colour_areas::areas::{Areas, GRID, depth};
use crate::colour_areas::areas_read::median;
use crate::kinds::light_on_form::answer::{Inner, Mass, Shading};
use crate::kinds::light_on_form::scene::LightTask;
use crate::kinds::lit_colours::scene::ColourTask;
use crate::maths::colour::Rgb8;
use crate::maths::path::Point;

/// A region's inner part lies this many cells inside it at most (a combination's core's), and
/// holds this many cells at least to be read.
const INSIDE: usize = 6;
const LEAST: usize = 6;

/// A region of one colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Region {
    /// The object's light family.
    Light,
    /// The object's shadow family (at 稳 but its reflected light).
    Shadow,
    /// The object's shadow facing down onto the lit ground (稳).
    Reflected,
    /// The lit ground.
    Ground,
    /// The cast shadow on the ground.
    Cast,
}

impl Region {
    pub const ALL: [Self; 5] = [
        Self::Light,
        Self::Shadow,
        Self::Reflected,
        Self::Ground,
        Self::Cast,
    ];

    /// The region a point the light paints as `shading` lies in; the reflected light told apart
    /// where `bounce`.
    pub(crate) fn of(shading: &Shading, bounce: bool) -> Self {
        match (shading.on_form, shading.family) {
            (true, Mass::Light) => Self::Light,
            (true, Mass::Shadow) if bounce && shading.inner == Inner::Reflected => Self::Reflected,
            (true, Mass::Shadow) => Self::Shadow,
            (false, Mass::Light) => Self::Ground,
            (false, Mass::Shadow) => Self::Cast,
        }
    }

    /// Whether it lies on the object.
    #[must_use]
    pub const fn on_form(self) -> bool {
        matches!(self, Self::Light | Self::Shadow | Self::Reflected)
    }
}

/// The scene's regions: those it holds (in `Region::ALL`'s order, their places the map's areas),
/// the map, each cell's region where it lies in that region's inner part, and the model's colour
/// of each region held.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Regions {
    pub(crate) held: Vec<Region>,
    pub(crate) areas: Areas,
    inner: Vec<Option<u8>>,
    pub(crate) colours: Vec<Rgb8>,
}

impl Regions {
    /// The regions of `task`'s scene, the reflected light told apart where `bounce`.
    pub(crate) fn of(task: &LightTask, bounce: bool) -> Self {
        let middle = |k: usize| (k as f64 + 0.5) / GRID as f64;
        let cells: Vec<Region> = (0..GRID * GRID)
            .map(|at| {
                let shading = task.shading([middle(at % GRID), middle(at / GRID)]);
                shading.map_or(Region::Ground, |s| Region::of(&s, bounce))
            })
            .collect();
        let held: Vec<Region> = (Region::ALL.into_iter())
            .filter(|region| cells.contains(region))
            .collect();
        let place = |region: &Region| held.iter().position(|r| r == region).unwrap_or(0);
        let map: Vec<u8> = cells.iter().map(|r| place(r) as u8).collect();
        let deep = depth(&map, [GRID; 2]);
        let deepest: Vec<u8> = (0..held.len())
            .map(|area| {
                (map.iter().zip(&deep))
                    .filter(|(a, _)| usize::from(**a) == area)
                    .map(|(_, d)| *d)
                    .max()
                    .unwrap_or(0)
            })
            .collect();
        let inner = (map.iter().zip(&deep))
            .map(|(area, d)| {
                let reach = usize::from(deepest[usize::from(*area)]).div_ceil(2);
                (usize::from(*d) >= reach.clamp(1, INSIDE)).then_some(*area)
            })
            .collect();
        let colours = vec![[128; 3]; held.len()];
        let areas = Areas::of_map([GRID; 2], map, colours.clone())
            .unwrap_or_else(|| Areas::laid([1, 1], vec![0], vec![[128; 3]], vec![0.0]));
        Self {
            held,
            areas,
            inner,
            colours,
        }
    }

    /// `region`'s place among those held.
    pub(crate) fn place(&self, region: Region) -> Option<usize> {
        self.held.iter().position(|held| *held == region)
    }

    /// The model's colour of `region`; none where it is not held.
    pub(crate) fn colour(&self, region: Region) -> Option<Rgb8> {
        self.place(region).map(|at| self.colours[at])
    }

    /// The middles of `region`'s inner cells (frame units).
    pub(crate) fn inner_middles(&self, region: Region) -> Vec<Point> {
        let Some(place) = self.place(region) else {
            return Vec::new();
        };
        let middle = |k: usize| (k as f64 + 0.5) / GRID as f64;
        (self.inner.iter().enumerate())
            .filter(|(_, held)| **held == u8::try_from(place).ok())
            .map(|(at, _)| [middle(at % GRID), middle(at / GRID)])
            .collect()
    }

    /// Whether `region` is held with an inner part large enough to read.
    pub(crate) fn readable(&self, region: Region) -> bool {
        self.inner_middles(region).len() >= LEAST
    }

    /// Each region held's colour in the copy whose pixels `rgba` (8-bit RGBA, `size` pixels)
    /// cover the frame exactly: its inner part's median, none while most of it is `paper` (a
    /// picture drawn whole, the model's render, has none).
    pub(crate) fn read(
        &self,
        [width, height]: [u32; 2],
        rgba: &[u8],
        paper: Option<Rgb8>,
    ) -> Vec<Option<Rgb8>> {
        let mut samples: Vec<Vec<Rgb8>> = vec![Vec::new(); self.held.len()];
        let (w, h) = (width.max(1) as usize, height.max(1) as usize);
        for (index, pixel) in rgba.as_chunks::<4>().0.iter().take(w * h).enumerate() {
            let (x, y) = (index % w, index / w);
            let cell = (y * GRID / h) * GRID + x * GRID / w;
            if let Some(Some(region)) = self.inner.get(cell) {
                samples[usize::from(*region)].push([pixel[0], pixel[1], pixel[2]]);
            }
        }
        (samples.into_iter())
            .map(|held| {
                let bare = held.iter().filter(|c| Some(**c) == paper).count();
                if held.is_empty() || 2 * bare > held.len() {
                    return None;
                }
                median(held)
            })
            .collect()
    }

    /// `region`'s outline in frame units; none where it is not held.
    pub(crate) fn outline(&self, region: Region) -> Vec<Vec<Point>> {
        self.place(region)
            .map(|area| self.areas.edges_in_frame(area, None, [1.0, 1.0]))
            .unwrap_or_default()
    }
}

impl ColourTask {
    /// Each region the level reads and its colour in the copy whose pixels `rgba` (8-bit RGBA,
    /// `size` pixels) cover the frame exactly, none where it is mostly `paper`.
    #[must_use]
    pub fn read(&self, size: [u32; 2], rgba: &[u8], paper: Rgb8) -> Vec<(Region, Option<Rgb8>)> {
        let read = self.regions.read(size, rgba, Some(paper));
        (self.asked().into_iter())
            .map(|region| {
                let place = self.regions.place(region);
                (region, place.and_then(|at| read.get(at).copied().flatten()))
            })
            .collect()
    }
}
