# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.14.0] - Unreleased

### Security

- Connections are no longer freed while their state handler can still run.
  Shim handles are reference counted and released only after
  Network.framework delivers the final `cancelled` event; the 5 s give-up that
  freed live handles is gone. `TcpClient` viability, better-path and path
  callbacks follow the same rule.
- Listener accept state is lock protected. Pending connections are no longer
  over-released, accept loops no longer end on semaphore drift, and a
  connection that fails its handshake is dropped instead of ending `accept()`.
- `ConnectionGroup` teardown waits for the real `cancelled` state instead of
  freeing the group on a stale semaphore signal. Browsers, path monitors,
  Bonjour advertisers and Ethernet channels use the same lifecycle, which
  fixes use-after-free on their teardown too.
- Establishment and data-transfer reports, interface lookups and the test
  error helper no longer signal a released semaphore after a timeout.
- `FramerDefinition` owns its factory through Network.framework, so parameters
  re-wrapped from a definition keep the factory alive (it used to be borrowed
  with `Arc::as_ptr` and could dangle).
- `WebSocket::connect` builds its URL safely. The host must be a DNS name or
  an IP address (IPv6 is bracketed, with an optional zone), so userinfo such
  as `user@` and other authority-changing characters are rejected; the path
  must start with `/` and may not contain `\`, `#`, whitespace or control
  characters, and non-ASCII bytes are percent-encoded. Long URLs are no longer
  silently truncated at 2048 bytes.
- The six browser and Ethernet channel callback trampolines that lacked panic
  guards now contain panics instead of unwinding into C.
- `ProxyConfig::set_credentials` zeroizes the crate's temporary copy
  of the proxy password.
- Two handles can no longer mutate one unsynchronized Network.framework
  object from different threads. Disassembly shows that parameters, protocol
  stacks, most protocol options, group, browse and advertise descriptors, proxy and
  relay configurations, WebSocket responses and content contexts do not lock
  their setters, so retain-sharing `Clone`s and accessors that returned the
  live object raced inside the framework. See **Changed** for the new API.
- `Sync` is removed from mutable Network.framework wrappers whose clones share
  one object: `AdvertiseDescriptor`, `BrowseDescriptor`, `ConnectionGroupDescriptor`,
  `ContentContext`, `FramerMessage`, `ProtocolMetadata`, `ProtocolOptions`,
  `ProtocolStack`, `ProxyConfig`, `QuicMetadata`, `QuicOptions` and
  `SecurityProtocolOptions` (**breaking**).

### Fixed

- The async listener and group new-connection handlers no longer block the
  listener's serial queue while each connection becomes ready. Inbound
  connections finish their handshakes concurrently and are handed off when
  ready; up to 128 ready connections queue for `accept()`, which used to keep
  only the newest one.
- Async streams subscribe next to the callback handlers instead of replacing
  them, so a `ConnectionStateStream`, `ListenerEventStream`, `PathUpdateStream`
  or `BrowserEventStream` no longer disables `Browser`, `PathMonitor` or
  listener callbacks, and dropping a stream no longer stalls.
- `TcpListener::bind_tls` can complete handshakes: it takes the identity the
  server presents.
- QUIC connections and listeners use `nw_parameters_create_quic` instead of a
  DTLS plus QUIC stack.
- Framer `deliver` calls and every WebSocket receive no longer leak retained
  objects (`nw_protocol_copy_ws_definition` was never released).
- WebSocket client-request and pong handler contexts are released when
  Network.framework drops the handler, and clearing a pong handler no longer
  leaves a block that calls a null callback.
- `ConnectionGroup::reinsert_extracted_connection` no longer reports failure
  on success.
- A connection reinserted into its `ConnectionGroup` no longer leaks its shim
  handle, callback contexts and closures when the group takes over its
  handlers. Connection handler blocks now own a reference to the handle that
  is released when Network.framework drops the blocks; the shim removes its
  handlers before reinsertion and after the final `cancelled` event.
- Browser handlers are installed before the browser starts, as the SDK
  requires.
