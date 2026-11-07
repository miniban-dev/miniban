// Copyright (c) 2026 Braden Hitchcock - AGPL-3.0-only (see LICENSE for details)

//! GitHub issue provider for Miniban.
//!
//! [`provider::display_name`] names this provider for users.

/// The identity of this provider.
pub mod provider {
    use miniban_core::PRODUCT_NAME;

    /// Returns the name of this provider as shown to users.
    pub fn display_name() -> String {
        format!("{PRODUCT_NAME} GitHub provider")
    }
}
