mod env;

use crate::env::load_environment;
use anyhow::{Context, Result};
use tokio::net::UdpSocket;
use tracing::info;

const MAX_PACKET_SIZE: usize = 2048;

#[tokio::main]
async fn main() -> Result<()> {
    let environment = load_environment().context("failed to load environment")?;
    let _logging = logging::init_logging("sink", environment.log_level, environment.log_for_humans)
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

    info!(
        local_addr = %local_addr,
        configured_bind_address = %environment.bind_address,
        configured_bind_port = environment.bind_port,
        "sink UDP socket bound"
    );

    run_sink(socket).await
}

async fn run_sink(socket: UdpSocket) -> Result<()> {
    let mut buffer = [0_u8; MAX_PACKET_SIZE];
    let mut packets_received = 0_u64;

    info!("sink running");

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                info!(packets_received, "shutdown signal received");
                return Ok(());
            }
            result = socket.recv_from(&mut buffer) => {
                let (bytes_received, peer_addr) = result.context("failed to receive UDP packet")?;
                packets_received += 1;

                info!(
                    packets_received,
                    bytes_received,
                    peer_addr = %peer_addr,
                    "UDP packet received"
                );
            }
        }
    }
}