- A connection that Network.framework reports as waiting with an error
  (connection refused, failed TLS trust) fails at once instead of after the
  30 s connect timeout.
- A malformed PKCS#12 blob is reported as an error instead of raising an
  uncaught Objective-C exception.

### Changed

- **Breaking:** `TcpListener::bind_tls(port)` is now
  `bind_tls(port, &TlsIdentity)` and requires TLS 1.2 or newer.
- **Breaking:** `ConnectionGroup::set_receive_handler` and
  `set_new_connection_handler` return `Result` and fail with
  `NetworkError::InvalidArgument` once the group has started.
- **Breaking:** `NetworkError` has new variants `MessageTooLarge { size, limit }`,
  `Unsupported(String)` and `Security(i32)`.
- **Breaking:** `UdpClient::receive`, `UdpClient::receive_with_context` and
  `WebSocket::receive` return `NetworkError::MessageTooLarge` for a message
  longer than `max_len` (the message is consumed) instead of truncating it or
  returning it in pieces.
- **Breaking:** UDP, WebSocket and QUIC connect timeouts are reported as
  `NetworkError::Timeout` instead of `ConnectFailed`.
- **Breaking:** `WebSocket::connect` returns `NetworkError::InvalidArgument`
  for hosts and paths that could change the URL's authority.
- **Breaking:** while a `ListenerEventStream` exists, ready inbound
  connections go to the stream instead of to `TcpListener::accept`.
- **Breaking:** `TcpListener::accept` never returns a connection that failed
  its handshake; it returns `NetworkError::Cancelled` once the listener is
  closed and no ready connection is left.
- **Breaking:** `TcpListener::set_new_connection_group_handler` is removed.
  It installed the group handler after the listener had started and next to
  the new-connection handler, both of which the SDK forbids.
  `TcpListener::bind_with_group_handler(port, &parameters, handler)` installs
  it before the listener starts, and `accept()` on such a listener returns
  `NetworkError::InvalidArgument`.
- **Breaking:** `PathMonitor::prohibit_interface_type` is removed: called
  after the monitor started, it had no effect. `PathMonitorBuilder` applies
  prohibited interface types, and the interface-type or Ethernet-channel
  scope, before the monitor starts.
- **Breaking:** `Clone` is removed from `ProtocolOptions`, `QuicOptions`,
  `SecurityProtocolOptions`, `ProtocolStack`, `ConnectionGroupDescriptor`,
  `BrowseDescriptor`, `AdvertiseDescriptor`, `ProxyConfig`, `RelayHop`,
  `ResolverConfig`, `WsResponse` and `ContentContext`, and therefore from
  `ReceivedContent` and `ConnectionGroupMessage`.
- **Breaking:** `ConnectionParameters::default_protocol_stack` takes
  `&mut self` and returns a `ProtocolStack<'_>` borrowed from the parameters.
  Its `application_protocols`, `transport_protocol` and `internet_protocol`
  return `ReadOnly` views.
- **Breaking:** `ConnectionParameters::prepend_application_protocol` and
  `ProtocolStack::set_transport_protocol` take the options by value.
- **Breaking:** `ConnectionGroup::new`, `start_browser_with_descriptor` and
  `start_browser_results_with_descriptor` take the descriptor by value;
  `ConnectionGroup::descriptor`, `Browser::browse_descriptor` and
  `BrowseResultsBrowser::browse_descriptor` return `ReadOnly` views.
- **Breaking:** `UrlSessionConfiguration::set_proxy_configurations` takes a
  `Vec<ProxyConfig>`, and `proxy_configurations` returns `ReadOnly` views.
- **Breaking:** `ProtocolOptions::tls_security_options` is replaced by
  `configure_tls_security(|security| ..)`, which rejects options that are not
  TLS options, and `QuicOptions::security_options` by
  `configure_security(|security| ..)`. `QuicOptions::protocol_options`
  returns `&ProtocolOptions`, and `ProtocolOptions` implements
  `From<QuicOptions>`.
- **Breaking:** `ContentContext::copy_antecedent` is replaced by
  `antecedent_identifier`.
- **Breaking:** `FramerContext::options` and
  `ProtocolMetadata::ws_server_response` return `ReadOnly` views.
