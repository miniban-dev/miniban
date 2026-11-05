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
    use axum::http::{Request, StatusCode, header};
    use axum::response::Response;
    use serde_json::{Value, json};
    use tower::ServiceExt;

    use super::router;

    /// Sends a GET request for `path` to the router without opening a socket.
    async fn get(path: &str) -> Response {
        let request = Request::get(path).body(Body::empty()).unwrap();
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
}
