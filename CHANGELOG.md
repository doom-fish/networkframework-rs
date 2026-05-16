# Changelog

## 0.9.0 - 2026-05-16

- Switched the native build from the legacy C-only shim to a SwiftPM bridge layout.
- Added dedicated safe modules for Connection, Endpoint, Path, Protocol, AdvertiseDescriptor,
  and alias modules for ContentContext, Group, Resolver, ProxyConfig, and PrivacyContext.
- Added parameter attribution, interface-type, expensive/constrained policy, and application-service helpers.
- Added browse-descriptor, advertise-descriptor, relay-hop, proxy-domain, QUIC-options,
  and connection/path metadata wrappers.
- Added `raw-ffi` for direct bridge access.
- Added `COVERAGE.md`, per-area smoke tests, and runnable examples for the requested logical areas.

## 0.8.0 - 2026-05-16

- Added custom protocol framers via `FramerDefinition`, `Framer`, `FramerContext`, and `FramerMessage`.
- Added `ConnectionParameters` plus explicit `ContentContext` send/receive support for TCP, UDP, and QUIC.
- Added `ConnectionGroup` / `ConnectionGroupDescriptor` wrappers for multicast and multiplex groups.
- Added interface enumeration via `list_interfaces()` and `PathMonitor::list_interfaces()`.
- Added `PrivacyContext`, `ProxyConfig`, and `ResolverConfig` wrappers attachable to parameters.
- Added `framer_length_prefix`, `interface_list`, and `connection_group` examples.

## 0.1.0 - 2025-

- First release.
- `TcpClient::connect/send/receive/close` over `nw_connection_t`.
- `TcpListener::bind/accept/local_port` over `nw_listener_t`.
- Built on a tiny C shim around Apple's block-based Network.framework C API; no Objective-C runtime, no Swift bridge.
