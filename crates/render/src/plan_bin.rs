//! Binning a scene's items into the plan's tiles (`plan.rs`): each tile lists, in scene order,
//! the items that reach it. A layer's end lists where its ops reached, a chain's end where its
//! layers' ends did, a group's end where its layers' and chains' ends did: each end resets what
//! it closes, so a tile nothing reached keeps no state from an earlier one.
//!
//! Transform records (S31): items under a frame have their bounds in the frame's content
//! pixels; they are cut to the frame's clip and mapped to the canvas before binning, and a tile
//! lists the frame's op before the first of them it lists. A moved selection's end is listed
//! wherever one of its items was. Its start, which sets the layer so far aside, only where the
//! layer had something there to set aside; elsewhere the moved content starts from nothing on
//! its own and its end only cuts it (`MovedOnly`). So the evaluator's stack holds only moves
//! that lie over content of their own level, not every move a nudged selection went through.

use super::{Grid, Touched};
use crate::affine::Affine;
use crate::geom::{Bounds, PixelRect};
use crate::protocol::FRESH;
use crate::scene::{Fresh, Item};

/// A moved selection being binned: its start's index, the tiles it entered, those where it set
/// the layer aside, and those where its own content was listed.
struct Move {
    begin: u32,
    entered: Touched,
    aside: Touched,
    filled: Touched,
}

pub(super) struct Binner<'a> {
    grid: &'a Grid<'a>,
    lists: Vec<Vec<u32>>,
    stroke: Touched,
    layer: Touched,
    chain: Touched,
    group: Touched,
    /// Where the layer's (or its mask's) own content was listed, outside every move.
    filled: Touched,
    moves: Vec<Move>,
    /// Per tile, how many moves set something aside there now, and the most ever.
    open: Vec<u32>,
    pub(super) deepest: u32,
    /// The frame items lie in now: its item (`None`: the canvas), the last frame item that is
    /// the canvas's, their bounds' map to the target's pixels (`None`: none) and their clip.
    frame: Option<u32>,
    canvas_frame: Option<u32>,
    map: Option<Affine>,
    clip: Option<Bounds>,
    /// Per tile, the frame its pixels are evaluated in now.
    tile_frames: Vec<Option<u32>>,
    /// Canvas pixels to the target's (a smudge's material under a record); `None`: the same.
    view: Option<Affine>,
    /// Items `[start, end)` whose entries give way to fresh op 0 at `end` (`Plan::resuming`).
    pub(super) resume: Option<[u32; 2]>,
    /// Items `[start, end)` never binned, the entry given instead in every tile at `start`
    /// (`Plan::over_snapshot`).
    pub(super) skip: Option<([u32; 2], u32)>,
}

impl<'a> Binner<'a> {
    pub(super) fn new(grid: &'a Grid<'a>, view: Option<Affine>) -> Self {
        let slots = grid.origins.len();
        Self {
            grid,
            lists: vec![Vec::new(); slots],
            stroke: Touched::new(slots),
            layer: Touched::new(slots),
            chain: Touched::new(slots),
            group: Touched::new(slots),
            filled: Touched::new(slots),
            moves: Vec::new(),
            open: vec![0; slots],
            deepest: 0,
            frame: None,
            canvas_frame: None,
            map: view,
            clip: None,
            tile_frames: vec![None; slots],
            view,
            resume: None,
            skip: None,
        }
    }

    /// Whether item `index` is one `skip` names; at the first, every tile lists its entry.
    fn skipped(&mut self, index: u32) -> bool {
        let Some(([start, end], entry)) = self.skip else {
            return false;
        };
        if index == start {
            // What the items would have left: the layer reached the tile, with content.
            for slot in 0..self.lists.len() {
                self.enter(slot);
                self.lists[slot].push(entry);
                self.filled.touch(slot);
            }
        }
        (start..end).contains(&index)
    }

    /// At the end of the items `resume` names: where they listed anything, their entries
    /// become fresh op 0.
    fn resume_at(&mut self, index: u32) {
        if let Some([start, end]) = self.resume
            && index == end
        {
            self.resume_from(start);
        }
    }