- **Breaking:** `TcpClient::parameters`, `Browser::parameters`,
  `BrowseResultsBrowser::parameters`, `ConnectionGroup::parameters` and
  `FramerContext::parameters` return independent deep copies instead of the
  live object Network.framework hands out.
- `ProtocolMetadata`, `QuicMetadata`, `FramerMessage`, `TxtRecord`,
  `DataTransferReport` and `PrivacyContext` keep their `Clone`: the framework
  locks or serializes their setters (`TxtRecord` clones deeply).
- **Breaking:** `ConnectionGroup::extract_connection` takes
  `Option<&Endpoint>` and `Option<&ProtocolOptions>`: the SDK requires no
  endpoint for multiplex groups, so the old signature could not extract from
  them. A rejected reinsertion reports `NetworkError::InvalidArgument` with a
  descriptive message.
- **Breaking (`raw-ffi`):** the async helper shims (`nw_shim_*_set_*_handler`,
  `nw_shim_*_drain_queue`) and `nw_shim_browser_start` are replaced by
  `nw_shim_*_subscribe_*` and `nw_shim_*_unsubscribe`, and the signatures of
  `nw_shim_tcp_receive`, `nw_shim_connection_receive_with_context`,
  `nw_shim_ws_connect`, `nw_shim_ws_receive`, `nw_shim_path_monitor_start*`,
  `nw_shim_browser_start*_with_descriptor`, `nw_shim_framer_definition_create`,
  `nw_shim_ws_metadata_set_pong_handler` and
  `nw_shim_ws_options_set_client_request_handler` changed.
  `nw_shim_path_monitor_start` takes the monitor scope and the prohibited
  interface types; `nw_shim_path_monitor_start_with_type`,
  `nw_shim_path_monitor_start_for_ethernet_channel`,
  `nw_shim_path_monitor_prohibit_interface_type` and
  `nw_shim_listener_subscribe_new_connection_group` are removed, and
  `nw_shim_listener_create_for_groups` is added.
- Dropping a connection, listener, group, browser or path monitor no longer
  waits for its queue to drain. No new callback starts after the drop, but one
  that is already running may finish afterwards. A `PathMonitor` cancel
  handler still runs when the monitor is dropped.
- Depends on `apple-cf` `>=0.11, <0.12` and `doom-fish-utils` `>=0.4.1, <0.5`,
  and declares `rust-version = "1.82"`.
- The README documents macOS 14 as the minimum, matching the Swift bridge.

### Added

- `tls` module: `TlsIdentity` (`from_pkcs12`, macOS 15 or later, and
  `from_sec_identity`), `TlsVersion`, `TlsPeer` and `certificate_sha256`.
- `SecurityProtocolOptions::set_local_identity`, `set_min_tls_version`,
  `set_max_tls_version`, `add_application_protocol`, `set_server_name`,
  `set_peer_authentication_required`, `set_verify_handler` and
  `pin_peer_certificate_sha256`; there is no accept-all verifier.
- `SecurityProtocolMetadata::negotiated_tls_version` and
  `negotiated_application_protocol`.
- `ConnectionParameters::tls_tcp_configured` and `quic_configured`.
- `TcpListener::bind_with_group_handler` and `PathMonitorBuilder`.
- `ReadOnly<'a, T>`, a borrowed handle that allows only `&T` access.
- `TcpListener::bind_loopback`, which listens on `127.0.0.1` only;
  `TcpListener::bind` is documented to listen on every interface.
- `TcpClient::receive_message`.
- Loopback regression tests for peer resets, failed TLS handshakes,
  simultaneous accepts, handler replacement, framer lifetimes, oversized
  datagrams, QUIC streams, connection groups, repeated create/drop, handler
  contexts released by dropped connections and group reinsertion.

## [0.13.3] - 2026-06-06

- Guarded the Network.framework callback trampolines against panics and pinned the FFI shim struct layouts.

## [0.13.2] - 2026-05-20

- Clippy hygiene sweep: cleared all `-D warnings` lints across the crate. No public API change.

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
