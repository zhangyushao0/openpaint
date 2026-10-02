//! Repository tooling: `cargo gate` and `cargo repo-check`. Tooling only, never product code.
#![forbid(unsafe_code)]

mod check;
mod gate;
mod process;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("gate") => gate::run(),
        Some("check") => check::run(),
        _ => {
            eprintln!("usage: cargo xtask <gate|check>");
            std::process::exit(2);
        }
    };
    if let Err(message) = result {
        eprintln!("{message}");
        std::process::exit(1);
    }
}
