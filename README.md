# networkframework-rs

Safe Rust bindings for Apple's
[Network.framework](https://developer.apple.com/documentation/network),
backed by a Swift bridge plus an opt-in raw FFI surface.

## What's new in v0.9

- `SwiftPM` bridge modeled after the `screencapturekit-rs` layout.
- Safe modules for the requested logical areas:
  `Connection`, `Listener`, `Browser`, `Parameters`, `Endpoint`, `Path`,
  `Framer`, `Group`, `Protocol`, `ContentContext`, `Resolver`, `Quic`,
  `PrivacyContext`, `ProxyConfig`, and `AdvertiseDescriptor`.
- `raw-ffi` feature for direct access to the low-level bridge symbols.
- `COVERAGE.md`, per-area smoke tests, and runnable examples for every area.

## Crate layout

- Safe API: enabled by default.
- Raw bridge API: `--features raw-ffi`
- Coverage report: [`COVERAGE.md`](COVERAGE.md)

## Quick start

```rust,no_run
use networkframework::{TcpClient, TcpListener};

let listener = TcpListener::bind(0)?;
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
# Ok::<_, networkframework::NetworkError>(())
```

## Availability notes

Some Apple APIs are runtime-gated by the operating system:

- Application-service browsing / advertising / parameters: macOS 13+
- Relay and Oblivious HTTP proxy configuration: macOS 14+
- Ultra-constrained path / parameter flags and link quality: newer SDK/runtime combinations

The safe wrappers return `NetworkError::InvalidArgument` when a requested API is unavailable at runtime.

## Examples

```bash
cargo run --example 01_get_example
cargo run --example 04_bonjour
cargo run --example 06_bonjour_advertise
cargo run --example framer_length_prefix
cargo run --example content_context_overview
cargo run --example resolver_overview
cargo run --example privacy_context_overview
cargo run --example quic_options
```

## Validation

```bash
cargo clippy --all-targets -- -D warnings
cargo test
for ex in examples/*.rs; do cargo run --example "$(basename "$ex" .rs)"; done
```
