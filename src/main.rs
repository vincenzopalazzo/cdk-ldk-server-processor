//! CDK gRPC payment processor backed by an external ldk-server daemon.
//!
//! Wraps the `cdk-ldk-server` `MintPayment` implementation in the CDK
//! payment processor gRPC protocol so a stock `cdk-mintd` (feature
//! `grpc-processor`) can use ldk-server without any fork or git dependency.

mod settings;

use std::sync::Arc;

use anyhow::{Context, Result};
use cdk_ldk_server::{CdkLdkServer, Config as LdkServerConfig};
use cdk_payment_processor::PaymentProcessorServer;
use tokio::signal;

use self::settings::Config;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("info".parse().expect("valid log directive")),
        )
        .init();

    let cfg = Config::from_env();

    let cert_pem = std::fs::read(&cfg.ldk_server_tls_cert).context("reading ldk-server TLS cert")?;

    let ldk_cfg = LdkServerConfig::new(
        cfg.ldk_server_addr.clone(),
        cfg.ldk_server_api_key.clone(),
        cert_pem,
        cfg.fee_reserve(),
    )
    .with_max_payment_scan_pages(cfg.max_payment_scan_pages);

    let backend = Arc::new(
        CdkLdkServer::new(ldk_cfg).context("initializing ldk-server backend")?,
    );

    let server_addr = format!("0.0.0.0:{}", cfg.server_port);
    tracing::info!("Starting CDK LDK-Server payment processor on {}", server_addr);

    let mut server = PaymentProcessorServer::new(backend, &server_addr, cfg.server_port)?;
    server.start(None).await?;

    match shutdown_signal().await {
        Ok(_) => tracing::info!("Shutdown signal received, stopping server..."),
        Err(e) => tracing::error!("Error waiting for shutdown signal: {}", e),
    }

    server.stop().await?;
    tracing::info!("Server stopped gracefully");
    Ok(())
}

/// Wait for shutdown signal (SIGTERM or SIGINT).
async fn shutdown_signal() -> Result<()> {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    Ok(())
}