    /// Where the items from `start` on listed anything, their entries become fresh op 0.
    fn resume_from(&mut self, start: u32) {
        for list in &mut self.lists {
            if let Some(cut) = list.iter().position(|&i| i >= start && i < FRESH) {
                list.truncate(cut);
                list.push(FRESH);
            }
        }
    }

    /// Per evaluated tile, the indices (from `first` on) of `items` that reach it, in order,
    /// and the most moves any tile set aside at once.
    pub(super) fn bin(mut self, items: &[Item], first: u32, fresh: &Fresh) -> (Vec<Vec<u32>>, u32) {
        let mut fresh_lists = Some(self.grid.bin(fresh));
        for (index, item) in items.iter().enumerate() {
            let index = u32::try_from(index)
                .unwrap_or(u32::MAX)
                .saturating_add(first);
            self.resume_at(index);
            if self.skipped(index) {
                continue;
            }
            match item {
                Item::Segment { bounds, .. } => {
                    for slot in self.slots(*bounds) {
                        self.list(slot, index);
                        self.stroke.touch(slot);
                    }
                }
                Item::Fill { bounds, .. } => {
                    for slot in self.slots(*bounds) {
                        self.list(slot, index);
                    }
                }
                Item::Clear { bounds, .. } => {
                    // Clearing adds nothing to set aside.
                    for slot in self.slots(*bounds) {
                        self.enter(slot);
                        self.lists[slot].push(index);
                    }
                }
                Item::Reach(bounds) => {
                    for slot in self.slots(*bounds) {
                        self.enter(slot);
                        self.stroke.touch(slot);
                    }
                }
                Item::EndStroke(_) | Item::Smudge(_) => {
                    for slot in self.stroke.close() {
                        self.list(slot, index);
                    }
                }
                Item::Active => self.active(fresh_lists.take().unwrap_or_default()),
                Item::Frame {
                    to_canvas, clip, ..
                } => self.set_frame(index, *to_canvas, *clip),
                Item::BeginMove(_) => self.moves.push(Move {
                    begin: index,
                    entered: Touched::new(self.lists.len()),
                    aside: Touched::new(self.lists.len()),
                    filled: Touched::new(self.lists.len()),
                }),
                Item::EndMove { whole, .. } => self.end_move(index, *whole),
                // A merged layer's end, as a moved selection's (S39).
                Item::EndNested(_) => self.end_move(index, false),
                Item::MovedOnly(_) | Item::NestedOnly(_) => {}
                _ => self.end(index, item),
            }
        }
        (self.lists, self.deepest)
    }

    /// The in-progress stroke's place: in every slot its fresh segments `binned` there, then its
    /// paint op (fresh op 0).
    fn active(&mut self, binned: Vec<Vec<u32>>) {
        for (slot, mut list) in binned.into_iter().enumerate() {
            self.enter(slot);
            list.push(FRESH);
            self.lists[slot].append(&mut list);
            self.filled.touch(slot);
        }
    }

    /// Layer, chain and group ends and a mask's start, as before transform records.
    fn end(&mut self, index: u32, item: &Item) {
        let lists = &mut self.lists;
        match item {
            Item::EndLayer {
                chain: chained,
                grouped,
                ..
            } => {
                let into = if *chained {
                    Some(&mut self.chain)
                } else {
                    grouped.then_some(&mut self.group)
                };
                close_into(&mut self.layer, (lists, index), into);
                self.filled.close();
            }
            Item::EndChain { grouped, .. } => {
                let into = grouped.then_some(&mut self.group);
                close_into(&mut self.chain, (lists, index), into);
            }
            Item::EndGroup(_) => close_into(&mut self.group, (lists, index), None),
            Item::BeginMask(_) if !self.moves.is_empty() => {
                // A merged layer's mask (S39): where its own paint was listed.
                if let Some(level) = self.moves.last_mut() {
                    for &slot in &level.entered.slots {
                        lists[slot].push(index);
                    }
                    level.filled.close();
                }
            }
            Item::BeginMask(_) => {
                for slot in self.layer.close() {
                    lists[slot].push(index);
                    self.layer.touch(slot);
                }
                self.filled.close();
            }
            _ => {}
        }
    }

