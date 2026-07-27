//! Live smoke test: call get_settings on a running processor.
use cdk_common::payment::MintPayment;

#[tokio::main]
async fn main() {
    let addr = std::env::var("PROC_ADDR").unwrap_or_else(|_| "127.0.0.1".to_string());
    let client = cdk_payment_processor::PaymentProcessorClient::new(&addr, 50051, None)
        .await
        .expect("connect");
    match client.get_settings().await {
        Ok(s) => println!("GET_SETTINGS_OK unit={} bolt11={} bolt12={}", s.unit, s.bolt11.is_some(), s.bolt12.is_some()),
        Err(e) => println!("GET_SETTINGS_ERR {}", e),
    }
}
