//! How one evaluation reads a canvas target's kept scene: made ready first (the preview's
//! snapshot taken, the plan binned or kept, S35), then read without changing the cache, so the
//! residuals near the tiles it evaluates can be made present in between (S51).

use std::ops::Range;

use super::{PlanKey, Reading, SceneCache};
use crate::core::Core;
use crate::error::RenderError;
use crate::geom::PixelRect;
use crate::output::Output;
use crate::plan::Plan;
use crate::preview::PreviewSnapshot;
use crate::program::Bindings;
use crate::protocol::{Op, TileRecord};
use crate::scene::Fresh;

/// What `SceneCache::ready` made ready for `reading`: the fresh ops, and whether a preview
/// resumes from its snapshot.
pub(super) struct Readied {
    fresh: Vec<Op>,
    resume: bool,
}

impl SceneCache {
    /// The items a previewed adjustment's snapshot is about to evaluate (S35): when `previewing`
    /// and it is not taken yet.
    pub(super) fn preview_items(&self, previewing: bool) -> Option<Range<usize>> {
        let place = self.previewed.filter(|_| previewing)?;
        let taken = self.snapshot.as_ref().is_some_and(|s| s.place == place);
        (!taken).then(|| self.scene.prefix_of(place)).flatten()
    }

    /// Makes the evaluation of `regions` at `output` with `fresh` ready to read this scene: the
    /// in-progress stroke's evaluations bin their own plan each time; others keep it, and a
    /// previewed adjustment's layer resumes from its snapshot (taken first through the target's
    /// `bindings`, with the arena and in-progress residual `residuals`).
    pub(super) fn ready(
        &mut self,
        core: &Core,
        (bindings, residuals): (&mut Bindings, [&wgpu::Buffer; 2]),
        fresh: Fresh,
        (size, output, regions): ([u32; 2], Output, &[PixelRect]),
    ) -> Result<Readied, RenderError> {
        let previewing = fresh.ops.is_empty() && output == Output::CANVAS;
        if let Some(place) = self.previewed.filter(|_| previewing)
            && self.snapshot.as_ref().is_none_or(|s| s.place != place)
        {
            let scene = (&*self.scene, &self.ops);
            self.snapshot = PreviewSnapshot::take(core, scene, (bindings, residuals), place)?;
        }
        let resume = self
            .snapshot
            .as_ref()
            .filter(|s| previewing && self.previewed == Some(s.place))
            .map(|s| s.prefix.clone());
        self.plan_for(core, &fresh, (size, output, regions), resume.clone());
        let plan = self
            .plan
            .as_ref()
            .map(|(_, plan)| plan)
            .ok_or(RenderError::Invalid("no plan was built"))?;
        let stale = |(spread, serial, _): &(u32, u64, Vec<TileRecord>)| {
            *spread != self.spread || *serial != plan.serial
        };
        if resume.is_some() && self.near.as_ref().is_none_or(stale) {
            self.near = Some((self.spread, plan.serial, plan.near_resumed(self.spread)));
        }
        Ok(Readied {
            fresh: if resume.is_some() {
                vec![Op::resume()]
            } else {
                fresh.ops
            },
            resume: resume.is_some(),
        })
    }

    /// How the evaluation `ready` made ready reads this scene.
    pub(super) fn reading(&self, readied: Readied) -> Result<Reading<'_>, RenderError> {
        let resume = readied.resume.then_some(());
        let plan = self.plan.as_ref().map(|(_, plan)| plan);
        Ok(Reading {
            ops: &self.ops,
            plan: plan.ok_or(RenderError::Invalid("no plan was built"))?,
            fresh: readied.fresh,
            snapshot: resume
                .and(self.snapshot.as_ref())
                .map(|snapshot| snapshot.reading(false)),
            only: resume
                .and(self.near.as_ref())
                .map(|(_, _, tiles)| tiles.as_slice()),
        })
    }

    /// Bins the plan for `regions` at `output`, unless the plan kept was binned for them and
    /// nothing is fresh: resuming a previewed layer (`resume`), or every item.
    fn plan_for(
        &mut self,
        core: &Core,
        fresh: &Fresh,
        (size, output, regions): ([u32; 2], Output, &[PixelRect]),
        resume: Option<Range<usize>>,
    ) {
        let key = PlanKey {
            regions: regions.to_vec(),
            output,
            resume: resume.clone(),
        };
        if fresh.ops.is_empty() && self.plan.as_ref().is_some_and(|(kept, _)| *kept == key) {
            return;
        }
        let target = (size, output);
        let plan = match resume {
            Some(prefix) => Plan::resuming(&self.scene, target, regions, prefix),
            None => Plan::build(&self.scene, fresh, target, regions),
        };
        core.evaluate.tally.binned(plan.indices.len());
        self.plan = Some((key, plan));
    }
}
