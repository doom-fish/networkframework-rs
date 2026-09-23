# Network.framework coverage (v0.14.0)

`COVERAGE_AUDIT.md` lists the 500 functions, types and constants declared
under `Network.framework/Headers` in the macOS 26.2 SDK. 499 of them are
reachable through the safe API and one, `nw_protocol_metadata_copy_definition`,
only through the `raw-ffi` feature. In 0.14.0 the list was re-checked against
the macOS 26.5 SDK headers: every listed function is still declared and no
function is missing. The count is still not the whole networking surface:

- It leaves out the `sec_protocol_options_*` / `sec_protocol_metadata_*`
  functions from Security.framework that configure Network.framework's TLS and
  QUIC. Since 0.14.0 the essentials are wrapped (see the TLS row below); the
  rest are not.
- macOS 27 SDK additions such as `nw_tcp_set_max_pacing_rate` are not wrapped.

"Reachable" means a wrapper calls the symbol; it is not a per-symbol safety
review.

## Requested logical areas

| Area | Headers | Safe Rust surface | Example | Test | Status |
| --- | --- | --- | --- | --- | --- |
| Connection | `connection.h` | `TcpClient`, advanced lifecycle callbacks, report accessors | `01_get_example` | `connection_area_round_trip_exposes_metadata` | ✅ implemented |
| Listener | `listener.h` | `TcpListener` (`bind` on every interface, `bind_loopback`, `bind_tls`), direct/launchd/connection-backed constructors, limit + callback hooks | `01_get_example` | `listener_area_accepts_connections`, `accept_survives_resets_and_serves_simultaneous_clients` | ✅ implemented |
| Browser | `browser.h`, `browse_descriptor.h`, `browse_result.h` | `Browser`, `BrowseResultsBrowser`, `BrowseDescriptor`, `BrowseResult`, `BrowseResultChange` | `04_bonjour` | `browser_area_descriptor_and_start` | ✅ implemented |
| Parameters | `parameters.h` | `ConnectionParameters`, `ParametersAttribution`, `custom_ip` | `02_tls_get` | `parameters_area_supports_policy_controls` | ✅ implemented |
| Endpoint | `endpoint.h` | `Endpoint`, `EndpointType` | `03_udp_and_path` | `endpoint_area_builds_common_endpoint_types` | ✅ implemented |
| Path | `path.h`, `path_monitor.h` | `Path`, `PathMonitor`, gateway enumeration, scoped monitors, cancel hooks | `03_udp_and_path` | `path_area_reports_connection_path`, `advanced_path_monitor_and_misc_area_smoke` | ✅ implemented |
| Framer | `framer_options.h` | `FramerDefinition`, `FramerOptions`, `FramerContext`, `FramerMessage`, object-value accessors | `framer_length_prefix` | `framer_area_round_trip` | ✅ implemented |
| Group | `group_descriptor.h`, `connection_group.h` | `ConnectionGroup`, `ConnectionGroupDescriptor`, extraction/reply helpers, new-connection hook | `connection_group` | `group_area_builds_descriptors_and_drops_unstarted_groups`, `quic_multiplex_group_starts_rejects_late_handlers_and_cancels` (the multicast `group_area_starts_and_cancels` is ignored by default because it listens on every interface) | ✅ implemented |
| Protocol | `protocol_options.h`, `tcp_options.h`, `udp_options.h`, `tls_options.h`, `ip_options.h`, `ws_options.h`, `quic_options.h` | `ProtocolDefinition`, `ProtocolOptions`, `ProtocolMetadata`, `WsRequest`, `WsResponse` | `05_websocket` | `protocol_area_exposes_definitions_and_options` | ✅ implemented |
| ContentContext | `content_context.h` | `ContentContext`, `ReceivedContent`, protocol metadata enumeration | `content_context_overview` | `content_context_area_tracks_properties` | ✅ implemented |
| Resolver | `resolver_config.h` | `ResolverConfig` | `resolver_overview` | `resolver_area_builders_work` | ✅ implemented |
| Quic | `quic_options.h` | `QuicConnection`, `QuicOptions`, `ConnectionParameters::quic_configured` (built with `nw_parameters_create_quic`) | `quic_options` | `quic_area_exposes_transport_settings`, `quic_uses_real_quic_parameters` | ✅ implemented |
| TLS (Security) | `Security/SecProtocolOptions.h`, `Security/SecProtocolMetadata.h`, `Security/SecProtocolTypes.h` | `TlsIdentity`, `SecurityProtocolOptions` (local identity, TLS version range, ALPN, server name, peer authentication, verify handler, SHA-256 pinning), `SecurityProtocolMetadata` (negotiated version and ALPN), `ConnectionParameters::tls_tcp_configured` | — | `tls_listener_completes_pinned_handshakes_and_survives_failed_ones`, `pkcs12_import_rejects_garbage_and_wrong_passwords` | 🟡 essentials only; not part of the 500-symbol list |
| PrivacyContext / URLSession | `privacy_context.h`, `NSURLSession+Network.h` | `PrivacyContext`, `UrlSessionConfiguration`, `ProxyConfig`, `RelayHop` | `privacy_context_overview` | `privacy_context_area_supports_default_and_encrypted_resolution`, `proxy_config_area_tracks_domains_and_optional_relay` | ✅ implemented |
| AdvertiseDescriptor | `advertise_descriptor.h` | `AdvertiseDescriptor`, `Advertiser`, TXT-record object helpers, `advertise_with_descriptor` | `06_bonjour_advertise` | `advertise_descriptor_area_builds_descriptors` (`advertise_descriptor_area_builds_and_advertises` is ignored by default because advertising listens on every interface) | ✅ implemented |

## Remaining gaps

- Within the 500-symbol list: `nw_protocol_metadata_copy_definition` has no
  safe wrapper (only `raw-ffi`).
- Outside it: the `sec_protocol_*` functions beyond the essentials listed
  above (cipher suites, pre-shared keys, session tickets and resumption, OCSP
  and SCT, key update, challenge blocks and the like).
- macOS 27 SDK additions, such as `nw_tcp_set_max_pacing_rate`.

## Notes

- Application-service, relay/OHTTP proxy, ultra-constrained-path, launchd-listener, and custom-IP helpers are now covered by the safe surface with runtime availability handling where required.
- Enable `raw-ffi` only when you need direct access to bridge symbols beyond the safe API.
