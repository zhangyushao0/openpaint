//! `RasterWorker` (S60, DECISIONS D87): the raster core's thread, `openpaint-raster`. As the
//! device is made it makes the raster core's pipelines (0.7 s on the tablet), so the first zoom
//! finds them; then it plans, encodes and submits the view tiers' tiles the canvas hands it
//! (`openpaint_render::RasterDrawer`), each once the GPU's queue of background work has room, so
//! tiles filling cost the frame loop nothing but handing them over. A tile handed over while
//! nothing touched the screen is dropped unsubmitted once a hand arrives (a touch, the pen
//! coming into hover: `hand_arrived` from `Runtime::motion`), so the first frame of a touch
//! waits for at most the background work already queued. Its targets live on this thread, made
//! again on a new device (S49) and let go when the GPU memory budget says.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Weak};
use std::time::{Duration, Instant};

use openpaint_render::{Core, RasterAnswer, RasterDrawer, RasterOrder};

use crate::error::RuntimeError;
use crate::hints::WorkHints;
use crate::shared::SharedCore;
use crate::worker::Queue;

/// How long an offscreen frame waits for the worker at most.
const DRAIN: Duration = Duration::from_secs(10);

enum Request {
    /// Make the raster core's pipelines on the device now.
    Warm,
    /// This thread does the frames' work for the host's performance hints (S67).
    Join(Arc<dyn WorkHints>),
    /// An order of the view's; a tile handed over while nothing touched the screen carries the
    /// hands counted then (`ahead`).
    Order {
        order: RasterOrder,
        ahead: Option<u64>,
    },
}

pub(crate) struct RasterWorker {
    queue: Queue<Request, Option<RasterAnswer>>,
    /// Hands arrived so far.
    hands: Arc<AtomicU64>,
}

impl RasterWorker {
    /// Starts the thread, which makes the pipelines first.
    pub(crate) fn start(shared: SharedCore) -> Result<Self, RuntimeError> {
        let hands = Arc::new(AtomicU64::new(0));
        let seen = Arc::clone(&hands);
        let mut drawer = RasterDrawer::default();
        let mut device: Weak<Core> = Weak::new();
        let queue = Queue::spawn("openpaint-raster", move |request: Request| {
            let core = shared.get();
            // Targets made on a device lost since are of no use on this one.
            if !Weak::ptr_eq(&device, &Arc::downgrade(&core)) {
                drawer.let_go();
                device = Arc::downgrade(&core);
            }
            match request {
                Request::Warm => {
                    warm(&core, &mut drawer);
                    None
                }
                Request::Join(hints) => {
                    hints.joined();
                    None
                }
                Request::Order {
                    order: RasterOrder::LetGo,
                    ..
                } => {
                    drawer.let_go();
                    None
                }
                Request::Order {
                    order: RasterOrder::Draw(job),
                    ahead,
                } => {
                    let wanted = || wanted(ahead, seen.load(Ordering::Acquire));
                    let done = drawer.draw(&core, (*job, ahead.is_some()), wanted);
                    Some(RasterAnswer::Tile(done))
                }
                Request::Order {
                    order: RasterOrder::Atlas(slots),
                    ..
                } => RasterDrawer::atlas(&core, slots).map(RasterAnswer::Atlas),
            }
        })?;
        let mut worker = Self { queue, hands };
        worker.warm();
        Ok(worker)
    }

    /// The pipelines are made on the device now (at the start, and on a new device).
    pub(crate) fn warm(&mut self) {
        self.queue.submit(Request::Warm);
    }

    /// The worker's thread joins the host's performance hints (S67).
    pub(crate) fn join(&mut self, hints: Arc<dyn WorkHints>) {
        self.queue.submit(Request::Join(hints));
    }

    /// A hand arrived: tiles handed over while nothing touched go unsubmitted.
    pub(crate) fn hand_arrived(&self) {
        self.hands.fetch_add(1, Ordering::AcqRel);
    }

    /// Hands `orders` over in order; `idle`: while nothing touches the screen.
    pub(crate) fn send(&mut self, orders: Vec<RasterOrder>, idle: bool) {
        let ahead = idle.then(|| self.hands.load(Ordering::Acquire));
        for order in orders {
            self.queue.submit(Request::Order { order, ahead });
        }
    }

    /// Waits until every order handed over is served (at most `DRAIN`), its answers kept for
    /// `answers` (S60: an offscreen frame, read back, waits for the worker as for the GPU).
    pub(crate) fn drain(&mut self) {
        if !self.queue.drain(DRAIN) {
            log::warn!("the raster worker still works after {} s", DRAIN.as_secs());
        }
    }

    /// The answers since the last call.
    pub(crate) fn answers(&mut self) -> Vec<RasterAnswer> {
        std::iter::from_fn(|| self.queue.take()).flatten().collect()
    }
}

/// Whether a tile handed over at `ahead` (hands counted then; `None`: while a hand touched) is
/// still wanted with `hands` arrived now.
pub(crate) const fn wanted(ahead: Option<u64>, hands: u64) -> bool {
    match ahead {
        Some(at) => at == hands,
        None => true,
    }
}

/// Makes the raster core's pipelines on `core` and `drawer`'s targets, logged.
fn warm(core: &Core, drawer: &mut RasterDrawer) {
    let started = Instant::now();
    match drawer.warm(core) {
        Ok(()) => log::info!(
            "raster core ready {} ms after the worker asked ({:.1} MB of targets)",
            started.elapsed().as_millis(),
            drawer.bytes() as f64 / (1024.0 * 1024.0)
        ),
        Err(error) => log::error!("the raster core could not be made ready: {error}"),
    }
}

#[cfg(test)]
mod tests {
    use super::wanted;

    #[test]
    fn a_tile_handed_over_while_idle_is_dropped_once_a_hand_arrives() {
        assert!(wanted(Some(3), 3), "no hand since");
        assert!(!wanted(Some(3), 4), "a hand arrived since");
        assert!(wanted(None, 9), "handed over while a hand touched: drawn");
    }
}
