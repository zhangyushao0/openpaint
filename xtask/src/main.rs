//! Repository tooling: `cargo gate`, `cargo repo-check`, the Android build and the device
//! chores. Tooling only, never product code.
#![forbid(unsafe_code)]

mod android;
mod check;
mod device;
mod gate;
mod process;

const USAGE: &str = "usage: cargo xtask <gate|check|android-build|android-install|device-backup|\
                     device-restore|device-check|device-input|device-clean|device-push> [args]";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let rest = args.get(1..).unwrap_or_default();
    let device = |result: Result<(), device::DeviceError>| result.map_err(|e| e.to_string());
    let result = match args.first().map(String::as_str) {
        Some("gate") => gate::run(),
        Some("check") => check::run(),
        Some("android-build") => android::build(),
        Some("android-install") => android::install(),
        Some("device-backup") => device(device::backup(rest)),
        Some("device-restore") => device(device::restore(rest)),
        Some("device-check") => device(device::check(rest)),
        Some("device-input") => device(device::input(rest)),
        Some("device-clean") => device(device::clean(rest)),
        Some("device-push") => device(device::push(rest)),
        _ => {
            eprintln!("{USAGE}");
            std::process::exit(2);
        }
    };
    if let Err(message) = result {
        eprintln!("{message}");
        std::process::exit(1);
    }
}
