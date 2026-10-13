//! A residual evaluated over frames as background work (S53): on the tablet a smudge's residual
//! over a busy place took most of a second, its material (the layer's earlier ops over every tile
//! its contacts read) the most of it, and a fresh start evaluated them all in its first frame
//! (5 s). Now the screen canvas's background work, before it takes waiting tiles that read a
//! residual the arena lacks, evaluates that residual a share at a time within the frame budget:
//! first its material's tiles, then its contacts in order; once whole it goes into the arena as
//! `prepare` would put it, and the tiles that read it are taken. The same contacts and tiles as
//! a residual evaluated at once, so the same nodes. Only one at a time; anything else using the
//! material texture meanwhile (a stroke's residual, a filter) starts it over.

use std::collections::{HashSet, VecDeque};

use openpaint_core::Artwork;

use super::budget::keys_of;
use super::needs::{Wanted, around};
use super::{
    Job, Keep, KernelBuffers, Key, Run, Smudges, binding_limit, dabs, key, material, read_tiles,
    tile_job,
};
use crate::core::Core;
use crate::cost::tile_cost;
use crate::error::RenderError;
use crate::filter::Filters;
use crate::geom::PixelRect;
use crate::plan::Plan;
use crate::program::SceneOps;
use crate::protocol::TileRecord;
use crate::scene::Scene;

/// What a contact costs the smudge kernels on the tablet per node it covers, stamped and
/// committed (S21: radius 300, about 363,000 nodes, 1.5 ms); a tile's reset, per node.
const CONTACT_NS: f64 = 4.1;
const RESET_NS: f64 = 0.2;

/// Residuals an evaluation needs that the arena lacks: the first in scene order (to evaluate
/// first: later ones read it), every residual the evaluation needs, and the canvas pixels around
/// which the lacking ones are read (tiles meeting them need them).
pub(crate) struct Lacking {
    pub(crate) first: usize,
    pub(crate) needed: HashSet<Key>,
    pub(crate) reach: Vec<PixelRect>,
}

/// The residual being evaluated over frames.
pub(super) struct Deriving {
    key: Key,
    /// The scene it was planned for (`SceneCache::serial`).
    scene: u64,
    run: Run,
    plan: Plan,
    /// Its material's tiles not evaluated yet, with their estimates.
    left: VecDeque<(TileRecord, f64)>,
    jobs: Vec<Job>,
    next: usize,
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
        let lacking: Vec<usize> = (keys.iter().zip(&needs).enumerate())
            .filter(|(_, (key, need))| need.tiles().is_some() && !self.arena.whole(key))
            .map(|(index, _)| index)
            .collect();
        let reach = (lacking.iter())
            .map(|index| around(&scene.smudges[*index].canvas))
            .collect();
        Some(Lacking {
            first: *lacking.first()?,
            needed,
            reach,
        })
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
        (scene, ops, serial, costs): (&Scene, &SceneOps, u64, &[f32]),
        artwork: &Artwork,
        (index, needed, filters): (usize, &HashSet<Key>, &mut Filters),
    ) -> Result<bool, RenderError> {
        let item = &scene.smudges[index];
        let wanted = key(item.layer, &item.stroke);
        let going = self.deriving.take().filter(|deriving| {
            deriving.key == wanted
                && deriving.scene == serial
                && self.tools.owner == deriving.run.owner
        });
        let mut deriving = match going {
            Some(deriving) => deriving,
            None => self.start(core, (scene, serial, costs), artwork, index)?,
        };
        if !self.materials(core, (scene, ops), &mut deriving)?
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
        (scene, serial, costs): (&Scene, u64, &[f32]),
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
        let material = material(&mut self.tools.material, core, size)?;
        let plan = material.plan(scene, (item.before.clone(), item.to_canvas), &rects);
        let left = (plan.tiles.iter())
            .map(|tile| (*tile, tile_cost(&plan, tile, costs)))
            .collect();
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
            plan,
            left,
            jobs,
            next: 0,
        })
    }

    /// Its material's tiles the frame budget takes; true once none is left.
    fn materials(
        &mut self,
        core: &Core,
        (scene, ops): (&Scene, &SceneOps),
        deriving: &mut Deriving,
    ) -> Result<bool, RenderError> {
        let mut pace = core.pace();
        let mut taken = Vec::new();
        while let Some((tile, cost)) = deriving.left.front().copied() {
            if !pace.take(cost) {
                break;
            }
            deriving.left.pop_front();
            taken.push(tile);
        }
        let estimate = pace.close();
        if !taken.is_empty() {
            let size = deriving.run.working.size();
            let item_canvas = scene
                .smudges
                .iter()
                .find(|item| key(item.layer, &item.stroke) == deriving.key);
            let to_canvas =
                item_canvas.map_or(crate::affine::Affine::IDENTITY, |item| item.to_canvas);
            let residuals = [self.arena.buffer(), &self.tools.empty];
            let timing = pace.time(estimate);
            let clocked = timing.and_then(|timing| Some((pace.clock()?, timing)));
            let material = material(&mut self.tools.material, core, size)?;
            let plan = (&deriving.plan, taken.as_slice());
            material.evaluate_tiles(core, ops, plan, to_canvas, (residuals, clocked))?;
            if let Some(timing) = timing {
                pace.finish(&core.gpu, timing);
            }
        }
        Ok(deriving.left.is_empty())
    }

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
        }
        deriving.next == deriving.jobs.len()
    }
}
