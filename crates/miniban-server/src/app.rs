// Copyright (c) 2026 Braden Hitchcock - AGPL-3.0-only (see LICENSE for details)

//! The application router, which defines the HTTP contract. Build it with [`router`].

use axum::http::StatusCode;
use axum::routing::{any, get};
use axum::{Json, Router};
use serde_json::{Value, json};

/// Creates the application router.
pub fn router() -> Router {
    let api = Router::new()
        .route("/health", get(health))
        .fallback(api_not_found);
    // The nested catch-all doesn't match an empty remainder, so `/api/` needs its own route.
    let app = Router::new()
        .nest("/api", api)
        .route("/api/", any(api_not_found));

    #[cfg(feature = "headless")]
    let app = app.fallback(frontend_not_included);
    #[cfg(not(feature = "headless"))]
    let app = app.fallback_service(get(crate::frontend::serve));

    app
}

/// Reports that the server is alive.
async fn health() -> Json<Value> {
    Json(json!({"status": "ok"}))
}

/// Answers API routes that don't exist.
async fn api_not_found() -> (StatusCode, Json<Value>) {
    (StatusCode::NOT_FOUND, Json(json!({"error": "not found"})))
}

/// Answers non-API routes in a headless build, which has no frontend to serve.
#[cfg(feature = "headless")]
async fn frontend_not_included() -> (StatusCode, &'static str) {
    (
        StatusCode::NOT_FOUND,
        "This is a headless build: the frontend isn't included.",
    )
}

