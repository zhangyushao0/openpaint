//! Repository tooling: `cargo gate`, `cargo repo-check` and the Android build. Tooling only,
//! never product code.
#![forbid(unsafe_code)]

mod android;
mod check;
mod gate;
mod process;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("gate") => gate::run(),
        Some("check") => check::run(),
        Some("android-build") => android::build(),
        Some("android-install") => android::install(),
        _ => {
            eprintln!("usage: cargo xtask <gate|check|android-build|android-install>");
            std::process::exit(2);
        }
    };
    if let Err(message) = result {
        eprintln!("{message}");
        std::process::exit(1);
    }
}
