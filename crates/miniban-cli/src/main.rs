// Copyright (c) 2026 Braden Hitchcock - AGPL-3.0-only (see LICENSE for details)

//! The `mb` executable, the command-line interface to Miniban.

use clap::Parser;
use miniban_core::PRODUCT_NAME;

/// Command-line arguments for `mb`.
#[derive(Parser)]
#[command(name = "mb", version, about = format!("{PRODUCT_NAME} command-line interface"))]
struct Cli {}

fn main() {
    Cli::parse();
}