    /// The slots of the evaluated tiles an item with `bounds` (in the frame's pixels) reaches.
    fn slots(&self, bounds: Bounds) -> Vec<usize> {
        // On the canvas itself (no transform record's frame), far items are passed over at once.
        if self.map.is_none() && !self.grid.may_reach(&bounds.0) {
            return Vec::new();
        }
        let Some(rect) = self.place(bounds) else {
            return Vec::new();
        };
        self.grid.slots(&rect)
    }

    /// `bounds` cut to the frame's clip and mapped to the target's pixels; `None` when the
    /// clip leaves nothing.
    fn place(&self, Bounds(b): Bounds) -> Option<PixelRect> {
        let bounds = match self.clip {
            Some(Bounds(c)) => {
                let cut = [
                    b[0].max(c[0]),
                    b[1].max(c[1]),
                    b[2].min(c[2]),
                    b[3].min(c[3]),
                ];
                (cut[0] <= cut[2] && cut[1] <= cut[3]).then_some(Bounds(cut))?
            }
            None => Bounds(b),
        };
        Some(match self.map {
            Some(map) => map.bounds(bounds).pixels(),
            None => bounds.pixels(),
        })
    }

    fn set_frame(&mut self, index: u32, to_canvas: Affine, clip: Option<Bounds>) {
        let map = match self.view {
            Some(view) => Some(to_canvas.then(view)),
            None => Some(to_canvas),
        };
        self.map = map.filter(|map| !map.is_identity());
        self.clip = clip;
        if to_canvas.is_identity() && clip.is_none() {
            self.frame = None;
            self.canvas_frame = Some(index);
        } else {
            self.frame = Some(index);
        }
    }

    /// A move's end. A whole layer's is listed wherever the content it cuts was. A selection's
    /// wherever its items were: as `index` where it set the layer aside, else as the
    /// `MovedOnly` item after it.
    fn end_move(&mut self, index: u32, whole: bool) {
        if whole {
            let slots = match self.moves.last() {
                Some(level) => level.entered.slots.clone(),
                None => self.layer.slots.clone(),
            };
            for slot in slots {
                self.enter(slot);
                self.lists[slot].push(index);
            }
            return;
        }
        let Some(mut level) = self.moves.pop() else {
            return;
        };
        for slot in level.entered.close() {
            let aside = level.aside.has(slot);
            self.list(slot, if aside { index } else { index + 1 });
            if aside {
                self.open[slot] -= 1;
            }
        }
    }

    /// An item of the current level in `slot`: entered there, then marked as content.
    fn list(&mut self, slot: usize, index: u32) {
        self.enter(slot);
        self.lists[slot].push(index);
        match self.moves.last_mut() {
            Some(level) => level.filled.touch(slot),
            None => self.filled.touch(slot),
        }
    }

    /// Before an item in `slot`: the layer reaches it, each open move starts there (setting
    /// aside what its level already holds there), and its pixels are evaluated in the frame
    /// the item lies in.
    fn enter(&mut self, slot: usize) {
        self.layer.touch(slot);
        for at in 0..self.moves.len() {
            if self.moves[at].entered.has(slot) {
                continue;
            }
            let below = match at.checked_sub(1) {
                Some(outer) => self.moves[outer].filled.has(slot),
                None => self.filled.has(slot),
            };
            if below {
                self.lists[slot].push(self.moves[at].begin);
                self.moves[at].aside.touch(slot);
                self.open[slot] += 1;
                self.deepest = self.deepest.max(self.open[slot]);
            }
            self.moves[at].entered.touch(slot);
        }
        if self.tile_frames[slot] != self.frame {
            if let Some(frame) = self.frame.or(self.canvas_frame) {
                self.lists[slot].push(frame);
            }
            self.tile_frames[slot] = self.frame;
        }
    }
}

/// An end: listed in every slot `from` reached, which `into` (what the end composites into)
/// has then reached too.
fn close_into(
    from: &mut Touched,
    (lists, index): (&mut [Vec<u32>], u32),
    into: Option<&mut Touched>,
) {
    let slots = from.close();
    for slot in &slots {
        lists[*slot].push(index);
    }
    if let Some(into) = into {
        for slot in slots {
            into.touch(slot);
        }
    }
}
