//! A residual evaluated over frames as background work (S53): on the tablet a smudge's residual
//! over a busy place took most of a second, its material (the layer's earlier ops over every tile
//! its contacts read) the most of it, and a fresh start evaluated them all in its first frame
//! (5 s). Now the screen canvas's background work, before it draws waiting tiles that read a
//! residual the arena lacks, evaluates that residual a share at a time within the frame budget:
//! first its material's regions, then its contacts in order; once whole it goes into the arena as
//! `prepare` would put it, and the tiles that read it are drawn. The same contacts and tiles as
//! a residual evaluated at once, so the same nodes. Only one at a time; anything else using the
//! material texture meanwhile (a stroke's residual, a filter) starts it over.

use std::collections::{HashSet, VecDeque};
use std::ops::Range;

use openpaint_core::Artwork;

use super::budget::keys_of;
use super::needs::Wanted;
use super::{
    Job, Keep, KernelBuffers, Key, Run, Smudges, binding_limit, dabs, held_in, key, material,
    read_tiles, tile_job,
};
use crate::affine::Affine;
use crate::core::Core;
use crate::error::RenderError;
use crate::filter::Filters;
use crate::program::SceneOps;
use crate::raster::SceneBuffers;
use crate::scene::Scene;
use crate::smudge::{MaterialRegion, regions};

/// What a contact costs the smudge kernels on the tablet per node it covers, stamped and
/// committed (S21: radius 300, about 363,000 nodes, 1.5 ms); a tile's reset, per node.
const CONTACT_NS: f64 = 4.1;
const RESET_NS: f64 = 0.2;

/// Residuals a draw needs that the arena lacks: the first in scene order (to evaluate first:
/// later ones read it), and every residual the draw needs.
pub(crate) struct Lacking {
    pub(crate) first: usize,
    pub(crate) needed: HashSet<Key>,
}

/// The residual being evaluated over frames.
pub(super) struct Deriving {
    key: Key,
    /// The scene it was planned for (`SceneCache::serial`).
    scene: u64,
    run: Run,
    /// Its material's parts not made yet.
    material: Making,
    jobs: Vec<Job>,
    next: usize,
}

/// A material made over frames by the raster core (S62): the layer's items before the smudge,
/// where its pixels lie on the canvas, its regions left, each planned as it is drawn (the arena
/// may have moved its earlier residuals since).
struct Making {
    items: Range<usize>,
    base: Affine,
    left: VecDeque<MaterialRegion>,
}

