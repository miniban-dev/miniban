// Copyright (c) 2026 Braden Hitchcock - AGPL-3.0-only (see LICENSE for details)

//! Fails the build when the frontend to embed is missing, unless the `headless` feature compiles
//! embedding out. It only checks for the frontend: building it is an explicit `mise` step.

use std::path::Path;

/// The frontend build output, relative to this package.
const BUILD_DIR: &str = "../../frontend/build";

/// The document the server can't run without, relative to [`BUILD_DIR`].
const FALLBACK_DOCUMENT: &str = "index.html";

fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    if cfg!(feature = "headless") {
        return;
    }

    // The embedding macro can't see files added to the directory, so this recompiles the package.
    println!("cargo::rerun-if-changed={BUILD_DIR}");
    if !Path::new(BUILD_DIR).join(FALLBACK_DOCUMENT).is_file() {
        println!(
            "cargo::error=the frontend assets to embed are missing from frontend/build. \
             Run `mise run build:front`, or enable the `headless` feature to build without them."
        );
    }
}
