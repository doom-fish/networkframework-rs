# networkframework-rs

Safe Rust bindings for Apple's
[Network.framework](https://developer.apple.com/documentation/network),
backed by a Swift bridge plus an opt-in raw FFI surface.

**SDK coverage:** the 500 functions, types and constants declared in
Network.framework's own headers in the macOS 26.2 SDK are reachable from the
crate, one of them only through `raw-ffi`. The count leaves out the
`sec_protocol_*` API from Security.framework that configures TLS and QUIC (the
essentials are now wrapped in the `tls` module) and anything added in later
SDKs. See [`COVERAGE.md`](COVERAGE.md) for the logical-area map and
[`COVERAGE_AUDIT.md`](COVERAGE_AUDIT.md) for the symbol list.

## Why this crate

- **Broad transport coverage:** TCP clients and listeners, UDP, TLS, QUIC,
  WebSocket, and Bonjour discovery/advertising all live in one safe crate.
- **More than sockets:** endpoints, connection parameters, path monitoring,
  content contexts, framers, connection groups, privacy contexts, and
  resolver / proxy configuration are part of the public safe API.
- **Async where it helps:** this is the Tier 1 crate with a native
  `async`/`await` story for Network.framework event streams.
- **Runtime-gated APIs handled for you:** newer framework features stay safe,
  and unsupported runtime combinations surface as Rust errors instead of raw
  Objective-C / C state leaking upward.
- **Escape hatch available:** enable `raw-ffi` when you need direct bridge
  access beyond the safe wrappers.

## Feature flags

- Default: safe blocking + callback-oriented API.
- `async`: enables `networkframework::async_api` stream wrappers.
- `raw-ffi`: exposes `networkframework::raw_ffi::*`.

## Installation

```toml
[dependencies]
networkframework = "0.14.0"
```

Enable async support explicitly when you want awaitable event streams:

```toml
[dependencies]
networkframework = { version = "0.14.0", features = ["async"] }
```

## Async usage

The `networkframework::async_api` module turns callback-based
Network.framework notifications into executor-agnostic Rust futures/streams.
It does **not** lock you into Tokio, async-std, or any other runtime-specific
public API. Instead, the crate uses `doom-fish-utils` async primitives
(`doom_fish_utils::completion` helpers plus the stream-based `next().await`
surface) so the same types compose with whatever executor your application
already uses.

Today the async surface includes:

- `ConnectionStateStream`
- `ConnectionViabilityStream`
- `ConnectionBetterPathStream`
- `ConnectionPathChangedStream`
- `ListenerEventStream`
- `PathUpdateStream`
- `BrowserEventStream`

While a `ListenerEventStream` exists, ready inbound connections go to the
stream instead of to `TcpListener::accept`.

A minimal async round trip can use an awaitable listener stream while keeping
transport I/O on the same `TcpClient` / accepted-connection types as the sync
API:

```rust,no_run
use networkframework::async_api::{ListenerEvent, ListenerEventStream};
use networkframework::{TcpClient, TcpListener};

async fn run() -> Result<(), networkframework::NetworkError> {
    let listener = TcpListener::bind_loopback(0)?;
    let port = listener.local_port();
    let events = ListenerEventStream::subscribe(&listener, 8);

    let client = TcpClient::connect("127.0.0.1", port)?;
    client.send(b"ping")?;

    while let Some(event) = events.next().await {
        match event {
            ListenerEvent::NewConnection(server) => {
                let request = server.receive(1024)?;
                assert_eq!(request, b"ping");
                server.send(b"pong")?;
                break;
            }
            ListenerEvent::State { .. } => {}
        }
    }

    let reply = client.receive(1024)?;
    assert_eq!(reply, b"pong");
    Ok(())
}
```

For tiny binaries, `pollster::block_on(run())` is often enough. In larger
applications, just `await` the same stream types from your existing runtime.
The bundled [`07_async_streams`](examples/07_async_streams.rs) example shows
`PathUpdateStream` and `ConnectionStateStream` in practice.

## Sync / callback usage

The original API remains small and direct for request / response flows. If you
prefer a blocking round trip, the same TCP primitives work without any feature
flags:

```rust,no_run
use networkframework::{TcpClient, TcpListener};

fn main() -> Result<(), networkframework::NetworkError> {
    let listener = TcpListener::bind_loopback(0)?;
    let port = listener.local_port();
    let server = std::thread::spawn(move || -> Result<(), networkframework::NetworkError> {
        let connection = listener.accept()?;
        let request = connection.receive(1024)?;
        assert_eq!(request, b"ping");
        connection.send(b"pong")?;
        Ok(())
    });

    let client = TcpClient::connect("127.0.0.1", port)?;
    client.send(b"ping")?;
    let reply = client.receive(1024)?;
    assert_eq!(reply, b"pong");

    server.join().expect("server thread")?;
    Ok(())
}
```

Callback-oriented APIs are still available for long-lived observers and
framework-managed events. Common entry points include
`start_path_monitor`, `start_browser_with_descriptor`,
`start_browser_results_with_descriptor`, `advertise_with_descriptor`, and the
various `set_*_handler` hooks on connection, group, and protocol types.
`PathMonitorBuilder` sets a monitor's interface scope and prohibited interface
types before it starts, which is the only time Network.framework applies them.

Dropping a connection, listener, group, browser or path monitor cancels it. No
new callback starts after that, but one that is already running may finish
after the drop returns. Native state is freed once Network.framework delivers
its final `cancelled` event.

Network.framework does not synchronize most of its configuration objects, so
one object never has two handles that can reach it from different threads.
`ConnectionParameters` and `TxtRecord` clone deeply, and parameters read back
from a connection, browser, group or framer are independent copies. Protocol
options, protocol stacks, descriptors, proxy and relay configurations,
WebSocket responses and content contexts are not `Clone`. Handing one to a
parent (`prepend_application_protocol`, `set_transport_protocol`,
`ConnectionGroup::new`, `start_browser_with_descriptor`,
`set_proxy_configurations`) moves it there, and accessors lend it back as a
`ReadOnly` view. Metadata, framer messages and privacy contexts, whose setters
the framework locks or serializes, still share one object across clones.

## Listeners, TLS and QUIC

- `TcpListener::bind(port)` listens on every interface, so other machines can
  connect. `TcpListener::bind_loopback(port)` listens on `127.0.0.1` only.
- `accept()` returns only connections whose handshake finished. Connections
  that fail, or that are not ready within 10 s, are dropped; at most 128 ready
  connections wait to be accepted.
- `TcpListener::bind_tls(port, &identity)` serves TLS 1.2 or newer on every
  interface. `TlsIdentity::from_pkcs12` imports an identity without touching
  the keychain (macOS 15 or later; older systems get
  `NetworkError::Unsupported`), and `TlsIdentity::from_sec_identity` wraps a
  `SecIdentityRef` you already have.
- `ConnectionParameters::tls_tcp_configured` and
  `ConnectionParameters::quic_configured` expose the `sec_protocol_options`
  essentials: local identity, TLS version range, ALPN, server name and peer
  verification. Peers are checked against the system trust store unless you
  install `set_verify_handler`. `pin_peer_certificate_sha256` accepts only leaf
  certificates with the given SHA-256 digests, which suits self-signed servers.
  There is no accept-all switch.
- QUIC parameters come from `nw_parameters_create_quic`. On a QUIC listener,
  `accept()` first returns the connection for the QUIC tunnel, then one
  connection per stream the peer opens.
- `TcpListener::bind_with_group_handler(port, &parameters, handler)` delivers
  each inbound QUIC connection as a `ConnectionGroup` instead. The handler is
  installed before the listener starts, and `accept()` on such a listener
  returns `NetworkError::InvalidArgument`.

```rust,no_run
use networkframework::{certificate_sha256, ConnectionParameters, TcpClient, TlsVersion};

fn main() -> Result<(), networkframework::NetworkError> {
    let certificate = std::fs::read("server-certificate.der").expect("certificate file");
    let pin = certificate_sha256(&certificate);
    let parameters = ConnectionParameters::tls_tcp_configured(|tls| {
        tls.set_min_tls_version(TlsVersion::Tls13)
            .pin_peer_certificate_sha256(&[pin]);
    })?;
    let client = TcpClient::connect_with_parameters("server.local", 8443, &parameters)?;
    client.send(b"hello")?;
    Ok(())
}
```

## Message boundaries

`UdpClient::receive`, `UdpClient::receive_with_context`, `WebSocket::receive`
and `TcpClient::receive_message` return one whole message. A message longer than the requested limit is
consumed and reported as `NetworkError::MessageTooLarge { size, limit }`
instead of being truncated or split. `TcpClient::receive` reads a byte stream
and returns up to `max_len` bytes.

## Examples

The repository ships runnable examples across the main areas of the crate:

- `01_get_example` — local TCP listener/client round trip.
- `02_tls_get` — `ConnectionParameters` policy tuning and protocol stacking.
- `03_udp_and_path` — UDP parameters, endpoint construction, and path monitor
  snapshots.
- `04_bonjour` — Bonjour browse descriptors and browser events.
- `05_websocket` — WebSocket protocol definitions plus QUIC option inspection.
- `06_bonjour_advertise` — Bonjour advertising with TXT records.
- `07_async_streams` — async stream subscriptions for path and connection
  events (`--features async`).
- `framer_length_prefix` — custom length-prefixed framer wiring.
- `interface_list` — enumerating local network interfaces.
- `connection_group` — multicast connection groups and state callbacks.
- `content_context_overview` — content-context identifiers, priorities, and
  antecedents.
- `resolver_overview` — DNS-over-HTTPS and DNS-over-TLS resolver configs.
- `privacy_context_overview` — encrypted name resolution plus proxy policy.
- `quic_options` — QUIC transport tuning and ALPN setup.

Run any example directly:

```bash
cargo run --example 01_get_example
cargo run --example 07_async_streams --features async
```

## Coverage and audit

- [`COVERAGE.md`](COVERAGE.md) — logical-area coverage map, example links, and
  test references.
- [`COVERAGE_AUDIT.md`](COVERAGE_AUDIT.md) — symbol table for the 500 symbols
  in Network.framework's macOS 26.2 headers.

## Availability notes

The crate needs macOS 14 or later, the Swift bridge's deployment target. Some
Apple APIs are runtime-gated further by the operating system:

- Application-service browsing / advertising / parameters: macOS 13+
- Relay and Oblivious HTTP proxy configuration: macOS 14+
- Importing a PKCS#12 identity with `TlsIdentity::from_pkcs12`: macOS 15+
- Ultra-constrained path / parameter flags and link quality: newer
  SDK/runtime combinations

The safe wrappers return `NetworkError::InvalidArgument` (or
`NetworkError::Unsupported` for `TlsIdentity::from_pkcs12`) when a requested
API is unavailable at runtime.

## Validation

```bash
cargo build --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

## Status

Actively developed. Targets macOS and tracks the audited Network.framework SDK
surface closely.

## License

Licensed under either of:

- MIT license ([`LICENSE-MIT`](LICENSE-MIT))
- Apache License 2.0 ([`LICENSE-APACHE`](LICENSE-APACHE))

at your option.
