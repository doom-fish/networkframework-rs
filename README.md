# networkframework-rs

Safe Rust bindings for Apple's
[Network.framework](https://developer.apple.com/documentation/network),
the modern (10.14+) replacement for BSD sockets / `CFNetwork` / `NSStream`.

v0.8 covers:

- `TcpClient`, `TcpListener`, `UdpClient`, `QuicConnection`, `WebSocket`, `Browser`, and `PathMonitor`.
- `ConnectionParameters` for advanced protocol-stack configuration.
- `ContentContext` for per-message priority, expiration, antecedents, and protocol metadata.
- Custom protocol framers via `FramerDefinition`, `Framer`, `FramerContext`, and `FramerMessage`.
- `ConnectionGroup` / `ConnectionGroupDescriptor` for multicast and multiplex groups.
- Interface enumeration via `list_interfaces()` and `PathMonitor::list_interfaces()`.
- `PrivacyContext`, `ProxyConfig`, and `ResolverConfig` for proxy and encrypted-DNS policy.

Built using a thin C shim around Apple's block-based `nw_*` C API; no
Objective-C runtime, no Swift bridge required.

## Why not just use `std::net`?

`std::net` calls BSD sockets directly, which works but bypasses macOS's
modern network stack (cellular fallback, Wi-Fi assist, Network
Extensions, secure DNS, multipath, on-device proxying). Apps shipped
via the Mac App Store **must** use Network.framework for many of those
behaviours. This crate provides a tiny safe surface for that.

## Remaining gaps

The crate now covers most publicly useful `Network.framework` surfaces. Remaining
work is focused on deeper protocol metadata helpers, richer connection-group
operations such as extraction / reinsertion, and additional low-level protocol
options.

## Quick start

```rust,no_run
use networkframework::TcpClient;

let client = TcpClient::connect("example.com", 80)?;
client.send(b"GET / HTTP/1.0\r\nHost: example.com\r\n\r\n")?;
let response = client.receive(8192)?;
println!("got {} bytes", response.len());
# Ok::<_, networkframework::NetworkError>(())
```

## Included examples

- `cargo run --example framer_length_prefix`
- `cargo run --example interface_list`
- `cargo run --example connection_group`
- `cargo run --example 03_udp_and_path`
