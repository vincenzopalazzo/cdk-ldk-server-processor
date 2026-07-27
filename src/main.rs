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

    // NB: addr must be a bare IP; the port is passed separately.
    let server_ip = "0.0.0.0";
    tracing::info!(
        "Starting CDK LDK-Server payment processor on {}:{}",
        server_ip,
        cfg.server_port
    );

    let mut server = PaymentProcessorServer::new(backend, server_ip, cfg.server_port)?;
    server.start(None).await?;

    // PaymentProcessorServer::start spawns the tonic server in a background
    // task and never surfaces bind errors, so a port conflict would silently
    // leave clients talking to whatever else listens on the port. Verify the
    // service actually answers before declaring startup successful.
    self_check(cfg.server_port).await?;

    match shutdown_signal().await {
        Ok(_) => tracing::info!("Shutdown signal received, stopping server..."),
        Err(e) => tracing::error!("Error waiting for shutdown signal: {}", e),
    }

    server.stop().await?;
    tracing::info!("Server stopped gracefully");
    Ok(())
}

/// Call our own `GetSettings` over loopback to prove the gRPC service is the
/// one answering on `port`. Retries while the spawned server task binds.
async fn self_check(port: u16) -> Result<()> {
    use cdk_common::payment::MintPayment;

    let mut last_err: Option<anyhow::Error> = None;
    for attempt in 1..=10u32 {
        let result = match tokio::time::timeout(
            std::time::Duration::from_secs(2),
            cdk_payment_processor::PaymentProcessorClient::new("127.0.0.1", port, None),
        )
        .await
        {
            Ok(Ok(client)) => match tokio::time::timeout(
                std::time::Duration::from_secs(2),
                client.get_settings(),
            )
            .await
            {
                Ok(Ok(settings)) => Ok(settings),
                Ok(Err(e)) => Err(anyhow::anyhow!(e.to_string())),
                Err(_) => Err(anyhow::anyhow!("get_settings timed out")),
            },
            Ok(Err(e)) => Err(e),
            Err(_) => Err(anyhow::anyhow!("connect timed out")),
        };
        match result {
            Ok(settings) => {
                tracing::info!(
                    "Self-check OK on port {}: unit={} bolt11={} bolt12={}",
                    port,
                    settings.unit,
                    settings.bolt11.is_some(),
                    settings.bolt12.is_some()
                );
                return Ok(());
            }
            Err(e) => {
                tracing::warn!("Self-check attempt {}/10 failed: {}", attempt, e);
                last_err = Some(e);
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
        }
    }
    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("no attempts made")))
        .context("startup self-check failed: is another service already listening on this port?")
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
