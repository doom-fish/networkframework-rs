# networkframework-rs

Safe Rust bindings for Apple's
[Network.framework](https://developer.apple.com/documentation/network),
the modern (10.14+) replacement for BSD sockets / `CFNetwork` / `NSStream`.

v0.1 covers:

- `TcpClient` — outbound TCP connection (`nw_connection`).
- `TcpListener` — inbound TCP listener (`nw_listener`).
- Blocking `send` / `receive` helpers built on top of dispatch queues.

Built using a thin C shim around Apple's block-based `nw_*` C API; no
Objective-C runtime, no Swift bridge required.

## Why not just use `std::net`?

`std::net` calls BSD sockets directly, which works but bypasses macOS's
modern network stack (cellular fallback, Wi-Fi assist, Network
Extensions, secure DNS, multipath, on-device proxying). Apps shipped
via the Mac App Store **must** use Network.framework for many of those
behaviours. This crate provides a tiny safe surface for that.

## Quick start

```rust,no_run
use networkframework::TcpClient;

let client = TcpClient::connect("example.com", 80)?;
client.send(b"GET / HTTP/1.0\r\nHost: example.com\r\n\r\n")?;
let response = client.receive(8192)?;
println!("got {} bytes", response.len());
# Ok::<_, networkframework::NetworkError>(())
```
