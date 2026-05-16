# Changelog

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
- Built on a tiny C shim around Apple's block-based Network.framework
  C API; no Objective-C runtime, no Swift bridge.
