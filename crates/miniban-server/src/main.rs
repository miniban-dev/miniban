// Copyright (c) 2026 Braden Hitchcock - AGPL-3.0-only (see LICENSE for details)

//! The `miniban` executable, the Miniban HTTP server.
//!
//! Configures logging, binds the address from [`config::Config`], and serves [`app::router`] until a
//! termination signal arrives.

mod app;
mod config;
#[cfg(not(feature = "headless"))]
mod frontend;

use std::error::Error;
use std::process::ExitCode;

use miniban_core::PRODUCT_NAME;
use tokio::net::TcpListener;
use tokio::signal;
use tracing::level_filters::LevelFilter;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

use crate::config::Config;

#[tokio::main]
async fn main() -> ExitCode {
    init_logging();

    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            error!(%error, "server failed");
            ExitCode::FAILURE
        }
    }
}

/// Sends log messages to stdout, filtered by `RUST_LOG` and defaulting to `info`.
fn init_logging() {
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();
    tracing_subscriber::fmt().with_env_filter(filter).init();
}

/// Serves the application until a termination signal arrives.
async fn run() -> Result<(), Box<dyn Error>> {
    let Config { host, port } = Config::from_env()?;
    info!(%host, port, "starting {PRODUCT_NAME} server");
    #[cfg(feature = "headless")]
    tracing::warn!("this is a headless build: the frontend isn't included");

    let listener = TcpListener::bind((host.as_str(), port)).await?;
    info!(address = %listener.local_addr()?, "listening");

    axum::serve(listener, app::router())
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    info!("server stopped");
    Ok(())
}

/// Completes when the process receives SIGINT or, on Unix, SIGTERM.
async fn shutdown_signal() {
    let interrupt = async {
        signal::ctrl_c()
            .await
            .expect("the SIGINT handler should install");
        "SIGINT"
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("the SIGTERM handler should install")
            .recv()
            .await;
        "SIGTERM"
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending();

    let signal = tokio::select! {
        signal = interrupt => signal,
        signal = terminate => signal,
    };
    info!(signal, "shutting down");
}
