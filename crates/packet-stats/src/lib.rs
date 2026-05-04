use std::collections::BTreeMap;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StatsSnapshot {
    pub total_packets: u64,
    pub total_bytes: u64,
    pub packets_per_second: f64,
    pub bytes_per_second: f64,
}

#[derive(Debug)]
pub struct PacketStats {
    total_packets: u64,
    total_bytes: u64,
    packets_since_snapshot: u64,
    bytes_since_snapshot: u64,
    last_snapshot_at: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SequenceStatsSnapshot {
    pub out_of_order_packets: u64,
    pub missing_packets: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketStatsTotals {
    pub total_packets: u64,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReceiverStatsSnapshot {
    pub total_packets: u64,
    pub total_bytes: u64,
    pub packets_per_second: f64,
    pub bytes_per_second: f64,
    pub out_of_order_packets: u64,
    pub missing_packets: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReceiverPacketStatsTotals {
    pub total_packets: u64,
    pub total_bytes: u64,
    pub out_of_order_packets: u64,
    pub missing_packets: u64,
}

impl PacketStats {
    pub fn new() -> Self {
        Self::with_started_at(Instant::now())
    }

    pub fn with_started_at(started_at: Instant) -> Self {
        Self {
            total_packets: 0,
            total_bytes: 0,
            packets_since_snapshot: 0,
            bytes_since_snapshot: 0,
            last_snapshot_at: started_at,
        }
    }

    pub fn record_packet(&mut self, bytes: usize) {
        let bytes = bytes as u64;
        self.total_packets += 1;
        self.total_bytes += bytes;
        self.packets_since_snapshot += 1;
        self.bytes_since_snapshot += bytes;
    }

    pub fn snapshot(&mut self) -> StatsSnapshot {
        self.snapshot_at(Instant::now())
    }

    pub fn snapshot_at(&mut self, now: Instant) -> StatsSnapshot {
        let elapsed_seconds = now.duration_since(self.last_snapshot_at).as_secs_f64();
        let packets_since_snapshot = self.packets_since_snapshot;
        let bytes_since_snapshot = self.bytes_since_snapshot;

        self.last_snapshot_at = now;
        self.packets_since_snapshot = 0;
        self.bytes_since_snapshot = 0;

        let (packets_per_second, bytes_per_second) = if elapsed_seconds > 0.0 {
            (
                packets_since_snapshot as f64 / elapsed_seconds,
                bytes_since_snapshot as f64 / elapsed_seconds,
            )
        } else {
            (0.0, 0.0)
        };

        StatsSnapshot {
            total_packets: self.total_packets,
            total_bytes: self.total_bytes,
            packets_per_second,
            bytes_per_second,
        }
    }

    pub const fn total_packets(&self) -> u64 {
        self.total_packets
    }

    pub const fn total_bytes(&self) -> u64 {
        self.total_bytes
    }

    pub const fn totals(&self) -> PacketStatsTotals {
        PacketStatsTotals {
            total_packets: self.total_packets,
            total_bytes: self.total_bytes,
        }
    }
}

#[derive(Debug)]
pub struct ReceiverPacketStats {
    packet_stats: PacketStats,
    sequence_stats: SequenceTracker,
}

impl ReceiverPacketStats {
    pub fn new() -> Self {
        Self::with_started_at(Instant::now())
    }

    pub fn with_started_at(started_at: Instant) -> Self {
        Self {
            packet_stats: PacketStats::with_started_at(started_at),
            sequence_stats: SequenceTracker::new(),
        }
    }

    pub fn record_packet(&mut self, sequence_number: u64, bytes: usize) {
        self.packet_stats.record_packet(bytes);
        self.sequence_stats.record_packet(sequence_number);
    }

    pub fn snapshot(&mut self) -> ReceiverStatsSnapshot {
        self.snapshot_at(Instant::now())
    }

    pub fn snapshot_at(&mut self, now: Instant) -> ReceiverStatsSnapshot {
        let packet_snapshot = self.packet_stats.snapshot_at(now);
        let sequence_snapshot = self.sequence_stats.snapshot();

        ReceiverStatsSnapshot {
            total_packets: packet_snapshot.total_packets,
            total_bytes: packet_snapshot.total_bytes,
            packets_per_second: packet_snapshot.packets_per_second,
            bytes_per_second: packet_snapshot.bytes_per_second,
            out_of_order_packets: sequence_snapshot.out_of_order_packets,
            missing_packets: sequence_snapshot.missing_packets,
        }
    }

    pub const fn total_packets(&self) -> u64 {
        self.packet_stats.total_packets()
    }

    pub const fn total_bytes(&self) -> u64 {
        self.packet_stats.total_bytes()
    }

    pub const fn out_of_order_packets(&self) -> u64 {
        self.sequence_stats.out_of_order_packets
    }

    pub const fn missing_packets(&self) -> u64 {
        self.sequence_stats.missing_packets
    }

    pub const fn totals(&self) -> ReceiverPacketStatsTotals {
        ReceiverPacketStatsTotals {
            total_packets: self.packet_stats.total_packets(),
            total_bytes: self.packet_stats.total_bytes(),
            out_of_order_packets: self.sequence_stats.out_of_order_packets,
            missing_packets: self.sequence_stats.missing_packets,
        }
    }
}

#[macro_export]
macro_rules! log_packet_stats {
    ($stats:expr, $total_bytes_field:ident, $($rest:tt)*) => {
        ::tracing::info!(
            pps = %$crate::format_pps($stats.packets_per_second),
            $total_bytes_field = %$crate::format_bytes($stats.total_bytes),
            throughput = %$crate::format_bytes_per_second($stats.bytes_per_second),
            $($rest)*
        )
    };
}

#[macro_export]
macro_rules! log_packet_stats_totals {
    ($stats:expr, $total_packets_field:ident, $total_bytes_field:ident, $($rest:tt)*) => {
        ::tracing::info!(
            $total_packets_field = $stats.total_packets,
            $total_bytes_field = %$crate::format_bytes($stats.total_bytes),
            $($rest)*
        )
    };
}

#[macro_export]
macro_rules! log_receiver_packet_stats {
    ($stats:expr, $total_bytes_field:ident, $($rest:tt)*) => {
        ::tracing::info!(
            pps = %$crate::format_pps($stats.packets_per_second),
            $total_bytes_field = %$crate::format_bytes($stats.total_bytes),
            throughput = %$crate::format_bytes_per_second($stats.bytes_per_second),
            out_of_order_packets = $stats.out_of_order_packets,
            missing_packets = $stats.missing_packets,
            $($rest)*
        )
    };
}

#[macro_export]
macro_rules! log_receiver_packet_stats_totals {
    ($stats:expr, $total_packets_field:ident, $total_bytes_field:ident, $($rest:tt)*) => {
        ::tracing::info!(
            $total_packets_field = $stats.total_packets,
            $total_bytes_field = %$crate::format_bytes($stats.total_bytes),
            out_of_order_packets = $stats.out_of_order_packets,
            missing_packets = $stats.missing_packets,
            $($rest)*
        )
    };
}

pub fn format_bytes(bytes: u64) -> String {
    format_byte_value(bytes as f64)
}

pub fn format_bytes_per_second(bytes_per_second: f64) -> String {
    format!("{}/s", format_byte_value(bytes_per_second))
}

pub fn format_pps(packets_per_second: f64) -> String {
    format_decimal(packets_per_second)
}

fn format_byte_value(bytes: f64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes;
    let mut unit_index = 0_usize;

    while value >= 1000.0 && unit_index < UNITS.len() - 1 {
        value /= 1000.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{value:.0} {}", UNITS[unit_index])
    } else {
        format!("{value:.2} {}", UNITS[unit_index])
    }
}

fn format_decimal(value: f64) -> String {
    let formatted = format!("{value:.2}");
    formatted
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_owned()
}

#[derive(Debug)]
struct SequenceTracker {
    next_expected_sequence_number: Option<u64>,
    highest_sequence_number_seen: Option<u64>,
    missing_ranges: BTreeMap<u64, u64>,
    out_of_order_packets: u64,
    missing_packets: u64,
}

impl SequenceTracker {
    fn new() -> Self {
        Self {
            next_expected_sequence_number: None,
            highest_sequence_number_seen: None,
            missing_ranges: BTreeMap::new(),
            out_of_order_packets: 0,
            missing_packets: 0,
        }
    }

    fn record_packet(&mut self, sequence_number: u64) {
        let Some(next_expected_sequence_number) = self.next_expected_sequence_number else {
            self.next_expected_sequence_number = Some(sequence_number.saturating_add(1));
            self.highest_sequence_number_seen = Some(sequence_number);
            return;
        };

        let highest_sequence_number_seen = self
            .highest_sequence_number_seen
            .expect("highest seen sequence is initialized with next expected sequence");

        if sequence_number < next_expected_sequence_number {
            self.out_of_order_packets += 1;
            return;
        }

        if sequence_number == next_expected_sequence_number {
            if sequence_number <= highest_sequence_number_seen {
                let removed = self.remove_missing_sequence_number(sequence_number);
                debug_assert!(removed, "next expected sequence should be marked missing");
                if removed {
                    self.out_of_order_packets += 1;
                    self.missing_packets = self.missing_packets.saturating_sub(1);
                }
            } else {
                self.highest_sequence_number_seen = Some(sequence_number);
            }

            self.next_expected_sequence_number = Some(sequence_number.saturating_add(1));
            self.advance_next_expected_sequence_number();
            return;
        }

        if sequence_number > highest_sequence_number_seen {
            let gap_start = highest_sequence_number_seen.saturating_add(1);
            let gap_end = sequence_number.saturating_sub(1);

            if gap_start <= gap_end {
                self.missing_ranges.insert(gap_start, gap_end);
                self.missing_packets += gap_end - gap_start + 1;
            }

            self.highest_sequence_number_seen = Some(sequence_number);
            return;
        }

        if self.remove_missing_sequence_number(sequence_number) {
            self.out_of_order_packets += 1;
            self.missing_packets = self.missing_packets.saturating_sub(1);
        }
    }

    fn snapshot(&self) -> SequenceStatsSnapshot {
        SequenceStatsSnapshot {
            out_of_order_packets: self.out_of_order_packets,
            missing_packets: self.missing_packets,
        }
    }

    fn advance_next_expected_sequence_number(&mut self) {
        let mut next_expected_sequence_number = self
            .next_expected_sequence_number
            .expect("next expected sequence should be initialized");

        let highest_sequence_number_seen = self
            .highest_sequence_number_seen
            .expect("highest seen sequence is initialized with next expected sequence");

        if next_expected_sequence_number > highest_sequence_number_seen {
            return;
        }

        if let Some((&range_start, _)) = self.missing_ranges.iter().next() {
            if next_expected_sequence_number < range_start {
                next_expected_sequence_number = range_start;
            }
        } else {
            next_expected_sequence_number = highest_sequence_number_seen.saturating_add(1);
        }

        self.next_expected_sequence_number = Some(next_expected_sequence_number);
    }

    fn remove_missing_sequence_number(&mut self, sequence_number: u64) -> bool {
        let Some((&range_start, &range_end)) =
            self.missing_ranges.range(..=sequence_number).next_back()
        else {
            return false;
        };

        if sequence_number > range_end {
            return false;
        }

        self.missing_ranges.remove(&range_start);

        if range_start < sequence_number {
            self.missing_ranges
                .insert(range_start, sequence_number.saturating_sub(1));
        }

        if sequence_number < range_end {
            self.missing_ranges
                .insert(sequence_number.saturating_add(1), range_end);
        }

        true
    }
}
