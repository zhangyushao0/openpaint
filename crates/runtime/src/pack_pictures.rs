//! The practice options' pictures (S83): a pack's picture read and reduced to a small one on a
//! worker of its own while the options show it (the picker of a colour copy's picture, the
//! copies, My pictures), one at a time, the first the options want first, so neither the frame
//! nor the store waits for them. The UI keeps them for the app's run; derived, never stored.

use std::time::Instant;

use openpaint_app::PictureImage;
use openpaint_practice::ItemRef;
use openpaint_practice_store::{Packs, thumbnail_picture};
use openpaint_ui::Shell;

use crate::error::RuntimeError;
use crate::worker::Queue;

/// The longest side of a picture the options show, in pixels: a row's picture of 96 points at
/// density 1.75, and the larger one a page shows, about 150 points.
const SIDE: u32 = 256;

pub(crate) struct PackPictureWorker {
    worker: Queue<ItemRef, (ItemRef, Option<PictureImage>)>,
}

impl PackPictureWorker {
    /// The worker reading pictures from `packs` (none kept: every picture is none).
    pub(crate) fn start(packs: Option<Packs>) -> Result<Self, RuntimeError> {
        let worker = Queue::spawn("openpaint-pack-pictures", move |item: ItemRef| {
            let started = Instant::now();
            let picture = packs.as_ref().and_then(|packs| read(packs, &item));
            log::debug!(
                "picture {}/{} for the options in {} ms",
                item.pack,
                item.item,
                started.elapsed().as_millis()
            );
            (item, picture)
        })?;
        Ok(Self { worker })
    }

    /// True while a picture is being read or waits to be taken.
    pub(crate) fn busy(&self) -> bool {
        self.worker.busy()
    }

    /// One frame's pictures: those read go to the options (one that could not be read as none),
    /// the next the options want to the worker.
    pub(crate) fn step(&mut self, shell: &mut Shell) {
        while let Some((item, picture)) = self.worker.take() {
            shell.set_pack_picture(item, picture);
        }
        if !self.worker.busy()
            && let Some(item) = shell.pack_picture_wanted()
        {
            self.worker.submit(item);
        }
    }
}

/// `item`'s picture reduced for the options; none when it cannot be (logged).
fn read(packs: &Packs, item: &ItemRef) -> Option<PictureImage> {
    let file = packs
        .picture(item)
        .inspect_err(|error| log::warn!("picture {}/{} unread: {error}", item.pack, item.item))
        .ok()
        .flatten()?;
    thumbnail_picture(&file, SIDE)
        .inspect_err(|error| log::warn!("picture {}/{} undecoded: {error}", item.pack, item.item))
        .ok()
}
