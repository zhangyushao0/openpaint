//! adb, run with explicit arguments and never through a host shell (no path rewriting by Git
//! Bash or anything else), and the typed errors of the device commands.

use std::fmt;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::android::sdk_dir;

/// The app on the tablet.
pub const PACKAGE: &str = "dev.openpaint";

/// Why a device command failed.
#[derive(Debug)]
pub enum DeviceError {
    /// No Android SDK, so no adb.
    NoAdb(String),
    /// adb sees no device ready for commands; what `get-state` said.
    NoDevice(String),
    /// A program could not be started.
    Start { command: String, error: String },
    /// A program ran and failed.
    Failed { command: String, output: String },
    /// A local file could not be read or written.
    File { path: PathBuf, error: String },
    /// A backup would replace one already there.
    Exists(PathBuf),
    /// What should be a tar archive is not one.
    Tar(String),
    /// The library read back differs from the backup.
    Differs(Vec<String>),
    /// The app panicked or crashed; its log lines.
    Panic(Vec<String>),
    /// The gesture instrumentation did not inject the gesture.
    Injection(String),
    /// The app's process (these pids) was still running after a force-stop.
    StillRunning(String),
    /// The command line asks for something else.
    Usage(&'static str),
}

impl fmt::Display for DeviceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoAdb(why) => write!(f, "no adb: {why}"),
            Self::NoDevice(state) => write!(f, "no device ready for adb (state: {state})"),
            Self::Start { command, error } => write!(f, "cannot start {command}: {error}"),
            Self::Failed { command, output } => write!(f, "{command} failed: {output}"),
            Self::File { path, error } => write!(f, "{}: {error}", path.display()),
            Self::Exists(path) => write!(f, "{} exists; choose another name", path.display()),
            Self::Tar(why) => write!(f, "not a tar archive: {why}"),
            Self::Differs(entries) => {
                write!(f, "the library differs from the backup:")?;
                entries
                    .iter()
                    .try_for_each(|entry| write!(f, "\n  {entry}"))
            }
            Self::Panic(lines) => {
                write!(f, "the app panicked or crashed:")?;
                lines.iter().try_for_each(|line| write!(f, "\n  {line}"))
            }
            Self::Injection(output) => write!(f, "the gesture was not injected: {output}"),
            Self::StillRunning(pids) => {
                write!(f, "{PACKAGE} still runs (pid {pids}) after am force-stop")
            }
            Self::Usage(usage) => write!(f, "usage: {usage}"),
        }
    }
}

pub type Result<T> = std::result::Result<T, DeviceError>;

/// The SDK's adb.
pub struct Adb {
    program: PathBuf,
}

impl Adb {
    /// adb from the Android SDK, with a device ready for commands.
    pub fn connected() -> Result<Self> {
        let sdk = sdk_dir().map_err(DeviceError::NoAdb)?;
        let program = sdk
            .join("platform-tools")
            .join(format!("adb{}", std::env::consts::EXE_SUFFIX));
        let adb = Self { program };
        let state = adb.text(&["get-state"]).map_err(|error| match error {
            DeviceError::Failed { output, .. } => DeviceError::NoDevice(output),
            other => other,
        })?;
        if state.trim() != "device" {
            return Err(DeviceError::NoDevice(state.trim().to_owned()));
        }
        Ok(adb)
    }

    /// Runs adb with `args`; its standard output as bytes.
    pub fn bytes(&self, args: &[&str]) -> Result<Vec<u8>> {
        self.run(args, None)
    }

    /// Runs adb with `args`; its standard output as text.
    pub fn text(&self, args: &[&str]) -> Result<String> {
        Ok(String::from_utf8_lossy(&self.bytes(args)?).into_owned())
    }

    /// Runs adb with `args`, `input` on its standard input.
    pub fn feed(&self, args: &[&str], input: &[u8]) -> Result<String> {
        Ok(String::from_utf8_lossy(&self.run(args, Some(input))?).into_owned())
    }

    /// One command in the device's shell, its words as given.
    pub fn shell(&self, words: &[&str]) -> Result<String> {
        let mut args = vec!["shell"];
        args.extend_from_slice(words);
        self.text(&args)
    }

    fn run(&self, args: &[&str], input: Option<&[u8]>) -> Result<Vec<u8>> {
        let command = format!("adb {}", args.join(" "));
        let mut child = Command::new(&self.program)
            .args(args)
            .stdin(if input.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| DeviceError::Start {
                command: command.clone(),
                error: error.to_string(),
            })?;
        if let (Some(input), Some(mut stdin)) = (input, child.stdin.take()) {
            stdin
                .write_all(input)
                .map_err(|error| DeviceError::Failed {
                    command: command.clone(),
                    output: format!("writing its input: {error}"),
                })?;
        }
        let output = child
            .wait_with_output()
            .map_err(|error| DeviceError::Start {
                command: command.clone(),
                error: error.to_string(),
            })?;
        if output.status.success() {
            return Ok(output.stdout);
        }
        let mut said = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        if said.is_empty() {
            said = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        }
        Err(DeviceError::Failed {
            command,
            output: said,
        })
    }
}

/// Writes `bytes` to `path`, making its directory.
pub fn write_file(path: &Path, bytes: &[u8]) -> Result<()> {
    let error = |error: std::io::Error| DeviceError::File {
        path: path.to_path_buf(),
        error: error.to_string(),
    };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(error)?;
    }
    std::fs::write(path, bytes).map_err(error)
}

/// Reads the file at `path`.
pub fn read_file(path: &Path) -> Result<Vec<u8>> {
    std::fs::read(path).map_err(|error| DeviceError::File {
        path: path.to_path_buf(),
        error: error.to_string(),
    })
}
