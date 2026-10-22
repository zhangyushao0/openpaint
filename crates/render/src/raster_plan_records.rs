//! A raster plan's records (`raster_plan.rs`): what rewrites the layer (or its mask) from a copy
//! of it, a smudge, a point adjustment and a filter's result; the frame a transform record's
//! content lies in; a moved selection's or merged layer's content set on a
//! level of its own and, at a move's end, cut to its selection and laid over the level under it
//! (or a whole layer's cut to the canvas in place).

use super::{Builder, DrawKind, Instance, Pass, Scratch};
use crate::affine::Affine;
use crate::error::RenderError;
use crate::geom::{Bounds, PixelRect};
use crate::protocol::MOVE_DEPTH;
use crate::raster_kinds::Draw;
use crate::scene::Item;

impl Builder<'_> {
    /// A smudge (op item `item`, the scene's smudge number `smudge`) over the region's pixels the
    /// canvas pixels `reach` reach: what is laid there so far copied, then laid back moved by its
    /// residual. The layer or mask starts as nothing here when nothing was laid yet: the
    /// smudge may carry paint in from outside.
    pub(super) fn smudge(
        &mut self,
        (item, smudge): (usize, usize),
        reach: &PixelRect,
    ) -> Result<(), RenderError> {
        let rect = self.placing.reach_canvas(reach);
        if rect.is_empty() {
            return Ok(());
        }
        let Some(base) = self.bases.smudges.get(smudge).copied().flatten() else {
            if self.lacking {
                return Ok(());
            }
            return Err(RenderError::Invalid(
                "a smudge's residual is not in the arena",
            ));
        };
        let frame = self.placing.frame;
        self.rewrite((item, base, frame), rect, DrawKind::Smudge);
        Ok(())
    }

    /// The smudge being drawn (op `op`, whose residual is the in-progress one) over the canvas
    /// pixels `reach`, on the canvas (its place lies in no frame).
    pub(super) fn smudge_drawn(&mut self, op: usize, reach: &PixelRect) {
        let rect = self.placing.reach_canvas(reach);
        if !rect.is_empty() {
            self.rewrite((op, 0, 0), rect, DrawKind::Smudge);
        }
    }

    /// A point adjustment inside the region's pixels its bounds reach. On paint it changes
    /// nothing where nothing was laid (transparent stays), on a mask everywhere (its white).
    pub(super) fn adjust(&mut self, item: usize, bounds: Bounds) {
        let rect = self.placing.reach(&bounds);
        let target = self.target();
        if rect.is_empty() || (!self.on_mask && !self.touched[target.index()]) {
            return;
        }
        let frame = self.placing.frame;
        self.rewrite((item, 0, frame), rect, DrawKind::Adjust);
    }

    /// A filter, the scene's filter number `filter`: inside the region's pixels its bounds
    /// reach, the layer so far becomes its result (read from the arena block at its base), which
    /// may lie where the layer had nothing here.
    pub(super) fn filter(
        &mut self,
        (item, filter): (usize, usize),
        bounds: Bounds,
    ) -> Result<(), RenderError> {
        let rect = self.placing.reach(&bounds);
        if rect.is_empty() {
            return Ok(());
        }
        let Some(base) = self.bases.filters.get(filter).copied().flatten() else {
            if self.lacking {
                return Ok(());
            }
            return Err(RenderError::Invalid(
                "a filter's result is not in the arena",
            ));
        };
        let frame = self.placing.frame;
        self.rewrite((item, base, frame), rect, DrawKind::Filter);
        Ok(())
    }

    /// The target's pixels `rect` copied, then `kind` for op `item` (its block in the arena at
    /// `base`, in frame `frame`) written over them from that copy; the target cleared to nothing
    /// first unless it was in the region.
    fn rewrite(&mut self, (item, base, frame): (usize, u32, u32), rect: PixelRect, kind: DrawKind) {
        self.flush();
        let target = self.target();
        self.ensure_cleared(target);
        let start = self.plan.instances.len() as u32;
        self.plan.instances.push(Instance {
            a: [item as u32, base, frame, 0],
            b: [rect.min[0], rect.min[1], rect.max[0], rect.max[1]],
        });
        let draw = Draw {
            kind,
            instances: start..start + 1,
        };
        let [x, y] = rect.min.map(|at| at.max(0) as u32);
        let [width, height] = rect.size();
        let pixels = u64::from(width) * u64::from(height);
        self.plan.passes.push(Pass::Rewrite {
            target,
            rect: [x, y, width, height],
            draw,
        });
        if kind == DrawKind::Smudge {
            self.plan.counts.smudge_pixels += pixels;
        } else {
            self.plan.counts.record_pixels += pixels;
        }
        self.touched[target.index()] = true;
    }

    /// The units from here lie in the frame of item `item`: their bounds its pixels, mapped
    /// onto the canvas; draws name it, so their fragments are drawn where the frame maps the
    /// pixel's centre. A frame that maps nothing names none (its rows give the same point).
    pub(super) fn frame(&mut self, item: usize) {
        let Item::Frame {
            op,
            to_canvas,
            clip,
        } = &self.scene.items[item]
        else {
            return;
        };
        self.placing.enter(item, (to_canvas, *clip));
        let identity = to_canvas.is_identity();
        let foot = if identity {
            self.base_foot
        } else {
            self.base_foot * op.frame_foot()
        };
        self.placing.foot_scale = self.output.scale as f32 * foot;
    }

    /// The region's pixels lie on the canvas by `base` (a material's): bounds on the canvas
    /// map back into them, and the footprint follows its scale.
    pub(super) fn based(&mut self, base: Affine) {
        if base.is_identity() {
            return;
        }
        self.plan.base = base;
        self.placing.based(base);
        self.base_foot = (1.0 / base.scale()) as f32;
        self.placing.foot_scale = self.output.scale as f32 * self.base_foot;
    }

    /// A moved selection's or a merged layer's content starts on the next level, from nothing.
    pub(super) fn begin_move(&mut self) -> Result<(), RenderError> {
        self.flush();
        if u32::from(self.depth) >= MOVE_DEPTH {
            return Err(RenderError::Invalid("moves nested deeper than MOVE_DEPTH"));
        }
        self.depth += 1;
        self.on_mask = false;
        let level = self.paint().index();
        (self.cleared[level], self.touched[level]) = (false, false);
        Ok(())
    }

    /// A move's end (item `item`): a selection's content cut to its selection and laid over the
    /// level under it, or a whole layer's content cut to the canvas in place.
    pub(super) fn end_move(&mut self, item: usize) -> Result<(), RenderError> {
        self.flush();
        let source = self.paint();
        let touched = self.touched[source.index()];
        if !self.scene.items[item].op().sets_aside() {
            if touched {
                let draw = self.quad(item, self.whole(), DrawKind::Cut);
                self.plan.passes.push(Pass::Draw {
                    target: source,
                    clear: false,
                    draws: vec![draw],
                });
                self.counted();
            }
            return Ok(());
        }
        let under = self.level_down()?;
        if touched {
            self.ensure_cleared(under);
            let draw = self.quad(item, self.whole(), DrawKind::Moved);
            self.push_end((under, source), draw);
            self.touched[under.index()] = true;
        }
        Ok(())
    }

    /// The level below the one ended, which draws go to again.
    pub(super) fn level_down(&mut self) -> Result<Scratch, RenderError> {
        if self.depth == 0 {
            return Err(RenderError::Invalid("a move ends where none began"));
        }
        let ended = self.paint().index();
        (self.cleared[ended], self.touched[ended]) = (false, false);
        self.depth -= 1;
        self.on_mask = false;
        Ok(self.paint())
    }

    /// A move's or a group's end drawn over the whole region: counted with the records.
    pub(super) fn counted(&mut self) {
        let [width, height] = self.plan.size;
        self.plan.counts.record_pixels += u64::from(width) * u64::from(height);
    }

    /// A material's end: it is what the level its units ended on holds (its layer at full
    /// opacity over nothing), so a level a moved content left open is copied onto the canvas.
    pub(super) fn material_done(&mut self) {
        if self.depth == 0 || self.paint != Scratch::Canvas {
            return;
        }
        let level = self.paint();
        if self.touched[level.index()] {
            let to = Scratch::Canvas;
            self.plan.passes.push(Pass::Copy { from: level, to });
        } else {
            self.plan.passes.push(Pass::Clear(Scratch::Canvas));
        }
    }
}
