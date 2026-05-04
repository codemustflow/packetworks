mod env;

use crate::env::load_environment;
use anyhow::{Context, Result, ensure};
use packet_stats::PacketStats;
use packet_wire::{Packet, PacketMut};
use std::net::SocketAddr;
use tokio::net::{UdpSocket, lookup_host};
use tokio::time::{Duration, MissedTickBehavior, interval};
use tracing::info;

const PAYLOAD_BODY: &[u8] = b"packetworks-source";

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

    run_source(
        socket,
        peer,
        environment.packet_size,
        environment.stats_interval_seconds,
    )
    .await
}

async fn run_source(
    socket: UdpSocket,
    peer: SocketAddr,
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

    let mut stats = PacketStats::new();
    let mut next_sequence_number = 0_u64;
    let mut packet_bytes = vec![0_u8; packet_size];
    let stats_interval = Duration::from_secs(stats_interval_seconds);
    let mut stats_ticker = interval(stats_interval);
    let shutdown = tokio::signal::ctrl_c();

    {
        let mut packet =
            PacketMut::new(&mut packet_bytes).expect("packet size was validated before allocation");
        let payload = packet.payload_mut();
        let body_len = payload.len().min(PAYLOAD_BODY.len());
        payload[..body_len].copy_from_slice(&PAYLOAD_BODY[..body_len]);
    }
    stats_ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    stats_ticker.tick().await;

    tokio::pin!(shutdown);

    info!(
        peer = %peer,
        packet_size = packet_bytes.len(),
        stats_interval_seconds,
        "source running"
    );

    loop {
        tokio::select! {
            biased;

            _ = &mut shutdown => {
                let totals = stats.totals();
                packet_stats::log_packet_stats_totals!(
                    totals,
                    total_packets_sent,
                    total_sent,
                    peer = %peer,
                    "shutdown signal received"
                );
                return Ok(());
            }
            _ = stats_ticker.tick() => {
                let stats = stats.snapshot();

                packet_stats::log_packet_stats!(
                    stats,
                    total_sent,
                );
            }
            result = async {
                let mut packet = PacketMut::new(&mut packet_bytes)
                    .expect("packet size was validated before entering the run loop");
                packet.header.sequence_number = next_sequence_number;
                let bytes_sent = socket
                    .send_to(packet.as_bytes(), peer)
                    .await
                    .with_context(|| format!("failed to send UDP packet to {peer}"))?;

                Ok::<usize, anyhow::Error>(bytes_sent)
            } => {
                let bytes_sent = result?;
                next_sequence_number += 1;
                stats.record_packet(bytes_sent);
            }
        }
    }
}
