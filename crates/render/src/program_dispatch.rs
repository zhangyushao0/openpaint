//! One evaluation of a plan into a target, in bounded runs of its tiles (S49, `bounded.rs`): the
//! plan's op lists are written once, then each run writes its tiles and the dispatch's values,
//! is dispatched and submitted on its own, so no submission runs past the GPU's time limit. The
//! runs follow one another on the queue, each reading what the one before left.

use std::sync::atomic::Ordering;

use super::layout::{Workgroups, workgroups};
use super::{Bindings, Dispatch, EvaluatePipeline};
use crate::bounded::{FRAME_DISPATCHES, runs, tile_work};
use crate::error::RenderError;
use crate::gpu::Gpu;
use crate::protocol::{MOVE_DEPTH, Params, encode};
use crate::readback::wait_for;

/// The most work one run of a worker's evaluation takes (`pace_dispatches`, S53): op entries
/// about what the screen's frame gives background work on the tablet.
const PACED_WORK: u64 = 500;

impl EvaluatePipeline {
    /// Writes the plan and the fresh ops (behind the scene's), then evaluates every plan tile into the target, in
    /// runs of at most the bound's work, each submitted on its own.
    pub(crate) fn dispatch(
        &self,
        gpu: &Gpu,
        bindings: &mut Bindings,
        work: &Dispatch<'_>,
    ) -> Result<(), RenderError> {
        if work.plan.deepest > MOVE_DEPTH {
            return Err(RenderError::Invalid(
                "moved selections lie over one another deeper than the evaluator holds",
            ));
        }
        let tiles = work.only.unwrap_or(&work.plan.tiles);
        // A plan kept between evaluations (a preview's, S35) uploads its op lists once.
        let written = (work.plan.serial, work.ops.count());
        if work.plan.serial == 0 || written != bindings.written {
            bindings.write(gpu, 1, &work.ops.entries(&work.plan.indices));
            bindings.written = written;
        }
        // The fresh ops behind the scene's, in the buffer every run binds (S55).
        let ops = work.ops.with_fresh(gpu, work.fresh);
        let most = if bindings.paced {
            PACED_WORK.min(self.work())
        } else {
            self.work()
        };
        for (index, run) in runs(tiles, most).into_iter().enumerate() {
            let groups = workgroups(gpu, run.len())?;
            let params = params(work, bindings, &groups);
            gpu.queue
                .write_buffer(&bindings.params, 0, &encode(&[params]));
            bindings.write(gpu, 0, run);
            self.bind(gpu, bindings, work, &ops);
            let mut encoder = gpu.device.create_command_encoder(&Default::default());
            // The in-progress stroke starts over before its first run only.
            if work.reset && index == 0 {
                for (_, view) in &bindings.kept {
                    clear_kept(&mut encoder, view);
                }
            }
            if let Some((_, group)) = bindings.group.as_ref().filter(|_| !run.is_empty()) {
                // S53: background work's runs are timed, from the first's start to the last's end.
                let timestamp_writes = work
                    .timing
                    .map(|(clock, timing)| clock.writes(timing, index == 0));
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: None,
                    timestamp_writes,
                });
                let pipeline = if work.plan.deepest > 0 {
                    &self.pipeline
                } else {
                    &self.flat
                };
                pass.set_pipeline(pipeline);
                pass.set_bind_group(0, group, &[]);
                pass.dispatch_workgroups(groups.row, groups.rows, 1);
            }
            let submitted = gpu.submit([encoder.finish()]);
            self.runs.fetch_add(1, Ordering::Relaxed);
            self.tally.dispatched(run.iter().map(tile_work).sum());
            // A worker's run, so the next waits for the screen's frames between them
            // (`pace_dispatches`, S53).
            if bindings.paced {
                wait_for(gpu, submitted, "a worker's evaluation")?;
            }
        }
        Ok(())
    }

    /// Dispatches submitted so far.
    pub(crate) fn runs(&self) -> u64 {
        self.runs.load(Ordering::Relaxed)
    }

    /// Builds the bind group again when a list grew or another scene, arena, residual or
    /// snapshot is bound.
    fn bind(&self, gpu: &Gpu, bindings: &mut Bindings, work: &Dispatch<'_>, ops: &wgpu::Buffer) {
        let [arena, active] = work.residuals;
        let snapshot = work.snapshot.map_or(&self.no_snapshot, |s| s.buffer);
        let named = [ops, arena, active, snapshot];
        if bindings
            .group
            .as_ref()
            .is_none_or(|(built, _)| built.iter().zip(named).any(|(a, b)| a != b))
        {
            let group = self.bind_group(gpu, bindings, work.ops, named);
            bindings.group = Some((named.map(Clone::clone), group));
        }
    }

    /// The most work one dispatch takes (`bounded::DISPATCH_WORK` at first).
    pub(crate) fn work(&self) -> u64 {
        self.bound.load(Ordering::Relaxed)
    }

    /// The most work the screen's canvas takes in one frame.
    pub(crate) fn frame_work(&self) -> u64 {
        self.work().saturating_mul(FRAME_DISPATCHES)
    }

    /// Sets the most work one dispatch takes.
    pub(crate) fn bound_to(&self, work: u64) {
        self.bound.store(work.max(1), Ordering::Relaxed);
    }
}

/// The dispatch's values: where the target lies in the output and on the canvas, its tiles'
/// layout, the snapshot it uses.
fn params(work: &Dispatch<'_>, bindings: &Bindings, groups: &Workgroups) -> Params {
    let (scale, inverse) = work.output.operands();
    let [a, b, c, d, e, f] = work.base.rows();
    let (foot, level) = if work.base.is_identity() {
        (1.0, 0.0)
    } else {
        let canvas = work.base.scale();
        ((1.0 / canvas) as f32, -canvas.log2() as f32)
    };
    Params {
        background: work.background,
        tiles: groups.tiles,
        row: groups.row,
        size: bindings.size,
        kept: u32::from(bindings.keeps),
        level: scale.log2(),
        scale,
        inverse,
        base_x: [a, b, c, foot],
        base_y: [d, e, f, level],
        snapshot: work.snapshot.map_or([0; 4], |snapshot| {
            let [columns, head] = snapshot.layout;
            [columns, head, 0, u32::from(snapshot.writes)]
        }),
        display: [u32::from(bindings.display), 0, 0, 0],
    }
}

/// Zero everywhere, through a render pass: clearing a texture directly needs an optional
/// device feature.
fn clear_kept(encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
    let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("clear kept"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                store: wgpu::StoreOp::Store,
            },
        })],
        ..Default::default()
    });
    drop(pass);
}
