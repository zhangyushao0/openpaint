//! How one evaluation reads a canvas target's kept scene: made ready first (the preview's
//! snapshot taken, the plan binned or kept, S35, S48), then read without changing the cache,
//! so the residuals near the tiles it evaluates can be made present in between (S51).

use std::ops::Range;

use super::{PlanKey, Reading, SceneCache, Skipped};
use crate::core::Core;
use crate::error::RenderError;
use crate::geom::PixelRect;
use crate::output::Output;
use crate::plan::Plan;
use crate::preview::PreviewSnapshot;
use crate::program::{Bindings, SnapshotUse};
use crate::protocol::{Op, TileRecord};
use crate::scene::Fresh;

/// What `SceneCache::ready` made ready for `reading`: the fresh ops, whether they resume the
/// in-progress stroke's layer from its snapshot, and whether a preview resumes from its own.
pub(super) struct Readied {
    fresh: Vec<Op>,
    over: bool,
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
    /// in-progress stroke's evaluations bin their own plan each time, over its layer's snapshot
    /// (S48: the items `over`) when there is one; others keep it, and a previewed adjustment's
    /// layer resumes from its snapshot (taken first through the target's `bindings`, with the
    /// arena and in-progress residual `residuals`).
    pub(super) fn ready(
        &mut self,
        core: &Core,
        (bindings, residuals): (&mut Bindings, [&wgpu::Buffer; 2]),
        fresh: Fresh,
        (size, output, regions): ([u32; 2], Output, &[PixelRect]),
        over: Option<Range<usize>>,
    ) -> Result<Readied, RenderError> {
        let previewing = fresh.ops.is_empty() && output == Output::CANVAS;
        if let Some(place) = self.previewed.filter(|_| previewing)
            && self.snapshot.as_ref().is_none_or(|s| s.place != place)
        {
            let scene = (&self.scene, &self.ops);
            self.snapshot = PreviewSnapshot::take(core, scene, (bindings, residuals), place)?;
        }
        let resume = self
            .snapshot
            .as_ref()
            .filter(|s| previewing && self.previewed == Some(s.place))
            .map(|s| s.prefix.clone());
        let (fresh, skipped) = match over.filter(|_| !fresh.ops.is_empty()) {
            Some(prefix) => {
                let (fresh, at) = fresh.resuming();
                (fresh, Some((prefix, at)))
            }
            None => (fresh, None),
        };
        let over = skipped.is_some();
        self.plan_for(
            core,
            &fresh,
            (size, output, regions),
            (resume.clone(), skipped),
        );
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
            over,
            resume: resume.is_some(),
        })
    }

    /// How the evaluation `ready` made ready reads this scene, `held` the in-progress stroke's
    /// layer snapshot it resumes from.
    pub(super) fn reading<'a>(
        &'a self,
        readied: Readied,
        held: Option<SnapshotUse<'a>>,
    ) -> Result<Reading<'a>, RenderError> {
        let resume = readied.resume.then_some(());
        let plan = self.plan.as_ref().map(|(_, plan)| plan);
        Ok(Reading {
            ops: &self.ops,
            plan: plan.ok_or(RenderError::Invalid("no plan was built"))?,
            fresh: readied.fresh,
            snapshot: match held.filter(|_| readied.over) {
                Some(snapshot) => Some(snapshot),
                None => resume
                    .and(self.snapshot.as_ref())
                    .map(|snapshot| snapshot.reading(false)),
            },
            only: resume
                .and(self.near.as_ref())
                .map(|(_, _, tiles)| tiles.as_slice()),
        })
    }

    /// Bins the plan for `regions` at `output`, unless the plan kept was binned for them and
    /// nothing is fresh: resuming a previewed layer (`resume`), over the in-progress stroke's
    /// layer snapshot (`over`: its items and the fresh op resuming them), or every item.
    fn plan_for(
        &mut self,
        core: &Core,
        fresh: &Fresh,
        (size, output, regions): ([u32; 2], Output, &[PixelRect]),
        (resume, over): (Option<Range<usize>>, Option<Skipped>),
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
        let plan = match (resume, over) {
            (Some(prefix), _) => Plan::resuming(&self.scene, target, regions, prefix),
            (None, Some(over)) => Plan::over_snapshot(&self.scene, fresh, target, regions, over),
            (None, None) => Plan::build(&self.scene, fresh, target, regions),
        };
        core.evaluate.tally.binned(plan.indices.len());
        self.plan = Some((key, plan));
    }
}