impl Smudges {
    /// The residuals `wanted` needs and the arena lacks, when every residual it needs fits the
    /// device's binding whole; `None` otherwise (and when a filter lacks its result): `prepare`
    /// then evaluates what is needed at once, as before.
    pub(crate) fn lacking(
        &mut self,
        core: &Core,
        (scene, artwork): (&Scene, &Artwork),
        (filters, wanted): (&mut Filters, &Wanted<'_>),
    ) -> Option<Lacking> {
        if scene.smudges.is_empty() {
            return None;
        }
        let keys = keys_of(scene, filters);
        let limit = binding_limit(core).min(self.limit);
        let (needs, whole) = self.needs(scene, artwork, (&keys, wanted), limit);
        let filters_held = keys[scene.smudges.len()..]
            .iter()
            .all(|key| self.arena.base(key).is_some());
        if !whole || !filters_held {
            return None;
        }
        let needed: HashSet<Key> = (keys.iter().zip(&needs))
            .filter(|(_, need)| need.tiles().is_some())
            .map(|(key, _)| *key)
            .collect();
        for key in &needed {
            self.arena.touch(key);
        }
        let first = (keys.iter().zip(&needs))
            .position(|(key, need)| need.tiles().is_some() && !self.arena.whole(key))?;
        Some(Lacking { first, needed })
    }

    /// Goes on evaluating smudge `index`'s residual within the frame budget (starting it when
    /// another, or nothing, was under way); true once it is in the arena, beside the residuals
    /// `needed` (none of which its placing lets go).
    ///
    /// # Errors
    /// As `prepare`.
    pub(crate) fn derive(
        &mut self,
        core: &Core,
        (scene, ops, serial): (&Scene, &SceneOps, u64),
        artwork: &Artwork,
        (index, needed, filters): (usize, &HashSet<Key>, &mut Filters),
    ) -> Result<bool, RenderError> {
        if self.tools.evaluated {
            return Err(RenderError::Invalid(
                "a reference's residuals are made at once",
            ));
        }
        let item = &scene.smudges[index];
        let wanted = key(item.layer, &item.stroke);
        let going = self.deriving.take().filter(|deriving| {
            deriving.key == wanted
                && deriving.scene == serial
                && self.tools.owner == deriving.run.owner
        });
        let mut deriving = match going {
            Some(deriving) => deriving,
            None => self.start(core, (scene, serial), artwork, index)?,
        };
        if !self.materials(core, (scene, ops, &*filters), &mut deriving)?
            || !self.contacts(core, &mut deriving)
        {
            self.deriving = Some(deriving);
            return Ok(false);
        }
        let keys = keys_of(scene, filters);
        let shown: HashSet<Key> = keys.iter().copied().collect();
        let mut needed = needed.clone();
        needed.insert(wanted);
        let keep = Keep {
            shown: &shown,
            needed: &needed,
            room: self.room,
            limit: binding_limit(core).min(self.limit),
        };
        let place = (&deriving.run.working, None);
        self.arena
            .place(&core.gpu, (wanted, &item.stroke), place, &keep)?;
        self.write_bases(core, ops, scene.bases_at, &keys);
        core.evaluate.tally.derived();
        self.tools.scratch = Some(deriving.run);
        Ok(true)
    }

    /// A residual's evaluation begun: its tiles allocated, its material planned and costed, its
    /// tiles' resets and contacts listed.
    fn start(
        &mut self,
        core: &Core,
        (scene, serial): (&Scene, u64),
        artwork: &Artwork,
        index: usize,
    ) -> Result<Deriving, RenderError> {
        let item = &scene.smudges[index];
        let size = [artwork.canvas().width(), artwork.canvas().height()];
        let mut run = self.tools.cold_run(core, size);
        let all = dabs(item.stroke.style(), item.stroke.samples());
        let new = run
            .working
            .allocate(&core.gpu, &read_tiles(&all, size), binding_limit(core))?;
        let rects: Vec<_> = new
            .iter()
            .map(|tile| run.working.tile_rect(*tile))
            .collect();
        let material = Making {
            items: item.before.clone(),
            base: item.to_canvas,
            left: regions(&rects).into(),
        };
        let mut jobs: Vec<Job> = new.iter().map(|tile| tile_job(*tile, size)).collect();
        jobs.extend(
            all.iter()
                .filter_map(|dab| dab.rect(size).map(|rect| Job::Contact(*dab, rect))),
        );
        // The material texture holds this run's tiles from now on.
        self.tools.owner = run.owner;
        Ok(Deriving {
            key: key(item.layer, &item.stroke),
            scene: serial,
            run,
            material,
            jobs,
            next: 0,
        })
    }

    /// Its material's parts the frame budget takes; true once none is left.
    fn materials(
        &mut self,
        core: &Core,
        (scene, ops, filters): (&Scene, &SceneOps, &Filters),
        deriving: &mut Deriving,
    ) -> Result<bool, RenderError> {
        let size = deriving.run.working.size();
        let Making { items, base, left } = &mut deriving.material;
        let held = held_in(&self.arena, &self.tools.empty, (scene, filters));
        let buffers = SceneBuffers::of(ops, held);
        let material = material(&mut self.tools.material, core, (size, false))?;
        while let Some(region) = left.front() {
            let units = ((scene, buffers.bases()), &*items, *base);
            let planned = material.plan_region(core, units, region)?;
            if !core
                .pace()
                .take_raster(planned.0.counts.estimate(planned.0.size))
            {
                break;
            }
            material.draw_region(core, &planned, region, &buffers)?;
            left.pop_front();
        }
        core.background_submitted(core.pace().unmarked());
        Ok(left.is_empty())
    }
}

impl Smudges {
    /// Its tiles' resets and contacts, in order, as many as the frame budget takes; true once
    /// all ran.
    fn contacts(&mut self, core: &Core, deriving: &mut Deriving) -> bool {
        let size = deriving.run.working.size();
        let start = deriving.next;
        {
            let mut pace = core.pace();
            while let Some(job) = deriving.jobs.get(deriving.next) {
                let (Job::Reset(rect) | Job::Contact(_, rect)) = job;
                let nodes = f64::from(rect[2]) * f64::from(rect[3]);
                let each = if matches!(job, Job::Reset(_)) {
                    RESET_NS
                } else {
                    CONTACT_NS
                };
                if !pace.take_serial(nodes * each) {
                    break;
                }
                deriving.next += 1;
            }
        }
        if deriving.next > start {
            let kernels = (self.tools.kernels).get_or_insert_with(|| KernelBuffers::new(&core.gpu));
            let run = &mut deriving.run;
            let buffer = run.working.buffer();
            if run.group.as_ref().is_none_or(|(built, _)| built != buffer)
                && let Some(material) = self.tools.material.as_ref()
            {
                let group = core
                    .smudge
                    .group(&core.gpu, kernels, buffer, material.view());
                run.group = Some((buffer.clone(), group));
            }
            if let Some((_, group)) = &run.group {
                let jobs = &deriving.jobs[start..deriving.next];
                core.smudge.run(&core.gpu, (kernels, group), jobs, size);
            }
            core.background_submitted(core.pace().unmarked());
        }
        deriving.next == deriving.jobs.len()
    }
}
