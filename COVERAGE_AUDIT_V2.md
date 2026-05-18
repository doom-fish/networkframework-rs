# networkframework-rs coverage audit v2 (vs MacOSX26.2.sdk)

SDK_PUBLIC_SYMBOLS: 500
VERIFIED: 500
GAPS: 0
EXEMPT: 0
COVERAGE_PCT: 100.00%

Methodology: Re-enumerated the macOS 26.2 Network.framework C public surface from 32 headers, including nw_* and sec_* functions, opaque types, and constants. Cross-referenced against the crate's shim layer and Rust safe API to identify verified symbols. No macOS-unavailable API_UNAVAILABLE(macos) symbols were included. Both v1 gaps have now been closed in v2: nw_error_copy_cf_error is now safely exposed via FrameworkError::cf_error() returning apple_cf::CFError, and nw_ws_metadata_set_pong_handler is now exposed via ProtocolMetadata::set_pong_handler().

## 🟢 VERIFIED

All 500 SDK symbols from v1 are now verified as implemented, including the two previously unimplemented symbols. Representative samples include:

| Symbol | Kind | Header | Wrapped by |
| --- | --- | --- | --- |
| `nw_advertise_descriptor_copy_txt_record_object` | function | `advertise_descriptor.h` | AdvertiseDescriptor, Advertiser, TxtRecord |
| `nw_advertise_descriptor_create_application_service` | function | `advertise_descriptor.h` | AdvertiseDescriptor, Advertiser |
| `nw_browser_create` | function | `browser.h` | Browser, BrowseDescriptor |
| `nw_connection_create` | function | `connection.h` | TcpClient, QuicConnection, Connection |
| `nw_connection_start` | function | `connection.h` | TcpClient, QuicConnection, Connection |
| `nw_endpoint_create_address` | function | `endpoint.h` | Endpoint, EndpointType |
| `nw_endpoint_create_host` | function | `endpoint.h` | Endpoint, EndpointType |
| `nw_error_copy_posix_error_code` | function | `error.h` | FrameworkError, ErrorDomain |
| `nw_error_copy_cf_error` | function | `error.h` | FrameworkError::cf_error() → apple_cf::CFError |
| `kNWErrorDomainDNS` | const | `error.h` | ErrorDomain, FrameworkError |
| `kNWErrorDomainPOSIX` | const | `error.h` | ErrorDomain, FrameworkError |
| `kNWErrorDomainTLS` | const | `error.h` | ErrorDomain, FrameworkError |
| `nw_parameters_create` | function | `parameters.h` | ConnectionParameters, ProtocolStack |
| `nw_parameters_create_secure_tcp` | function | `parameters.h` | ConnectionParameters, ProtocolStack |
| `nw_path_get_status` | function | `path.h` | Path, PathStatus |
| `nw_listener_create_with_port` | function | `listener.h` | TcpListener |
| `nw_listener_start` | function | `listener.h` | TcpListener |
| `nw_quic_create_options` | function | `quic_options.h` | QuicOptions, QuicConnection |
| `nw_quic_set_idle_timeout` | function | `quic_options.h` | QuicOptions, QuicConnection |
| `nw_tcp_create_options` | function | `tcp_options.h` | ProtocolOptions, TcpClient |
| `nw_tcp_options_set_no_delay` | function | `tcp_options.h` | ProtocolOptions, TcpClient |
| `nw_tls_create_options` | function | `tls_options.h` | SecurityProtocolOptions, ProtocolOptions |
| `nw_txt_record_apply` | function | `txt_record.h` | TxtRecord, AdvertiseDescriptor |
| `nw_ws_create_options` | function | `ws_options.h` | WebSocketOptions, WebSocket |
| `nw_ws_metadata_set_pong_handler` | function | `ws_options.h` | ProtocolMetadata::set_pong_handler() |
| `nw_release` | function | `nw_object.h` | All reference-counted types |
| `nw_retain` | function | `nw_object.h` | All reference-counted types |

(Complete verified list available in v1 COVERAGE_AUDIT.md; 500 total symbols now covered via safe Rust API or swift-bridge layer)

## 🟢 GAPS CLOSED

All gaps have been closed. No missing symbols remain.

## ⏭️ EXEMPT

No symbols are exempt. All 500 public macOS symbols in the SDK are now verified as implemented.

---

## Audit notes (v1 → v2 verification → v2 closure)

- **v1 metrics confirmed then improved**: Both v1 GAPS have been successfully closed in v2.
- **Gap 1 (nw_error_copy_cf_error)**: Now implemented. Added `FrameworkError::cf_error()` which safely wraps the underlying C function and returns an owned `apple_cf::cf::CFError` via `from_raw_retained`.
- **Gap 2 (nw_ws_metadata_set_pong_handler)**: Now implemented. Added `ProtocolMetadata::set_pong_handler()` for safe pong callbacks on WebSocket metadata.
- **No new gaps introduced**: All 498 previously verified symbols remain verified and 2 additional symbols are now verified.
- **No exemptions apply**: All 500 symbols are confirmed public, available on current macOS, and now fully covered.
- **Version bump**: Minor bump (0.12.2 → 0.13.0) for the new public APIs.
