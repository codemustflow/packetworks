mod env;

use crate::env::load_environment;
use anyhow::{Context, Result};
use std::net::SocketAddr;
use tokio::net::{UdpSocket, lookup_host};
use tokio::time::{Duration, interval};
use tracing::info;

const PAYLOAD: &[u8] = b"packetworks-source";
const SEND_INTERVAL: Duration = Duration::from_secs(1);

#[tokio::main]
async fn main() -> Result<()> {
    let environment = load_environment().context("failed to load environment")?;
    let _logging =
        logging::init_logging("source", environment.log_level, environment.log_for_humans)
            .context("failed to initialize logging")?;

    info!(?environment, "environment loaded");

    let socket = UdpSocket::bind((environment.bind_address.as_str(), environment.bind_port))
        .await
        .with_context(|| {
            format!(
                "failed to bind UDP socket to {}:{}",
                environment.bind_address, environment.bind_port
            )
        })?;
    let local_addr = socket
        .local_addr()
        .context("failed to read bound UDP socket address")?;
    let peer = lookup_host((environment.peer_address.as_str(), environment.peer_port))
        .await
        .with_context(|| {
            format!(
                "failed to resolve peer {}:{}",
                environment.peer_address, environment.peer_port
            )
        })?
        .next()
        .with_context(|| {
            format!(
                "peer {}:{} resolved to no addresses",
                environment.peer_address, environment.peer_port
            )
        })?;

    info!(
        local_addr = %local_addr,
        configured_bind_address = %environment.bind_address,
        configured_bind_port = environment.bind_port,
        peer = %peer,
        "source UDP socket bound"
    );

    run_source(socket, peer).await
}

async fn run_source(socket: UdpSocket, peer: SocketAddr) -> Result<()> {
    let mut packets_sent = 0_u64;
    let mut ticker = interval(SEND_INTERVAL);

    info!(
        peer = %peer,
        payload_size = PAYLOAD.len(),
        interval_seconds = SEND_INTERVAL.as_secs(),
        "source running"
    );

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                info!(
                    packets_sent,
                    peer = %peer,
                    "shutdown signal received"
                );
                return Ok(());
            }
            _ = ticker.tick() => {
                let bytes_sent = socket
                    .send_to(PAYLOAD, peer)
                    .await
                    .with_context(|| format!("failed to send UDP packet to {peer}"))?;
                packets_sent += 1;

                info!(
                    packets_sent,
                    bytes_sent,
                    peer = %peer,
                    "UDP packet sent"
                );
            }
        }
    }
}
