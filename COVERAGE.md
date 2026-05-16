# Network.framework coverage (v0.9.0)

Audited against the macOS 26.2 SDK headers under `Network.framework/Headers`.

## Requested logical areas

| Area | Headers | Safe Rust surface | Example | Test | Status |
| --- | --- | --- | --- | --- | --- |
| Connection | `connection.h` | `TcpClient`, `Connection` | `01_get_example` | `connection_area_round_trip_exposes_metadata` | ✅ implemented |
| Listener | `listener.h` | `TcpListener` | `01_get_example` | `listener_area_accepts_connections` | ✅ implemented |
| Browser | `browser.h`, `browse_descriptor.h`, `browse_result.h` | `Browser`, `BrowseDescriptor`, `start_browser_with_descriptor` | `04_bonjour` | `browser_area_descriptor_and_start` | ✅ implemented |
| Parameters | `parameters.h` | `ConnectionParameters`, `ParametersAttribution` | `02_tls_get` | `parameters_area_supports_policy_controls` | ✅ implemented |
| Endpoint | `endpoint.h` | `Endpoint`, `EndpointType` | `03_udp_and_path` | `endpoint_area_builds_common_endpoint_types` | ✅ implemented |
| Path | `path.h` | `Path`, `PathStatus`, `PathUnsatisfiedReason`, `LinkQuality` | `03_udp_and_path` | `path_area_reports_connection_path` | ✅ implemented |
| Framer | `framer_options.h` | `FramerDefinition`, `FramerOptions`, `FramerContext`, `FramerMessage` | `framer_length_prefix` | `framer_area_round_trip` | ✅ implemented |
| Group | `group_descriptor.h`, `connection_group.h` | `ConnectionGroup`, `ConnectionGroupDescriptor`, `Group*` aliases | `connection_group` | `group_area_starts_and_cancels` | ✅ implemented |
| Protocol | `protocol_options.h`, `tcp_options.h`, `udp_options.h`, `tls_options.h`, `ip_options.h`, `ws_options.h`, `quic_options.h` | `ProtocolDefinition`, `ProtocolOptions` | `05_websocket` | `protocol_area_exposes_definitions_and_options` | ✅ implemented |
| ContentContext | `content_context.h` | `ContentContext`, `ReceivedContent` | `content_context_overview` | `content_context_area_tracks_properties` | ✅ implemented |
| Resolver | `resolver_config.h` | `ResolverConfig` | `resolver_overview` | `resolver_area_builders_work` | ✅ implemented |
| Quic | `quic_options.h` | `QuicConnection`, `QuicOptions` | `quic_options` | `quic_area_exposes_transport_settings` | ✅ implemented |
| PrivacyContext | `privacy_context.h` | `PrivacyContext` | `privacy_context_overview` | `privacy_context_area_supports_default_and_encrypted_resolution` | ✅ implemented |
| ProxyConfig | `proxy_config.h` | `ProxyConfig`, `RelayHop` | `privacy_context_overview` | `proxy_config_area_tracks_domains_and_optional_relay` | ✅ implemented |
| AdvertiseDescriptor | `advertise_descriptor.h` | `AdvertiseDescriptor`, `Advertiser`, `advertise_with_descriptor` | `06_bonjour_advertise` | `advertise_descriptor_area_builds_and_advertises` | ✅ implemented |

## Explicit skips and caveats

| SDK surface | Status | Reason |
| --- | --- | --- |
| `nw_parameters_create_custom_ip` | ⏭️ skipped | Entitlement-only custom IP parameters are intentionally not exposed as safe defaults; use `raw-ffi` if you need that exact symbol. |
| `nw_listener_create_with_launchd_key` | ⏭️ skipped | Launchd-managed listeners are deployment-specific and do not fit the crate's synchronous RAII listener model. |
| Application-service constructors | ✅ implemented | Safe wrappers are present; they return `NetworkError::InvalidArgument` when run on unsupported macOS versions. |
| Relay / Oblivious HTTP proxy constructors | ✅ implemented | Safe wrappers are present with runtime availability handling. |
| Ultra-constrained path and parameter properties | ✅ implemented | Safe getters/setters are present; effective values depend on the runtime OS support. |

## Raw access

Enable `raw-ffi` to reach the bridge symbols directly when the safe API intentionally leaves an SDK symbol out of the default surface.
