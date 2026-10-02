//! `Latest`: one background thread serving one request at a time. A new request replaces the
//! one still waiting; the result waits until the frame loop takes it. Dropping it serves the
//! request still waiting, then joins the thread, so nothing handed over is lost on shutdown.
//!
//! This module is the only place product code starts a thread (ARCHITECTURE section 3).

use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::error::RuntimeError;

struct Slots<Req, Out> {
    /// The request waiting to run; a newer one replaces it.
    waiting: Option<Req>,
    running: bool,
    /// The newest result not taken yet.
    done: Option<Out>,
    closing: bool,
}

struct Shared<Req, Out> {
    slots: Mutex<Slots<Req, Out>>,
    changed: Condvar,
}

impl<Req, Out> Shared<Req, Out> {
    /// The job runs outside the lock, so a poisoned lock only means a panic elsewhere while
    /// holding it; the slots stay consistent and are used as they are.
    fn lock(&self) -> MutexGuard<'_, Slots<Req, Out>> {
        self.slots.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

pub(crate) struct Latest<Req, Out> {
    shared: Arc<Shared<Req, Out>>,
    thread: Option<JoinHandle<()>>,
}

impl<Req: Send + 'static, Out: Send + 'static> Latest<Req, Out> {
    /// Starts the thread `name`, which runs `job` for each request.
    pub(crate) fn spawn(
        name: &'static str,
        mut job: impl FnMut(Req) -> Out + Send + 'static,
    ) -> Result<Self, RuntimeError> {
        let shared = Arc::new(Shared {
            slots: Mutex::new(Slots {
                waiting: None,
                running: false,
                done: None,
                closing: false,
            }),
            changed: Condvar::new(),
        });
        let serving = Arc::clone(&shared);
        let thread = std::thread::Builder::new()
            .name(name.to_owned())
            .spawn(move || serve(&serving, &mut job))
            .map_err(|_| RuntimeError::Worker(name))?;
        Ok(Self {
            shared,
            thread: Some(thread),
        })
    }
}

impl<Req, Out> Latest<Req, Out> {
    /// Hands `request` to the thread, replacing one still waiting.
    pub(crate) fn submit(&self, request: Req) {
        self.shared.lock().waiting = Some(request);
        self.shared.changed.notify_all();
    }

    /// The newest finished result, once.
    pub(crate) fn take(&self) -> Option<Out> {
        self.shared.lock().done.take()
    }

    /// Waits until no request is waiting or running, at most `limit`. Returns whether it is
    /// idle; the results stay for `take`.
    pub(crate) fn drain(&self, limit: Duration) -> bool {
        let deadline = Instant::now() + limit;
        let mut slots = self.shared.lock();
        while slots.waiting.is_some() || slots.running {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return false;
            }
            slots = self
                .shared
                .changed
                .wait_timeout(slots, left)
                .unwrap_or_else(PoisonError::into_inner)
                .0;
        }
        true
    }
}

impl<Req, Out> Drop for Latest<Req, Out> {
    fn drop(&mut self) {
        self.shared.lock().closing = true;
        self.shared.changed.notify_all();
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            log::error!("a worker thread panicked");
        }
    }
}

/// The thread's loop: wait for a request, run it, publish the result; after `closing`, finish
/// the request still waiting and stop.
fn serve<Req, Out>(shared: &Shared<Req, Out>, job: &mut impl FnMut(Req) -> Out) {
    loop {
        let request = {
            let mut slots = shared.lock();
            loop {
                if let Some(request) = slots.waiting.take() {
                    slots.running = true;
                    break request;
                }
                if slots.closing {
                    return;
                }
                slots = shared
                    .changed
                    .wait(slots)
                    .unwrap_or_else(PoisonError::into_inner);
            }
        };
        let out = job(request);
        let mut slots = shared.lock();
        slots.running = false;
        slots.done = Some(out);
        drop(slots);
        shared.changed.notify_all();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use super::*;

    #[test]
    fn results_come_back_and_a_newer_request_replaces_a_waiting_one() {
        let (release, gate) = mpsc::channel::<()>();
        let worker = Latest::spawn("test-worker", move |n: u32| {
            gate.recv().ok();
            n * 10
        })
        .expect("spawn");
        worker.submit(1);
        // 1 is running (blocked on the gate) once 2 waits; 3 replaces 2.
        while !worker.shared.lock().running {
            std::thread::yield_now();
        }
        worker.submit(2);
        worker.submit(3);
        assert!(!worker.drain(Duration::from_millis(10)), "still blocked");
        release.send(()).expect("release 1");
        release.send(()).expect("release 3");
        assert!(worker.drain(Duration::from_secs(5)));
        assert_eq!(worker.take(), Some(30), "the newest result");
        assert_eq!(worker.take(), None, "taken once");
    }

    #[test]
    fn drop_serves_the_waiting_request_first() {
        let (sender, results) = mpsc::channel();
        let worker = Latest::spawn("test-worker", move |n: u32| {
            sender.send(n).ok();
        })
        .expect("spawn");
        worker.submit(7);
        drop(worker);
        assert_eq!(results.recv_timeout(Duration::from_secs(5)), Ok(7));
    }
}
