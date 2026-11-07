// Copyright (c) 2026 Braden Hitchcock - AGPL-3.0-only (see LICENSE for details)

//! The frontend embedded in the executable. Serve it with [`serve`].

use axum::http::{Uri, header};
use axum::response::{IntoResponse, Response};
use rust_embed::RustEmbed;

/// The name of the document returned for browser-side routes.
const FALLBACK_DOCUMENT: &str = "index.html";

/// The directory of the build output whose file names contain a hash of their content.
const HASHED_ASSETS_PREFIX: &str = "_app/immutable/";

/// The frontend build output, embedded at compile time in every profile.
#[derive(RustEmbed)]
#[folder = "../../frontend/build/"]
struct Assets;

/// Serves the embedded file at the request's path, or the fallback document if there is none.
pub async fn serve(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let (path, file) = match Assets::get(path) {
        Some(file) => (path, file),
        None => (
            FALLBACK_DOCUMENT,
            Assets::get(FALLBACK_DOCUMENT).expect("the build script should check for it"),
        ),
    };
    let headers = [
        (header::CONTENT_TYPE, content_type(path)),
        (header::CACHE_CONTROL, cache_control(path)),
    ];
    (headers, file.data).into_response()
}

/// Returns the caching policy for a file: only content-hashed assets may be cached for good.
fn cache_control(path: &str) -> &'static str {
    if path.starts_with(HASHED_ASSETS_PREFIX) {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    }
}

/// Returns the MIME type for the file types the frontend build emits, by the path's extension.
///
/// Other types get a generic binary type, which browsers won't interpret as a document or script.
fn content_type(path: &str) -> &'static str {
    match path.rsplit_once('.') {
        Some((_, "html")) => "text/html; charset=utf-8",
        Some((_, "js")) => "text/javascript; charset=utf-8",
        Some((_, "css")) => "text/css; charset=utf-8",
        Some((_, "json")) => "application/json",
        _ => "application/octet-stream",
    }
}
