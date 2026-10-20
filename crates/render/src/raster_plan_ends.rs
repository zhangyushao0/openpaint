//! A raster plan's ends (`raster_plan.rs`): what a layer reached composited onto the canvas or
//! into its clipping chain by its mode, through its mask, from a copy of what lies under it where
//! fixed-function blending cannot (Overlay); S65 (D92): a merged layer's content laid onto the
//! level under it by the same expressions, a group's layers ending onto the canvas after what lay
//! under the group was copied, and its end fading the canvas over that copy.

use super::{Builder, Chain};
use crate::error::RenderError;
use crate::geom::PixelRect;
use crate::protocol::{LayerInto, Masking, Mode};
use crate::raster_kinds::{Draw, DrawKind, EndBlend, Pass, Scratch};
use crate::raster_scene::opens_group;
use crate::scene::Item;

impl Builder<'_> {
    /// A layer's or a chain's end (item `item`): what it reached composited by its mode.
    pub(super) fn end(&mut self, item: usize) {
        self.flush();
        let found = &self.scene.items[item];
        let end = found.op().layer_end();
        if opens_group(found) && self.group.is_none() {
            // What lies under the group, as its first layer ending onto the canvas finds it.
            let (from, to) = (Scratch::Canvas, Scratch::Group);
            self.plan.passes.push(Pass::Copy { from, to });
            self.group = Some(false);
        }
        if matches!(found, Item::EndChain { .. }) {
            if self.chain == Chain::Held {
                self.onto((Scratch::Canvas, Scratch::Chain), item, (end.mode, false));
                self.group = self.group.map(|_| true);
            }
            self.chain = Chain::Empty;
            return;
        }
        let touched = self.touched[Scratch::Layer.index()];
        if touched && end.masking != Masking::None {
            // A mask that drew nothing here is nothing: the layer shows through white.
            self.ensure_cleared(Scratch::Mask);
        }
        match end.into {
            LayerInto::Canvas if touched => {
                self.onto((Scratch::Canvas, Scratch::Layer), item, (end.mode, false));
                self.group = self.group.map(|_| true);
            }
            LayerInto::Base if touched => {
                let draw = self.quad(item, self.whole(), DrawKind::End(EndBlend::Base));
                self.push_end((Scratch::Chain, Scratch::Layer), draw);
                self.chain = Chain::Held;
            }
            LayerInto::Base => self.chain = Chain::Empty,
            LayerInto::Clipped if touched && self.chain == Chain::Held => {
                self.onto((Scratch::Chain, Scratch::Layer), item, (end.mode, true));
            }
            _ => {}
        }
        self.on_mask = false;
        for target in [Scratch::Layer, Scratch::Mask] {
            (self.cleared[target.index()], self.touched[target.index()]) = (false, false);
        }
    }

    /// A merged layer's end (S65, item `item`): its content (the level ended) through its mask at
    /// its opacity laid onto the level under it by its mode, over it or atop it (clipped), as its
    /// layer's end laid it onto the canvas or its chain before the merge.
    pub(super) fn end_nested(&mut self, item: usize) -> Result<(), RenderError> {
        self.flush();
        let end = self.scene.items[item].op().layer_end();
        let source = self.paint();
        let touched = self.touched[source.index()];
        if touched && end.masking != Masking::None {
            self.ensure_cleared(Scratch::Mask);
        }
        let under = self.level_down()?;
        if touched {
            self.ensure_cleared(under);
            let clipped = end.into == LayerInto::Clipped;
            self.lay_end((under, source), item, (end.mode, clipped), false);
            self.touched[under.index()] = true;
        }
        let mask = Scratch::Mask.index();
        (self.cleared[mask], self.touched[mask]) = (false, false);
        Ok(())
    }

    /// A group's end (S65, item `item`): where one of its layers reached the region, the canvas
    /// faded by the group's opacity over what lay under the group (`group_fragment`), written
    /// whole from a copy of the canvas.
    pub(super) fn end_group(&mut self, item: usize) {
        self.flush();
        if self.group.take() != Some(true) {
            return;
        }
        let (from, to) = (Scratch::Canvas, Scratch::Backdrop);
        self.plan.passes.push(Pass::Copy { from, to });
        let draw = self.quad(item, self.whole(), DrawKind::End(EndBlend::Group));
        self.push_end((Scratch::Canvas, Scratch::Group), draw);
        self.counted();
    }

    /// The whole region, in its pixels.
    pub(super) fn whole(&self) -> PixelRect {
        PixelRect::full(self.plan.size)
    }

    /// The end of op `item` onto `target` (the opaque canvas, or `clipped` into the chain) by
    /// `mode`, reading `reads`.
    fn onto(&mut self, places: (Scratch, Scratch), item: usize, look: (Mode, bool)) {
        self.lay_end(places, item, look, true);
    }

    /// The end of op `item` onto `target` by `mode`, atop it when `clipped`, reading `reads`: by
    /// the blend unit where its equation is one (`opaque`: the target is the canvas),
    /// else written whole from a copy of the target.
    fn lay_end(
        &mut self,
        (target, reads): (Scratch, Scratch),
        item: usize,
        (mode, clipped): (Mode, bool),
        opaque: bool,
    ) {
        let blend = match (mode, clipped) {
            (Mode::Normal, false) => EndBlend::Over,
            (Mode::Normal, true) => EndBlend::Atop,
            (Mode::Multiply, true) => EndBlend::Multiply,
            (Mode::Multiply, false) if opaque => EndBlend::Multiply,
            (Mode::Screen, false) if opaque => EndBlend::ScreenCanvas,
            (Mode::Screen, true) => EndBlend::ScreenChain,
            _ => {
                let to = Scratch::Backdrop;
                self.plan.passes.push(Pass::Copy { from: target, to });
                EndBlend::Backdrop
            }
        };
        let draw = self.quad(item, self.whole(), DrawKind::End(blend));
        self.push_end((target, reads), draw);
    }

    pub(super) fn push_end(&mut self, (target, reads): (Scratch, Scratch), draw: Draw) {
        self.plan.counts.ends += 1;
        self.plan.passes.push(Pass::End {
            target,
            reads,
            draw,
        });
    }
}
