# AGENTS.md

## What this project is

packetworks is an eBPF learning and engineering project. The end goal is a **dumb but very fast UDP relay**. It forwards packets based on a route table that is configured **from outside while it runs** (over HTTP or Redis) and changes take effect without a restart.

"Dumb" is a design rule. The relay looks up the destination address and port, rewrites the headers and forwards the packet. It does not inspect payloads, reassemble anything or keep per-flow logic beyond what routing needs. All of the flexibility lives in the configuration, not in the packet path.

The repo also has its own traffic generator (`source`) and receiver (`sink`). The packet header carries a sequence number, so the sink can measure end-to-end throughput, loss and reordering through the relay without knowing the relay exists.

## Current status

- `source` and `sink` work. They send and receive sequence-numbered UDP packets and log packets/s, throughput, out-of-order and missing counts.
- `relay` is a placeholder. It loads its config, starts logging and exits.
- There is no eBPF code yet and there are no tests yet.

## Target architecture

```
            ┌──────────── relay (userspace, Rust/tokio) ──────────────┐
 HTTP ─────▶│  control plane: RouteSource (HTTP | Redis) → reconcile  │
 Redis ────▶│  stats poller: per-CPU counters → logs (packet-stats)   │
            └───────────────┬─────────────────────────▲───────────────┘
                     map updates                 map reads
            ┌───────────────▼─────────────────────────┴───────────────┐
 NIC ──────▶│  XDP program (kernel): parse eth/ip/udp → ROUTES lookup │──▶ NIC
            │  rewrite addrs, fix checksums → XDP_TX / redirect       │
            │  no match → XDP_PASS (normal kernel stack)              │
            └─────────────────────────────────────────────────────────┘
```

- **Data plane:** an XDP program written in Rust with [aya](https://aya-rs.dev).
  - Looks up `ROUTES: HashMap<(dst_ip, dst_port), Target>`.
  - Uses `bpf_fib_lookup` to get the next-hop MAC address and outgoing interface.
  - Updates checksums incrementally.
  - Sends the packet out with `XDP_TX`, or with `bpf_redirect_map` through a `DEVMAP` when it leaves on another interface.
  - Packets that match no route get `XDP_PASS`, so normal traffic is never broken.
- **Control plane:** the userspace `relay` binary.
  - Owns the map handles.
  - Reads the desired routes from a pluggable `RouteSource` (HTTP or Redis) and writes them into the eBPF map. Updating one map entry is atomic, so routes can change while packets are flowing.
- **Stats:** per-route counters in per-CPU maps. Userspace polls them and logs them with the `packet-stats` formatters.

Open design questions (do not assume an answer without asking):

- **Fan-out (one packet to several destinations):** XDP cannot copy packets, so this would need a TC program with `bpf_clone_redirect`.
- **Return traffic:** two-way relaying needs reverse mappings, which is basically NAT state. One-way is much simpler.

### Roadmap

1. Userspace relay (tokio, routes from env vars). This is the performance baseline and lets the test harness cover the relay end to end.
2. Network namespace and veth test topology in xtask: `source ns ⇄ relay ns ⇄ sink ns`.
3. XDP data plane with aya, routes from env vars, measured against the baseline.
4. HTTP control plane, then Redis.
5. Faster load generator (`sendmmsg` batching, multiple threads, possibly AF_XDP).

## Repository layout

```
apps/
  source/         UDP traffic generator: sends sequence-numbered packets as fast as it can
  sink/           UDP receiver: parses packets and tracks throughput, loss and reordering
  relay/          The relay (placeholder for now)
crates/
  packet-wire/    Packet format: 8-byte big-endian u64 sequence number + payload
                  (`Packet` reads a received buffer without copying, `PacketMut` fills one for sending)
  packet-stats/   PacketStats, ReceiverPacketStats, SequenceTracker (gaps stored as ranges
                  in a BTreeMap), byte and rate formatters, log_*! macros
  logging/        tracing setup: flattened JSON by default, pretty output with LOG_FOR_HUMANS=true
xtask/            Build, CI and dev commands (see below)
.circleci/        CI: runs `cargo ci`, saves debug and release binaries as artifacts
.zed/             Editor debugger configs for each app
```

## Commands

These are aliases defined in `.cargo/config.toml`, all backed by `xtask`:

| Command | What it does |
|---|---|
| `cargo ci` | fmt check, then clippy (`-D warnings`), tests, and debug + release builds |
| `cargo build-all` | Debug and release builds of every crate except xtask |
| `cargo p2p` | Runs `sink` on 127.0.0.1:9101 and `source` on :9102 sending to it, with output prefixed by app name |
| `cargo xtask fmt\|lint\|test` | Runs that step on its own |

Run `cargo ci` before saying any change is done.

## Configuration

Every app reads environment variables in its own `src/env.rs` (a `confique` `Config` struct, and a `.env` file is loaded if present):

| Variable | Apps | Default |
|---|---|---|
| `LOG_LEVEL` | all | `info` |
| `LOG_FOR_HUMANS` | all | `false` (JSON) |
| `BIND_ADDRESS` | source, sink | `0.0.0.0` |
| `BIND_PORT` | source, sink | source `0` (OS picks a port), sink `9000` |
| `PEER_ADDRESS`, `PEER_PORT` | source | required |
| `PACKET_SIZE` | source, sink | `1500` |
| `STATS_INTERVAL_SECONDS` | source, sink | `1` |

## Conventions

- **Toolchain:** stable Rust, edition 2024, `style_edition = "2024"`. The eBPF crate will need nightly and `bpf-linker`. Keep that confined to the eBPF build so the rest of the workspace stays on stable.
- **Dependencies:** declare them once in the root `[workspace.dependencies]` and use `dep.workspace = true` in member crates.
- **Binaries:** set `autobins = false` and put the entry point at `src/<name>.rs` with an explicit `[[bin]]`.
- **Errors:** use `anyhow` with `.context(...)` / `.with_context(...)` in apps. Library error types implement `std::error::Error` (see `PacketError`). Don't use bare `unwrap()`. Use `expect("why this can't fail")` only for true invariants.
- **Logging:** always go through the `logging` crate and use structured `tracing` fields (`info!(peer = %peer, "...")`), not formatted strings. Use the `packet_stats::log_*!` macros for stats so field names stay the same across apps.
- **Async apps:** tokio with a `biased` `select!` loop: shutdown first, then the stats ticker, then I/O. Ctrl-C logs final totals.
- **Hot paths:** don't allocate per packet. Reuse buffers, as `source` and `sink` already do.
- **Testing XDP:** loopback (`127.0.0.1`) only supports generic XDP and never really routes traffic through the relay. Use the network namespace and veth topology for any eBPF testing. Setting it up needs root or `CAP_NET_ADMIN`.
