mod env;

use crate::env::load_environment;
use anyhow::{Context, Result, ensure};
use packet_stats::ReceiverPacketStats;
use packet_wire::Packet;
use tokio::net::UdpSocket;
use tokio::time::{Duration, MissedTickBehavior, interval};
use tracing::info;

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

    run_sink(
        socket,
        environment.packet_size,
        environment.stats_interval_seconds,
    )
    .await
}

async fn run_sink(
    socket: UdpSocket,
    packet_size: usize,
    stats_interval_seconds: u64,
) -> Result<()> {
    ensure!(
        packet_size >= Packet::MINIMUM_SIZE,
        "PACKET_SIZE must be at least {} bytes",
        Packet::MINIMUM_SIZE
    );
    ensure!(
        stats_interval_seconds > 0,
        "STATS_INTERVAL_SECONDS must be greater than 0"
    );

    let mut buffer = vec![0_u8; packet_size];
    let mut stats = ReceiverPacketStats::new();
    let stats_interval = Duration::from_secs(stats_interval_seconds);
    let mut stats_ticker = interval(stats_interval);
    let shutdown = tokio::signal::ctrl_c();

    stats_ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    stats_ticker.tick().await;
    tokio::pin!(shutdown);

    info!(packet_size, stats_interval_seconds, "sink running");

    loop {
        tokio::select! {
            biased;

            _ = &mut shutdown => {
                let totals = stats.totals();
                packet_stats::log_receiver_packet_stats_totals!(
                    totals,
                    total_packets_received,
                    total_bytes_received,
                    "shutdown signal received"
                );
                return Ok(());
            }
            _ = stats_ticker.tick() => {
                let stats = stats.snapshot();

                packet_stats::log_receiver_packet_stats!(
                    stats,
                    total_bytes_received,
                    "sink throughput"
                );
            }
            result = socket.recv_from(&mut buffer) => {
                let (bytes_received, peer_addr) = result.context("failed to receive UDP packet")?;
                let packet_bytes = &buffer[..bytes_received];

                match Packet::parse(packet_bytes) {
                    Ok(packet) => {
                        let sequence_number = packet.header.sequence_number;
                        stats.record_packet(sequence_number, bytes_received);
                    }
                    Err(error) => {
                        info!(
                            bytes_received,
                            peer_addr = %peer_addr,
                            error = %error,
                            "received invalid packet"
                        );
                    }
                }
            }
        }
    }
}
