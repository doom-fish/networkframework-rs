# Network.framework coverage (v0.9.3)

Audited against the macOS 26.2 SDK headers under `Network.framework/Headers`.
Current audit status: **498 / 500 SDK symbols verified (99.60%)**. See `COVERAGE_AUDIT.md` for the full symbol-by-symbol table.

## Requested logical areas

| Area | Headers | Safe Rust surface | Example | Test | Status |
| --- | --- | --- | --- | --- | --- |
| Connection | `connection.h` | `TcpClient`, advanced lifecycle callbacks, report accessors | `01_get_example` | `connection_area_round_trip_exposes_metadata` | ✅ implemented |
| Listener | `listener.h` | `TcpListener`, direct/launchd/connection-backed constructors, limit + callback hooks | `01_get_example` | `listener_area_accepts_connections` | ✅ implemented |
| Browser | `browser.h`, `browse_descriptor.h`, `browse_result.h` | `Browser`, `BrowseResultsBrowser`, `BrowseDescriptor`, `BrowseResult`, `BrowseResultChange` | `04_bonjour` | `browser_area_descriptor_and_start` | ✅ implemented |
| Parameters | `parameters.h` | `ConnectionParameters`, `ParametersAttribution`, `custom_ip` | `02_tls_get` | `parameters_area_supports_policy_controls` | ✅ implemented |
| Endpoint | `endpoint.h` | `Endpoint`, `EndpointType` | `03_udp_and_path` | `endpoint_area_builds_common_endpoint_types` | ✅ implemented |
| Path | `path.h`, `path_monitor.h` | `Path`, `PathMonitor`, gateway enumeration, scoped monitors, cancel hooks | `03_udp_and_path` | `path_area_reports_connection_path`, `advanced_path_monitor_and_misc_area_smoke` | ✅ implemented |
| Framer | `framer_options.h` | `FramerDefinition`, `FramerOptions`, `FramerContext`, `FramerMessage`, object-value accessors | `framer_length_prefix` | `framer_area_round_trip` | ✅ implemented |
| Group | `group_descriptor.h`, `connection_group.h` | `ConnectionGroup`, `ConnectionGroupDescriptor`, extraction/reply helpers, new-connection hook | `connection_group` | `group_area_starts_and_cancels` | ✅ implemented |
| Protocol | `protocol_options.h`, `tcp_options.h`, `udp_options.h`, `tls_options.h`, `ip_options.h`, `ws_options.h`, `quic_options.h` | `ProtocolDefinition`, `ProtocolOptions`, `ProtocolMetadata`, `WsRequest`, `WsResponse` | `05_websocket` | `protocol_area_exposes_definitions_and_options` | ✅ implemented |
| ContentContext | `content_context.h` | `ContentContext`, `ReceivedContent`, protocol metadata enumeration | `content_context_overview` | `content_context_area_tracks_properties` | ✅ implemented |
| Resolver | `resolver_config.h` | `ResolverConfig` | `resolver_overview` | `resolver_area_builders_work` | ✅ implemented |
| Quic | `quic_options.h` | `QuicConnection`, `QuicOptions` | `quic_options` | `quic_area_exposes_transport_settings` | ✅ implemented |
| PrivacyContext / URLSession | `privacy_context.h`, `NSURLSession+Network.h` | `PrivacyContext`, `UrlSessionConfiguration`, `ProxyConfig`, `RelayHop` | `privacy_context_overview` | `privacy_context_area_supports_default_and_encrypted_resolution`, `proxy_config_area_tracks_domains_and_optional_relay` | ✅ implemented |
| AdvertiseDescriptor | `advertise_descriptor.h` | `AdvertiseDescriptor`, `Advertiser`, TXT-record object helpers, `advertise_with_descriptor` | `06_bonjour_advertise` | `advertise_descriptor_area_builds_and_advertises` | ✅ implemented |

## Remaining gaps

| SDK surface | Status | Reason |
| --- | --- | --- |
| `nw_error_copy_cf_error` | ⏭️ pending | `FrameworkError` exposes the Network.framework domain/code plus CFError-derived strings, but the raw `CFErrorRef` itself is not surfaced as a safe Rust type yet. |
| `nw_ws_metadata_set_pong_handler` | ⏭️ pending | WebSocket request/response objects, metadata, and option handlers are wrapped, but the metadata-level pong callback is still missing. |

## Notes

- Application-service, relay/OHTTP proxy, ultra-constrained-path, launchd-listener, and custom-IP helpers are now covered by the safe surface with runtime availability handling where required.
- Enable `raw-ffi` only when you need direct access to bridge symbols beyond the safe API.
