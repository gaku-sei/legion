//! Dumb binary to test async span fn

#![allow(clippy::never_loop)]

use std::time::Duration;

use lgn_telemetry_sink::TelemetryGuard;
use lgn_tracing::span_fn;
use tokio::time::sleep;

#[span_fn]
async fn empty_return() {
    sleep(Duration::from_millis(1)).await;

    return;
}

#[span_fn]
async fn iteration_with_cond() {
    let a = 3;

    loop {
        if a == 3 {
            println!("a was 3");
            sleep(Duration::from_millis(1)).await;
        }

        break;
    }
}

#[span_fn]
async fn delayed_value() -> String {
    sleep(Duration::from_millis(1)).await;

    "After".into()
}

#[span_fn]
fn consume_delayed_value(_: String) {
    println!("Consumed a delayed value");
}

#[span_fn]
async fn delayed() {
    println!("Before");

    sleep(Duration::from_millis(1)).await;

    println!("Second");

    let msg = delayed_value().await;

    println!("{}", msg);

    consume_delayed_value(delayed_value().await);

    return;
}

#[tokio::main]
async fn main() {
    let _telemetry_guard = TelemetryGuard::default().unwrap();

    delayed().await;

    iteration_with_cond().await;

    empty_return().await;
}
