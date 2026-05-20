# Changelog

## [0.13.1] - 2026-05-20

- Widened `doom-fish-utils` dependency bound to `<0.4` so the 0.3.x SPSC-ring release resolves cleanly.
- Expanded README with explicit async / sync usage sections, executor-agnostic design notes, and a curated examples list.

## [0.13.0] - 2026-05-18

- Added `FrameworkError::cf_error()` for safe access to the underlying `apple_cf::cf::CFError` via `nw_error_copy_cf_error`.
- Added `ProtocolMetadata::set_pong_handler()` for safe WebSocket pong callbacks via `nw_ws_metadata_set_pong_handler`.
- Closed the final two SDK coverage gaps and refreshed `COVERAGE_AUDIT.md` to 100.0% verified coverage (500 / 500 symbols).

## [0.12.2] - 2026-05-18

- Widen apple-cf version bound to `<0.10` so 0.9.x resolves.

## [0.12.1] - 2026-05-18

- Added `Debug` coverage for the remaining public structs across `src/`, using derives for stream/message containers and manual pointer-printing impls for raw-handle wrappers.

## [0.12.0] - 2026-05-18

### Breaking

- `pub mod ffi` is now `pub(crate) mod ffi` — raw FFI symbols are no longer
  reachable through `networkframework::ffi::*`. The `raw_ffi` module is the
  intended escape hatch and was already gated behind the `raw-ffi` Cargo
  feature; this change makes the gating effective. Users who need raw FFI
  access must add `features = ["raw-ffi"]` and import from
  `networkframework::raw_ffi::*` instead.
- Added `#[cfg_attr(docsrs, doc(cfg(feature = "raw-ffi")))]` to `raw_ffi` so
  the feature requirement renders in rustdoc.

## [0.11.1] - 2026-05-17

- Fixed Arc strong-reference leak in `PathMonitor`: the raw Arc clone given to
  the C shim's update handler was never reconstituted; it is now freed in `Drop`
  after `nw_path_monitor_cancel` + `dispatch_sync` confirms the queue is idle.
- Fixed Arc strong-reference leak in `TcpClient`: `viability_raw`,
  `better_path_raw`, and `path_raw` Arc clones given to the C shim were never
  freed; they are now reconstituted in `Drop` after `nw_shim_tcp_close` confirms
  the connection's serial queue has drained.
- Added `doom_fish_utils::panic_safe::catch_user_panic` wrappers to all
  `extern "C"` callbacks that invoke user-supplied `FnMut` closures, preventing
  UB from Rust panics unwinding across the C ABI.
- Added `// SAFETY:` comments to all unsafe blocks in `client/mod.rs`,
  `path_monitor/mod.rs`, and `async_api.rs`.
- Tightened `Cargo.toml` version ranges: `apple-cf` `>=0.7, <0.9`;
  `doom-fish-utils` `>=0.1, <0.3`.

## [0.11.0] - 2026-05-17

- Added a new `async_api` Tier-2 module backed by `doom_fish_utils::stream::BoundedAsyncStream`.
- Exposed seven async stream surfaces for connections, listeners, path monitors, and browsers.
- Added new C-shim/FFI subscription helpers, an async example, and smoke tests for stream subscription/drop flows.
- Bumped the crate version to 0.11.0.

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
