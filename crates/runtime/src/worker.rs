//! `Queue`: one background thread serving requests in the order they were submitted, each
//! result kept until the frame loop takes it. Dropping it serves every request still queued,
//! then joins the thread, so nothing handed over is lost on shutdown.
//!
//! This module is the only place product code starts a thread (ARCHITECTURE section 3).

use std::collections::VecDeque;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::error::RuntimeError;

pub(crate) struct Queue<Req, Out> {
    /// Dropped first on shutdown, which ends the thread's loop after the queued requests.
    requests: Option<Sender<Req>>,
    results: Receiver<Out>,
    /// Results received while draining, handed out by `take` first.
    ready: VecDeque<Out>,
    /// Requests submitted whose result has not been received.
    outstanding: usize,
    thread: Option<JoinHandle<()>>,
}

impl<Req: Send + 'static, Out: Send + 'static> Queue<Req, Out> {
    /// Starts the thread `name`, which runs `job` for each request.
    pub(crate) fn spawn(
        name: &'static str,
        mut job: impl FnMut(Req) -> Out + Send + 'static,
    ) -> Result<Self, RuntimeError> {
        let (requests, inbox) = mpsc::channel::<Req>();
        let (outbox, results) = mpsc::channel();
        let thread = std::thread::Builder::new()
            .name(name.to_owned())
            .spawn(move || {
                for request in inbox {
                    // The receiver goes only after the thread is joined.
                    let _ = outbox.send(job(request));
                }
            })
            .map_err(|_| RuntimeError::Worker(name))?;
        Ok(Self {
            requests: Some(requests),
            results,
            ready: VecDeque::new(),
            outstanding: 0,
            thread: Some(thread),
        })
    }
}

impl<Req, Out> Queue<Req, Out> {
    /// Queues `request` after the ones submitted before it.
    pub(crate) fn submit(&mut self, request: Req) {
        let sent = self
            .requests
            .as_ref()
            .is_some_and(|requests| requests.send(request).is_ok());
        if sent {
            self.outstanding += 1;
        } else {
            log::error!("a worker thread stopped; a request was dropped");
        }
    }

    /// The oldest result not taken yet.
    pub(crate) fn take(&mut self) -> Option<Out> {
        if let Some(out) = self.ready.pop_front() {
            return Some(out);
        }
        let out = self.results.try_recv().ok()?;
        self.outstanding -= 1;
        Some(out)
    }

    /// True while a request is queued or running, or a result waits to be taken.
    pub(crate) fn busy(&self) -> bool {
        self.outstanding > 0 || !self.ready.is_empty()
    }

    /// Waits until every submitted request has run, at most `limit`. Returns whether they all
    /// did; the results stay for `take`.
    pub(crate) fn drain(&mut self, limit: Duration) -> bool {
        let deadline = Instant::now() + limit;
        while self.outstanding > 0 {
            let left = deadline.saturating_duration_since(Instant::now());
            match self.results.recv_timeout(left) {
                Ok(out) => {
                    self.outstanding -= 1;
                    self.ready.push_back(out);
                }
                Err(RecvTimeoutError::Timeout) => return false,
                Err(RecvTimeoutError::Disconnected) => {
                    log::error!("a worker thread stopped with requests left");
                    self.outstanding = 0;
                }
            }
        }
        true
    }
}

impl<Req, Out> Drop for Queue<Req, Out> {
    fn drop(&mut self) {
        self.requests = None;
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            log::error!("a worker thread panicked");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn results_come_back_in_order_and_drain_waits_for_all() {
        let (release, gate) = mpsc::channel::<()>();
        let mut worker = Queue::spawn("test-worker", move |n: u32| {
            gate.recv().ok();
            n * 10
        })
        .expect("spawn");
        for n in 1..=3 {
            worker.submit(n);
        }
        assert!(worker.busy());
        assert!(!worker.drain(Duration::from_millis(10)), "still blocked");
        for _ in 0..3 {
            release.send(()).expect("release");
        }
        assert!(worker.drain(Duration::from_secs(5)));
        let results: Vec<u32> = std::iter::from_fn(|| worker.take()).collect();
        assert_eq!(results, [10, 20, 30]);
        assert!(!worker.busy());
    }

    #[test]
    fn drop_serves_the_queued_requests_first() {
        let (sender, results) = mpsc::channel();
        let mut worker = Queue::spawn("test-worker", move |n: u32| {
            sender.send(n).ok();
        })
        .expect("spawn");
        worker.submit(7);
        worker.submit(8);
        drop(worker);
        let served: Vec<u32> = results.try_iter().collect();
        assert_eq!(served, [7, 8]);
    }
}