// -------------------------------------------------------------------------------------------------
// END OF BUSINESS LOGIC - TESTS BELOW HERE
// -------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use axum::body::{Body, to_bytes};
    use axum::http::{Method, Request, StatusCode, header};
    use axum::response::Response;
    use serde_json::{Value, json};
    use tower::ServiceExt;

    use super::router;

    /// Sends a GET request for `path` to the router without opening a socket.
    async fn get(path: &str) -> Response {
        send(Method::GET, path).await
    }

    /// Sends a `method` request for `path` to the router without opening a socket.
    async fn send(method: Method, path: &str) -> Response {
        let request = Request::builder()
            .method(method)
            .uri(path)
            .body(Body::empty())
            .unwrap();
        router().oneshot(request).await.unwrap()
    }

    /// Asserts that `response` declares a JSON body, and returns it parsed.
    async fn json_body(response: Response) -> Value {
        let content_type = response.headers().get(header::CONTENT_TYPE).unwrap();
        assert_eq!(content_type, "application/json");
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn health_reports_ok() {
        let response = get("/api/health").await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(json_body(response).await, json!({"status": "ok"}));
    }

    #[tokio::test]
    async fn unknown_api_routes_return_a_json_404() {
        for path in ["/api", "/api/", "/api/unknown", "/api/health/unknown"] {
            let response = get(path).await;

            assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
            assert_eq!(
                json_body(response).await,
                json!({"error": "not found"}),
                "{path}"
            );
        }
    }

    #[cfg(feature = "headless")]
    #[tokio::test]
    async fn headless_non_api_routes_say_the_frontend_is_not_included() {
        for path in ["/", "/apiary", "/board/1"] {
            let response = get(path).await;

            assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
            let content_type = response.headers().get(header::CONTENT_TYPE).unwrap();
            assert!(
                content_type.to_str().unwrap().starts_with("text/plain"),
                "{path}"
            );
            let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
            let body = String::from_utf8(bytes.to_vec()).unwrap();
            assert_eq!(
                body, "This is a headless build: the frontend isn't included.",
                "{path}"
            );
        }
    }

    /// Router tests for the embedded frontend, with the build output on disk as the expected value.
    #[cfg(not(feature = "headless"))]
    mod embedded {
        use std::path::{Path, PathBuf};

        use super::*;

        /// Reads a file from the frontend build output, given its path relative to the output root.
        fn built_file(path: &str) -> Vec<u8> {
            std::fs::read(build_dir().join(path)).unwrap()
        }

        /// Returns the frontend build output directory.
        fn build_dir() -> PathBuf {
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../frontend/build")
        }

        /// Lists every file in the frontend build output, relative to the output root.
        fn built_paths() -> Vec<String> {
            let root = build_dir();
            let mut directories = vec![root.clone()];
            let mut paths = Vec::new();
            while let Some(directory) = directories.pop() {
                for entry in std::fs::read_dir(directory).unwrap() {
                    let path = entry.unwrap().path();
                    if path.is_dir() {
                        directories.push(path);
                    } else {
                        let relative = path.strip_prefix(&root).unwrap();
                        paths.push(relative.to_str().unwrap().to_owned());
                    }
                }
            }
            paths
        }

        #[tokio::test]
        async fn browser_side_routes_return_the_html_document() {
            for path in ["/", "/board/1", "/apiary", "/index.html"] {
                let response = get(path).await;

                assert_eq!(response.status(), StatusCode::OK, "{path}");
                let content_type = response.headers().get(header::CONTENT_TYPE).unwrap();
                assert_eq!(content_type, "text/html; charset=utf-8", "{path}");
                let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
                assert!(bytes.starts_with(b"<!DOCTYPE html>"), "{path}");
                assert_eq!(bytes.as_ref(), built_file("index.html"), "{path}");
            }
        }

        #[tokio::test]
        async fn unknown_api_routes_do_not_return_the_frontend() {
            let paths = ["/api/unknown", "/api/index.html", "/api/_app/version.json"];
            for method in [Method::GET, Method::POST] {
                for path in paths {
                    let response = send(method.clone(), path).await;

                    assert_eq!(response.status(), StatusCode::NOT_FOUND, "{method} {path}");
                    assert_eq!(json_body(response).await, json!({"error": "not found"}));
                }
            }
        }

        #[tokio::test]
        async fn head_requests_describe_the_document_without_a_body() {
            let response = send(Method::HEAD, "/board/1").await;

            assert_eq!(response.status(), StatusCode::OK);
            let content_type = response.headers().get(header::CONTENT_TYPE).unwrap();
            assert_eq!(content_type, "text/html; charset=utf-8");
            let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
            assert!(bytes.is_empty());
        }

        #[tokio::test]
        async fn other_methods_are_not_allowed_on_the_frontend() {
            for path in ["/", "/board/1", "/_app/version.json"] {
                let response = send(Method::POST, path).await;

                assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED, "{path}");
            }
        }

        #[tokio::test]
        async fn only_hashed_assets_are_cached_immutably() {
            for path in built_paths() {
                let response = get(&format!("/{path}")).await;

                let cache_control = response.headers().get(header::CACHE_CONTROL).unwrap();
                let expected = if path.starts_with("_app/immutable/") {
                    "public, max-age=31536000, immutable"
                } else {
                    "no-cache"
                };
                assert_eq!(cache_control, expected, "{path}");
            }
        }

        #[tokio::test]
        async fn browser_side_routes_are_never_cached_immutably() {
            for path in ["/", "/board/1", "/_app/immutable/missing.js"] {
                let response = get(path).await;

                let cache_control = response.headers().get(header::CACHE_CONTROL).unwrap();
                assert_eq!(cache_control, "no-cache", "{path}");
            }
        }

        #[tokio::test]
        async fn every_built_file_is_served_directly_with_a_known_mime_type() {
            let paths = built_paths();
            assert!(!paths.is_empty());

            for path in paths {
                let response = get(&format!("/{path}")).await;

                assert_eq!(response.status(), StatusCode::OK, "{path}");
                let content_type = response.headers().get(header::CONTENT_TYPE).unwrap();
                let expected = match path.rsplit_once('.').unwrap().1 {
                    "html" => "text/html; charset=utf-8",
                    "js" => "text/javascript; charset=utf-8",
                    "css" => "text/css; charset=utf-8",
                    "json" => "application/json",
                    other => panic!("add `.{other}` to the frontend MIME table and here"),
                };
                assert_eq!(content_type, expected, "{path}");
                let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
                assert_eq!(bytes.as_ref(), built_file(&path), "{path}");
            }
        }
    }
}
