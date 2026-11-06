//! The hang guard (S106b): a command still running after a limit is killed, its output so far
//! kept. A test binary that deadlocks fails the gate instead of holding it (the store's lib tests
//! held one for nine hours on 2026-11-06).

use std::io::{self, ErrorKind, Read};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex, PoisonError, mpsc};
use std::thread;
use std::time::{Duration, Instant};

/// How often a running child is asked whether it has ended.
const POLL: Duration = Duration::from_millis(10);

/// How long a killed child's output may still take to arrive: a grandchild that inherited the
/// pipes (cargo's rustdoc under the doc tests) can hold them open after the child is gone.
const DRAIN: Duration = Duration::from_secs(5);

/// How a guarded command ended.
pub struct Run {
    /// Its exit status; `None` when it was killed at the limit.
    pub status: Option<ExitStatus>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub elapsed: Duration,
}

impl Run {
    pub fn passed(&self) -> bool {
        self.status.is_some_and(|status| status.success())
    }
}

/// Runs `command` as `Command::output` does (no input, both outputs captured), killing it if it
/// is still running after `limit`.
pub fn run(command: &mut Command, limit: Duration) -> io::Result<Run> {
    let started = Instant::now();
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let stdout = Pipe::read(child.stdout.take());
    let stderr = Pipe::read(child.stderr.take());
    let status = wait(&mut child, started + limit)?;
    let elapsed = started.elapsed();
    Ok(Run {
        status,
        stdout: stdout.taken(),
        stderr: stderr.taken(),
        elapsed,
    })
}

/// The child's status once it ends; `None` after killing it at `deadline`.
fn wait(child: &mut Child, deadline: Instant) -> io::Result<Option<ExitStatus>> {
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(Some(status));
        }
        if Instant::now() >= deadline {
            child.kill()?;
            child.wait()?;
            return Ok(None);
        }
        thread::sleep(POLL);
    }
}

/// One of the child's output pipes, read on a thread of its own into a buffer taken when the
/// child has ended.
struct Pipe {
    bytes: Arc<Mutex<Vec<u8>>>,
    /// Disconnected when the pipe has closed.
    closed: mpsc::Receiver<()>,
}

impl Pipe {
    fn read(pipe: Option<impl Read + Send + 'static>) -> Pipe {
        let bytes = Arc::new(Mutex::new(Vec::new()));
        let (open, closed) = mpsc::channel();
        if let Some(mut pipe) = pipe {
            let bytes = Arc::clone(&bytes);
            thread::spawn(move || {
                let _open = open;
                let mut chunk = [0; 8192];
                loop {
                    match pipe.read(&mut chunk) {
                        Ok(0) => break,
                        Ok(n) => lock(&bytes).extend_from_slice(&chunk[..n]),
                        Err(error) if error.kind() == ErrorKind::Interrupted => {}
                        Err(_) => break,
                    }
                }
            });
        }
        Pipe { bytes, closed }
    }

    /// Everything once the pipe has closed, or what came before `DRAIN` ran out.
    fn taken(self) -> Vec<u8> {
        // Only a disconnect or the timeout can end the wait: nothing is ever sent.
        let _closed_or_drained = self.closed.recv_timeout(DRAIN);
        std::mem::take(&mut *lock(&self.bytes))
    }
}

fn lock(bytes: &Mutex<Vec<u8>>) -> std::sync::MutexGuard<'_, Vec<u8>> {
    bytes.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The child the guard's test starts: this test binary running only this test, which says
    /// what libtest says of a test running past a minute, then sleeps far past the limit.
    #[test]
    #[ignore = "a child process of a_command_running_past_its_limit_is_killed_with_its_output"]
    fn sleeper() {
        println!("test test_run::guard::tests::sleeper has been running for over 60 seconds");
        thread::sleep(Duration::from_secs(120));
    }

    #[test]
    fn a_command_running_past_its_limit_is_killed_with_its_output() {
        let exe = std::env::current_exe().expect("the test binary's path");
        let args = [
            "--ignored",
            "--exact",
            "test_run::guard::tests::sleeper",
            "--nocapture",
        ];
        // Long enough for the sleeper to start and print beside a whole gate.
        let limit = Duration::from_secs(5);
        let run = run(Command::new(&exe).args(args), limit).expect("the sleeper starts");
        assert!(run.status.is_none(), "killed, not exited: {:?}", run.status);
        assert!(run.elapsed >= limit, "{:?}", run.elapsed);
        assert!(
            run.elapsed < limit + Duration::from_secs(10),
            "{:?}",
            run.elapsed
        );
        assert!(!run.passed());
        let stdout = String::from_utf8_lossy(&run.stdout);
        assert_eq!(
            crate::test_run::hung(&stdout),
            "still running: \"test test_run::guard::tests::sleeper has been running for over 60 \
             seconds\"",
            "{stdout}"
        );
    }

    #[test]
    fn a_command_ending_in_time_keeps_its_status_and_output() {
        let exe = std::env::current_exe().expect("the test binary's path");
        let args = ["--list"];
        let run = run(Command::new(&exe).args(args), Duration::from_secs(60)).expect("it runs");
        assert!(run.passed(), "{:?}", run.status);
        let stdout = String::from_utf8_lossy(&run.stdout);
        assert!(stdout.contains("guard::tests::sleeper: test"), "{stdout}");
    }
}
