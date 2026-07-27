//! Configuration via environment variables.

use cdk_common::amount::Amount;
use cdk_common::common::FeeReserve;

/// Processor configuration, populated from environment variables.
#[derive(Debug, Clone)]
pub struct Config {
    /// Port the payment processor gRPC server listens on.
    pub server_port: u16,
    /// ldk-server gRPC address without scheme, e.g. `127.0.0.1:3536`.
    pub ldk_server_addr: String,
    /// HMAC API key expected by ldk-server.
    pub ldk_server_api_key: String,
    /// Path to the PEM-encoded TLS certificate to pin for ldk-server.
    pub ldk_server_tls_cert: String,
    /// Fee reserve: absolute minimum (sats).
    pub fee_reserve_min_sat: u64,
    /// Fee reserve: relative share (e.g. 0.01 = 1%).
    pub fee_reserve_percent: f32,
    /// Maximum `ListPayments` pages to scan for incoming status lookups.
    pub max_payment_scan_pages: u16,
}

impl Config {
    /// Read configuration from environment variables.
    ///
    /// Required: `LDK_SERVER_ADDR`, `LDK_SERVER_API_KEY`, `LDK_SERVER_TLS_CERT`.
    /// Optional: `SERVER_PORT` (50071), `FEE_RESERVE_MIN_SAT` (2),
    /// `FEE_RESERVE_PERCENT` (0.01), `MAX_PAYMENT_SCAN_PAGES` (32).
    ///
    /// NB: the default port is 50071, not the CDK-customary 50051, because
    /// 50051 is a popular gRPC port and a silent bind conflict there is
    /// hard to diagnose (cdk-payment-processor swallows bind errors).
    pub fn from_env() -> Self {
        Self {
            server_port: std::env::var("SERVER_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(50071),
            ldk_server_addr: std::env::var("LDK_SERVER_ADDR")
                .expect("LDK_SERVER_ADDR is required (e.g. 127.0.0.1:3536)"),
            ldk_server_api_key: std::env::var("LDK_SERVER_API_KEY")
                .expect("LDK_SERVER_API_KEY is required"),
            ldk_server_tls_cert: std::env::var("LDK_SERVER_TLS_CERT")
                .expect("LDK_SERVER_TLS_CERT is required (path to PEM cert)"),
            fee_reserve_min_sat: std::env::var("FEE_RESERVE_MIN_SAT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(2),
            fee_reserve_percent: std::env::var("FEE_RESERVE_PERCENT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.01),
            max_payment_scan_pages: std::env::var("MAX_PAYMENT_SCAN_PAGES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(32),
        }
    }

    /// Build the cdk-common `FeeReserve` for melt quotes.
    pub fn fee_reserve(&self) -> FeeReserve {
        FeeReserve {
            min_fee_reserve: Amount::from(self.fee_reserve_min_sat),
            percent_fee_reserve: self.fee_reserve_percent,
        }
    }
}
