//! A raster plan's layer and chain ends (`raster_plan.rs`): what a layer reached composited onto
//! the canvas or into its clipping chain by its mode, through its mask, from a copy of what lies
//! under it where fixed-function blending cannot (Overlay).

use super::{Builder, Chain};
use crate::geom::PixelRect;
use crate::protocol::{LayerInto, Masking, Mode};
use crate::raster_kinds::{Draw, DrawKind, EndBlend, Pass, Scratch};
use crate::scene::Item;

impl Builder<'_> {
    /// A layer's or a chain's end (item `item`): what it reached composited by its mode.
    pub(super) fn end(&mut self, item: usize) {
        self.flush();
        let end = self.scene.items[item].op().layer_end();
        if matches!(self.scene.items[item], Item::EndChain { .. }) {
            if self.chain == Chain::Held {
                self.onto((Scratch::Canvas, Scratch::Chain), item, (end.mode, false));
            }
            self.chain = Chain::Empty;
            return;
        }
        let touched = self.touched[0];
        if touched && end.masking != Masking::None && !self.cleared[1] {
            // A mask that drew nothing here is nothing: the layer shows through white.
            self.plan.passes.push(Pass::Clear(Scratch::Mask));
        }
        match end.into {
            LayerInto::Canvas if touched => {
                self.onto((Scratch::Canvas, Scratch::Layer), item, (end.mode, false));
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
        self.target = Scratch::Layer;
        (self.cleared, self.touched) = ([false; 2], [false; 2]);
    }

    /// The whole region, in its pixels.
    fn whole(&self) -> PixelRect {
        PixelRect::full(self.plan.size)
    }

    /// The end of op `item` onto `target` (the canvas, or `clipped` into the chain) by `mode`,
    /// reading `reads`.
    fn onto(
        &mut self,
        (target, reads): (Scratch, Scratch),
        item: usize,
        (mode, clipped): (Mode, bool),
    ) {
        let blend = match (mode, clipped) {
            (Mode::Normal, false) => EndBlend::Over,
            (Mode::Normal, true) => EndBlend::Atop,
            (Mode::Multiply, _) => EndBlend::Multiply,
            (Mode::Screen, false) => EndBlend::ScreenCanvas,
            (Mode::Screen, true) => EndBlend::ScreenChain,
            (Mode::Overlay, _) => {
                self.plan.passes.push(Pass::Copy(target));
                EndBlend::Overlay
            }
        };
        let draw = self.quad(item, self.whole(), DrawKind::End(blend));
        self.push_end((target, reads), draw);
    }

    fn push_end(&mut self, (target, reads): (Scratch, Scratch), draw: Draw) {
        self.plan.counts.ends += 1;
        self.plan.passes.push(Pass::End {
            target,
            reads,
            draw,
        });
    }
}
