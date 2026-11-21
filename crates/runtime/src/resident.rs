//! The process's resident memory as the system counts it: `VmRSS` of `/proc/self/status` on
//! Linux and Android, what the low-memory killer weighs (the native heap, the graphics driver's
//! mappings, the GPU memory it maps); `None` where the system has no such file.

/// The process's resident bytes now, when the system says.
pub(crate) fn resident() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find(|line| line.starts_with("VmRSS:"))?;
    let kib: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    Some(kib * 1024)
}
