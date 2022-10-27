use clap::Parser;
use std::process;

use kdump::{run, CLIConfig};

fn main() {
    let config = CLIConfig::parse();

    if let Err(e) = run(&config) {
        eprintln!("Application error: {}", e);

        process::exit(1);
    }
}
