# Changelog

## 0.10.0 - 2025-MM-DD

- Closed all 2 remaining coverage gaps: added `FrameworkError::copy_cf_error()` for safe Core Foundation error conversion via `apple_cf::cf::CFError`, and added `WebSocket::set_pong_handler()` / `set_pong_handler_with_context()` for WebSocket pong-frame callback registration.
- Dependency: Added `apple-cf` v0.7.0 for safe Core Foundation type wrappers.
- Coverage audit v2: 100.0% verified (500 / 500 SDK symbols).

## 0.9.3 - 2026-05-17

- Added seven new integration tests covering Connection, Listener, Browser, Parameters,
  Endpoint, Path, and Framer areas under `tests/additional_area_smoke.rs`.
- Bumped the crate version to 0.9.3.

## 0.9.2 - 2026-05-17

- Added rich browse-result coverage via `BrowseResult`, `BrowseResultChange`, and `BrowseResultsBrowser`.
- Added advanced connection, listener, connection-group, path-monitor, and WebSocket callback hooks to the safe API.
- Added URL-session proxy-configuration coverage, launchd-listener support, custom-IP parameters, and remaining framer/browser/listener/group helpers.
- Refreshed `COVERAGE_AUDIT.md` to 99.60% verified coverage (498 / 500 SDK symbols).

## 0.9.1 - 2026-05-16

- Added `ConnectionParameters` coverage for required/prohibited interfaces, local endpoints,
  service/multipath policy, DNS controls, and `ProtocolStack` access.
- Added `Endpoint::raw_address()`, `Endpoint::txt_record()`, and a new `TxtRecord` wrapper for
  dictionary- and byte-backed TXT records.
- Added `EstablishmentReport`, `DataTransferReport`, `ResolutionReport`, `QuicMetadata`, QUIC
  security-handle wrappers, and `EthernetChannel`.
- Expanded smoke coverage and refreshed `COVERAGE_AUDIT.md` to 73.20% verified coverage
  (366 / 500 SDK symbols).

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
