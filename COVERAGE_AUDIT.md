# networkframework-rs coverage audit (vs MacOSX26.2.sdk)

SDK_PUBLIC_SYMBOLS: 500
VERIFIED: 498
GAPS: 2
EXEMPT: 0
COVERAGE_PCT: 99.60%

Methodology: enumerated the macOS 26.2 Network.framework C surface from headers, then marked symbols as verified when they are reachable through the crate's safe Rust API or (where noted) the public `raw-ffi` shim bridge. No macOS-deprecated Network.framework symbols were present in this SDK, so the exempt set is empty.

## 🟢 VERIFIED
| Symbol | Kind | Header | Wrapped by |
| --- | --- | --- | --- |
| `NSURLSessionConfiguration.proxyConfigurations` | property | `NSURLSession+Network.h` | UrlSessionConfiguration, ProxyConfig |
| `_nw_data_transfer_report_all_paths` | const | `connection_report.h` | DataTransferReport, EstablishmentReport, ResolutionReport, TcpClient, QuicConnection |
| `kNWErrorDomainDNS` | const | `error.h` | ErrorDomain, FrameworkError |
| `kNWErrorDomainPOSIX` | const | `error.h` | ErrorDomain, FrameworkError |
| `kNWErrorDomainTLS` | const | `error.h` | ErrorDomain, FrameworkError |
| `kNWErrorDomainWiFiAware` | const | `error.h` | ErrorDomain, FrameworkError |
| `nw_advertise_descriptor_copy_txt_record_object` | function | `advertise_descriptor.h` | AdvertiseDescriptor, Advertiser, TxtRecord, advertise_with_descriptor |
| `nw_advertise_descriptor_create_application_service` | function | `advertise_descriptor.h` | AdvertiseDescriptor, Advertiser, advertise_with_descriptor |
| `nw_advertise_descriptor_create_bonjour_service` | function | `advertise_descriptor.h` | AdvertiseDescriptor, Advertiser, advertise_with_descriptor, Browser, BrowseDescriptor, start_browser_with_descriptor |
| `nw_advertise_descriptor_get_application_service_name` | function | `advertise_descriptor.h` | AdvertiseDescriptor, Advertiser, advertise_with_descriptor |
| `nw_advertise_descriptor_get_no_auto_rename` | function | `advertise_descriptor.h` | AdvertiseDescriptor, Advertiser, advertise_with_descriptor |
| `nw_advertise_descriptor_set_no_auto_rename` | function | `advertise_descriptor.h` | AdvertiseDescriptor, Advertiser, advertise_with_descriptor |
| `nw_advertise_descriptor_set_txt_record` | function | `advertise_descriptor.h` | AdvertiseDescriptor, Advertiser, advertise_with_descriptor |
| `nw_advertise_descriptor_set_txt_record_object` | function | `advertise_descriptor.h` | AdvertiseDescriptor, Advertiser, TxtRecord, advertise_with_descriptor |
| `nw_advertise_descriptor_t` | type | `advertise_descriptor.h` | AdvertiseDescriptor, Advertiser, advertise_with_descriptor, Browser, BrowseDescriptor, start_browser_with_descriptor |
| `nw_browse_descriptor_create_application_service` | function | `browse_descriptor.h` | Browser, BrowseDescriptor, start_browser_with_descriptor |
| `nw_browse_descriptor_create_bonjour_service` | function | `browse_descriptor.h` | Browser, BrowseDescriptor, start_browser_with_descriptor |
| `nw_browse_descriptor_get_application_service_name` | function | `browse_descriptor.h` | Browser, BrowseDescriptor, start_browser_with_descriptor |
| `nw_browse_descriptor_get_bonjour_service_domain` | function | `browse_descriptor.h` | Browser, BrowseDescriptor, start_browser_with_descriptor |
| `nw_browse_descriptor_get_bonjour_service_type` | function | `browse_descriptor.h` | Browser, BrowseDescriptor, start_browser_with_descriptor |
| `nw_browse_descriptor_get_include_txt_record` | function | `browse_descriptor.h` | Browser, BrowseDescriptor, start_browser_with_descriptor |
| `nw_browse_descriptor_set_include_txt_record` | function | `browse_descriptor.h` | Browser, BrowseDescriptor, start_browser_with_descriptor |
| `nw_browse_descriptor_t` | type | `browse_descriptor.h` | Browser, BrowseDescriptor, start_browser_with_descriptor |
| `nw_browse_result_copy_endpoint` | function | `browse_result.h` | BrowseResult, BrowseResultChange, BrowseResultsBrowser, Browser, start_browser_results_with_descriptor |
| `nw_browse_result_copy_txt_record_object` | function | `browse_result.h` | BrowseResult, BrowseResultChange, BrowseResultsBrowser, Browser, start_browser_results_with_descriptor |
| `nw_browse_result_enumerate_interfaces` | function | `browse_result.h` | BrowseResult, BrowseResultChange, BrowseResultsBrowser, Browser, start_browser_results_with_descriptor |
| `nw_browse_result_get_changes` | function | `browse_result.h` | BrowseResult, BrowseResultChange, BrowseResultsBrowser, Browser, start_browser_results_with_descriptor |
| `nw_browse_result_get_interfaces_count` | function | `browse_result.h` | BrowseResult, BrowseResultChange, BrowseResultsBrowser, Browser, start_browser_results_with_descriptor |
| `nw_browse_result_t` | type | `browse_result.h` | BrowseResult, BrowseResultChange, BrowseResultsBrowser, Browser, start_browser_results_with_descriptor |
| `nw_browser_cancel` | function | `browser.h` | Browser, BrowseDescriptor, start_browser_with_descriptor |
| `nw_browser_copy_browse_descriptor` | function | `browser.h` | Browser, BrowseResultsBrowser, BrowserState, start_browser_with_descriptor, start_browser_results_with_descriptor |
| `nw_browser_copy_parameters` | function | `browser.h` | Browser, BrowseResultsBrowser, BrowserState, start_browser_with_descriptor, start_browser_results_with_descriptor |
| `nw_browser_create` | function | `browser.h` | Browser, BrowseDescriptor, start_browser_with_descriptor |
| `nw_browser_set_browse_results_changed_handler` | function | `browser.h` | BrowseResult, BrowseResultChange, BrowseResultsBrowser, Browser, start_browser_results_with_descriptor |
| `nw_browser_set_queue` | function | `browser.h` | Browser, BrowseDescriptor, start_browser_with_descriptor |
| `nw_browser_set_state_changed_handler` | function | `browser.h` | Browser, BrowseResultsBrowser, BrowserState, start_browser_with_descriptor, start_browser_results_with_descriptor |
| `nw_browser_start` | function | `browser.h` | Browser, BrowseDescriptor, start_browser_with_descriptor |
| `nw_browser_state_t` | enum | `browser.h` | Browser, BrowseResultsBrowser, BrowserState, start_browser_with_descriptor, start_browser_results_with_descriptor |
| `nw_browser_t` | type | `browser.h` | Browser, BrowseDescriptor, start_browser_with_descriptor |
| `nw_connection_access_establishment_report` | function | `connection_report.h` | DataTransferReport, EstablishmentReport, ResolutionReport, TcpClient, QuicConnection |
| `nw_connection_batch` | function | `connection.h` | TcpClient, Path, ProtocolDefinition, ProtocolMetadata |
| `nw_connection_cancel` | function | `connection.h` | AdvertiseDescriptor, Advertiser, advertise_with_descriptor, Browser, BrowseDescriptor, start_browser_with_descriptor, TcpListener |
| `nw_connection_cancel_current_endpoint` | function | `connection.h` | TcpClient, Path, ProtocolDefinition, ProtocolMetadata |
| `nw_connection_copy_current_path` | function | `connection.h` | TcpClient |
| `nw_connection_copy_description` | function | `connection.h` | TcpClient, Path, ProtocolDefinition, ProtocolMetadata |
| `nw_connection_copy_endpoint` | function | `connection.h` | TcpClient |
| `nw_connection_copy_parameters` | function | `connection.h` | TcpClient |
| `nw_connection_copy_protocol_metadata` | function | `connection.h` | TcpClient, Path, ProtocolDefinition, ProtocolMetadata |
| `nw_connection_create` | function | `connection.h` | QuicConnection, QuicOptions, TcpClient, UdpClient, WebSocket |
| `nw_connection_create_new_data_transfer_report` | function | `connection_report.h` | DataTransferReport, EstablishmentReport, ResolutionReport, TcpClient, QuicConnection |
| `nw_connection_force_cancel` | function | `connection.h` | TcpClient, Path, ProtocolDefinition, ProtocolMetadata |
| `nw_connection_get_maximum_datagram_size` | function | `connection.h` | TcpClient, Path, ProtocolDefinition, ProtocolMetadata |
| `nw_connection_group_cancel` | function | `connection_group.h` | ConnectionGroup, ConnectionGroupDescriptor |
| `nw_connection_group_copy_descriptor` | function | `connection_group.h` | ConnectionGroup, ConnectionGroupDescriptor, ContentContext, Endpoint, Path, ProtocolDefinition, ProtocolMetadata, TcpClient |
| `nw_connection_group_copy_local_endpoint_for_message` | function | `connection_group.h` | ConnectionGroup, ConnectionGroupDescriptor, ContentContext, Endpoint, Path, ProtocolDefinition, ProtocolMetadata, TcpClient |
| `nw_connection_group_copy_parameters` | function | `connection_group.h` | ConnectionGroup, ConnectionGroupDescriptor, ContentContext, Endpoint, Path, ProtocolDefinition, ProtocolMetadata, TcpClient |
| `nw_connection_group_copy_path_for_message` | function | `connection_group.h` | ConnectionGroup, ConnectionGroupDescriptor, ContentContext, Endpoint, Path, ProtocolDefinition, ProtocolMetadata, TcpClient |
| `nw_connection_group_copy_protocol_metadata` | function | `connection_group.h` | ConnectionGroup, ConnectionGroupDescriptor, ContentContext, Endpoint, Path, ProtocolDefinition, ProtocolMetadata, TcpClient |
| `nw_connection_group_copy_protocol_metadata_for_message` | function | `connection_group.h` | ConnectionGroup, ConnectionGroupDescriptor, ContentContext, Endpoint, Path, ProtocolDefinition, ProtocolMetadata, TcpClient |
| `nw_connection_group_copy_remote_endpoint_for_message` | function | `connection_group.h` | ConnectionGroup, ConnectionGroupDescriptor, ContentContext, Endpoint, Path, ProtocolDefinition, ProtocolMetadata, TcpClient |
| `nw_connection_group_create` | function | `connection_group.h` | ConnectionGroup, ConnectionGroupDescriptor |
| `nw_connection_group_extract_connection` | function | `connection_group.h` | ConnectionGroup, ConnectionGroupDescriptor, ContentContext, Endpoint, Path, ProtocolDefinition, ProtocolMetadata, TcpClient |
| `nw_connection_group_extract_connection_for_message` | function | `connection_group.h` | ConnectionGroup, ConnectionGroupDescriptor, ContentContext, Endpoint, Path, ProtocolDefinition, ProtocolMetadata, TcpClient |
| `nw_connection_group_reinsert_extracted_connection` | function | `connection_group.h` | ConnectionGroup, ConnectionGroupDescriptor, ContentContext, Endpoint, Path, ProtocolDefinition, ProtocolMetadata, TcpClient |
| `nw_connection_group_reply` | function | `connection_group.h` | ConnectionGroup, ConnectionGroupDescriptor, ContentContext, Endpoint, Path, ProtocolDefinition, ProtocolMetadata, TcpClient |
| `nw_connection_group_send_message` | function | `connection_group.h` | ConnectionGroup, ConnectionGroupDescriptor |
| `nw_connection_group_set_new_connection_handler` | function | `connection_group.h` | ConnectionGroup, ConnectionGroupDescriptor, ContentContext, Endpoint, Path, ProtocolDefinition, ProtocolMetadata, TcpClient |
| `nw_connection_group_set_queue` | function | `connection_group.h` | ConnectionGroup, ConnectionGroupDescriptor |
| `nw_connection_group_set_receive_handler` | function | `connection_group.h` | ConnectionGroup, ConnectionGroupDescriptor |
| `nw_connection_group_set_state_changed_handler` | function | `connection_group.h` | ConnectionGroup, ConnectionGroupDescriptor |
| `nw_connection_group_start` | function | `connection_group.h` | ConnectionGroup, ConnectionGroupDescriptor |
| `nw_connection_group_state_t` | enum | `connection_group.h` | ConnectionGroup, ConnectionGroupDescriptor |
| `nw_connection_group_t` | type | `connection_group.h` | ConnectionGroup, ConnectionGroupDescriptor |
| `nw_connection_receive` | function | `connection.h` | QuicConnection, QuicOptions, TcpClient, UdpClient |
| `nw_connection_receive_message` | function | `connection.h` | WebSocket |
| `nw_connection_restart` | function | `connection.h` | TcpClient, Path, ProtocolDefinition, ProtocolMetadata |
| `nw_connection_send` | function | `connection.h` | QuicConnection, QuicOptions, TcpClient, UdpClient, WebSocket |
| `nw_connection_set_better_path_available_handler` | function | `connection.h` | TcpClient, Path, ProtocolDefinition, ProtocolMetadata |
| `nw_connection_set_path_changed_handler` | function | `connection.h` | TcpClient, Path, ProtocolDefinition, ProtocolMetadata |
| `nw_connection_set_queue` | function | `connection.h` | QuicConnection, QuicOptions, TcpClient, TcpListener, UdpClient, WebSocket |
| `nw_connection_set_state_changed_handler` | function | `connection.h` | QuicConnection, QuicOptions, TcpClient, TcpListener, UdpClient, WebSocket |
| `nw_connection_set_viability_changed_handler` | function | `connection.h` | TcpClient, Path, ProtocolDefinition, ProtocolMetadata |
| `nw_connection_start` | function | `connection.h` | QuicConnection, QuicOptions, TcpClient, TcpListener, UdpClient, WebSocket |
| `nw_connection_state_t` | enum | `connection.h` | QuicConnection, QuicOptions, TcpClient, TcpListener, UdpClient, WebSocket |
| `nw_connection_t` | type | `connection.h` | AdvertiseDescriptor, Advertiser, advertise_with_descriptor, Browser, BrowseDescriptor, start_browser_with_descriptor, QuicConnection, QuicOptions, TcpClient, TcpListener, UdpClient, WebSocket |
| `nw_content_context_copy_antecedent` | function | `content_context.h` | ContentContext, ReceivedContent |
| `nw_content_context_copy_protocol_metadata` | function | `content_context.h` | ContentContext, ReceivedContent, WebSocket |
| `nw_content_context_create` | function | `content_context.h` | ContentContext, ReceivedContent, WebSocket |
| `nw_content_context_foreach_protocol_metadata` | function | `content_context.h` | ContentContext, ProtocolDefinition, ProtocolMetadata |
| `nw_content_context_get_expiration_milliseconds` | function | `content_context.h` | ContentContext, ReceivedContent |
| `nw_content_context_get_identifier` | function | `content_context.h` | ContentContext, ReceivedContent |
| `nw_content_context_get_is_final` | function | `content_context.h` | ContentContext, ReceivedContent |
| `nw_content_context_get_relative_priority` | function | `content_context.h` | ContentContext, ReceivedContent |
| `nw_content_context_set_antecedent` | function | `content_context.h` | ContentContext, ReceivedContent |
| `nw_content_context_set_expiration_milliseconds` | function | `content_context.h` | ContentContext, ReceivedContent |
| `nw_content_context_set_is_final` | function | `content_context.h` | ContentContext, ReceivedContent |
| `nw_content_context_set_metadata_for_protocol` | function | `content_context.h` | ContentContext, ReceivedContent, WebSocket |
| `nw_content_context_set_relative_priority` | function | `content_context.h` | ContentContext, ReceivedContent |
| `nw_content_context_t` | type | `content_context.h` | ConnectionGroup, ConnectionGroupDescriptor, ContentContext, ReceivedContent, QuicConnection, QuicOptions, TcpClient, UdpClient, WebSocket |
| `nw_data_transfer_report_collect` | function | `connection_report.h` | DataTransferReport, EstablishmentReport, ResolutionReport, TcpClient, QuicConnection |
| `nw_data_transfer_report_copy_path_interface` | function | `connection_report.h` | DataTransferReport, EstablishmentReport, ResolutionReport, TcpClient, QuicConnection |
| `nw_data_transfer_report_get_duration_milliseconds` | function | `connection_report.h` | DataTransferReport, EstablishmentReport, ResolutionReport, TcpClient, QuicConnection |
| `nw_data_transfer_report_get_path_count` | function | `connection_report.h` | DataTransferReport, EstablishmentReport, ResolutionReport, TcpClient, QuicConnection |
| `nw_data_transfer_report_get_path_radio_type` | function | `connection_report.h` | DataTransferReport, EstablishmentReport, ResolutionReport, TcpClient, QuicConnection |
| `nw_data_transfer_report_get_received_application_byte_count` | function | `connection_report.h` | DataTransferReport, EstablishmentReport, ResolutionReport, TcpClient, QuicConnection |
| `nw_data_transfer_report_get_received_ip_packet_count` | function | `connection_report.h` | DataTransferReport, EstablishmentReport, ResolutionReport, TcpClient, QuicConnection |
| `nw_data_transfer_report_get_received_transport_byte_count` | function | `connection_report.h` | DataTransferReport, EstablishmentReport, ResolutionReport, TcpClient, QuicConnection |
| `nw_data_transfer_report_get_received_transport_duplicate_byte_count` | function | `connection_report.h` | DataTransferReport, EstablishmentReport, ResolutionReport, TcpClient, QuicConnection |
| `nw_data_transfer_report_get_received_transport_out_of_order_byte_count` | function | `connection_report.h` | DataTransferReport, EstablishmentReport, ResolutionReport, TcpClient, QuicConnection |
| `nw_data_transfer_report_get_sent_application_byte_count` | function | `connection_report.h` | DataTransferReport, EstablishmentReport, ResolutionReport, TcpClient, QuicConnection |
| `nw_data_transfer_report_get_sent_ip_packet_count` | function | `connection_report.h` | DataTransferReport, EstablishmentReport, ResolutionReport, TcpClient, QuicConnection |
| `nw_data_transfer_report_get_sent_transport_byte_count` | function | `connection_report.h` | DataTransferReport, EstablishmentReport, ResolutionReport, TcpClient, QuicConnection |
| `nw_data_transfer_report_get_sent_transport_retransmitted_byte_count` | function | `connection_report.h` | DataTransferReport, EstablishmentReport, ResolutionReport, TcpClient, QuicConnection |
| `nw_data_transfer_report_get_state` | function | `connection_report.h` | DataTransferReport, EstablishmentReport, ResolutionReport, TcpClient, QuicConnection |
| `nw_data_transfer_report_get_transport_minimum_rtt_milliseconds` | function | `connection_report.h` | DataTransferReport, EstablishmentReport, ResolutionReport, TcpClient, QuicConnection |
| `nw_data_transfer_report_get_transport_rtt_variance` | function | `connection_report.h` | DataTransferReport, EstablishmentReport, ResolutionReport, TcpClient, QuicConnection |
| `nw_data_transfer_report_get_transport_smoothed_rtt_milliseconds` | function | `connection_report.h` | DataTransferReport, EstablishmentReport, ResolutionReport, TcpClient, QuicConnection |
| `nw_data_transfer_report_state_t` | enum | `connection_report.h` | DataTransferReport, EstablishmentReport, ResolutionReport, TcpClient, QuicConnection |
| `nw_data_transfer_report_t` | type | `connection_report.h` | DataTransferReport, EstablishmentReport, ResolutionReport, TcpClient, QuicConnection |
| `nw_endpoint_copy_address_string` | function | `endpoint.h` | Endpoint |
| `nw_endpoint_copy_port_string` | function | `endpoint.h` | Endpoint |
| `nw_endpoint_copy_txt_record` | function | `endpoint.h` | Endpoint, TxtRecord |
| `nw_endpoint_create_address` | function | `endpoint.h` | ConnectionGroup, ConnectionGroupDescriptor, Endpoint |
| `nw_endpoint_create_bonjour_service` | function | `endpoint.h` | Endpoint |
| `nw_endpoint_create_host` | function | `endpoint.h` | ConnectionGroup, ConnectionGroupDescriptor, Endpoint, PrivacyContext, ProxyConfig, RelayHop, ResolverConfig, QuicConnection, QuicOptions, TcpClient, UdpClient |
| `nw_endpoint_create_url` | function | `endpoint.h` | Endpoint, PrivacyContext, ProxyConfig, RelayHop, ResolverConfig, WebSocket |
| `nw_endpoint_get_address` | function | `endpoint.h` | Endpoint, TxtRecord |
| `nw_endpoint_get_bonjour_service_domain` | function | `endpoint.h` | Browser, BrowseDescriptor, start_browser_with_descriptor, Endpoint |
| `nw_endpoint_get_bonjour_service_name` | function | `endpoint.h` | Browser, BrowseDescriptor, start_browser_with_descriptor, Endpoint |
| `nw_endpoint_get_bonjour_service_type` | function | `endpoint.h` | Browser, BrowseDescriptor, start_browser_with_descriptor, Endpoint |
| `nw_endpoint_get_hostname` | function | `endpoint.h` | Browser, BrowseDescriptor, start_browser_with_descriptor, Endpoint |
| `nw_endpoint_get_port` | function | `endpoint.h` | Endpoint |
| `nw_endpoint_get_signature` | function | `endpoint.h` | Endpoint |
| `nw_endpoint_get_type` | function | `endpoint.h` | Endpoint |
| `nw_endpoint_get_url` | function | `endpoint.h` | Endpoint |
| `nw_endpoint_type_t` | enum | `endpoint.h` | Endpoint |
| `nw_error_domain_t` | enum | `error.h` | ErrorDomain, FrameworkError |
| `nw_error_get_error_code` | function | `error.h` | ErrorDomain, FrameworkError |
| `nw_error_get_error_domain` | function | `error.h` | ErrorDomain, FrameworkError |
| `nw_error_t` | type | `error.h` | AdvertiseDescriptor, Advertiser, advertise_with_descriptor, Browser, BrowseDescriptor, start_browser_with_descriptor, ConnectionGroup, ConnectionGroupDescriptor, QuicConnection, QuicOptions, TcpClient, TcpListener, UdpClient, WebSocket |
| `nw_establishment_report_copy_proxy_endpoint` | function | `connection_report.h` | DataTransferReport, EstablishmentReport, ResolutionReport, TcpClient, QuicConnection |
| `nw_establishment_report_enumerate_protocols` | function | `connection_report.h` | DataTransferReport, EstablishmentReport, ResolutionReport, TcpClient, QuicConnection |
| `nw_establishment_report_enumerate_resolution_reports` | function | `connection_report.h` | DataTransferReport, EstablishmentReport, ResolutionReport, TcpClient, QuicConnection |
| `nw_establishment_report_enumerate_resolutions` | function | `connection_report.h` | DataTransferReport, EstablishmentReport, ResolutionReport, TcpClient, QuicConnection |
| `nw_establishment_report_get_attempt_started_after_milliseconds` | function | `connection_report.h` | DataTransferReport, EstablishmentReport, ResolutionReport, TcpClient, QuicConnection |
| `nw_establishment_report_get_duration_milliseconds` | function | `connection_report.h` | DataTransferReport, EstablishmentReport, ResolutionReport, TcpClient, QuicConnection |
| `nw_establishment_report_get_previous_attempt_count` | function | `connection_report.h` | DataTransferReport, EstablishmentReport, ResolutionReport, TcpClient, QuicConnection |
| `nw_establishment_report_get_proxy_configured` | function | `connection_report.h` | DataTransferReport, EstablishmentReport, ResolutionReport, TcpClient, QuicConnection |
| `nw_establishment_report_get_used_proxy` | function | `connection_report.h` | DataTransferReport, EstablishmentReport, ResolutionReport, TcpClient, QuicConnection |
| `nw_establishment_report_t` | type | `connection_report.h` | DataTransferReport, EstablishmentReport, ResolutionReport, TcpClient, QuicConnection |
| `nw_ethernet_channel_cancel` | function | `ethernet_channel.h` | EthernetChannel |
| `nw_ethernet_channel_create` | function | `ethernet_channel.h` | EthernetChannel |
| `nw_ethernet_channel_create_with_parameters` | function | `ethernet_channel.h` | EthernetChannel |
| `nw_ethernet_channel_get_maximum_payload_size` | function | `ethernet_channel.h` | EthernetChannel |
| `nw_ethernet_channel_send` | function | `ethernet_channel.h` | EthernetChannel |
| `nw_ethernet_channel_set_queue` | function | `ethernet_channel.h` | EthernetChannel |
| `nw_ethernet_channel_set_receive_handler` | function | `ethernet_channel.h` | EthernetChannel |
| `nw_ethernet_channel_set_state_changed_handler` | function | `ethernet_channel.h` | EthernetChannel |
| `nw_ethernet_channel_start` | function | `ethernet_channel.h` | EthernetChannel |
| `nw_ethernet_channel_t` | type | `ethernet_channel.h` | EthernetChannel |
| `nw_framer_async` | function | `framer_options.h` | FramerDefinition, FramerOptions, FramerContext, FramerMessage |
| `nw_framer_copy_local_endpoint` | function | `framer_options.h` | FramerContext, FramerMessage, FramerOptions, ConnectionParameters, Endpoint |
| `nw_framer_copy_options` | function | `framer_options.h` | FramerContext, FramerMessage, FramerOptions, ConnectionParameters, Endpoint |
| `nw_framer_copy_parameters` | function | `framer_options.h` | FramerContext, FramerMessage, FramerOptions, ConnectionParameters, Endpoint |
| `nw_framer_copy_remote_endpoint` | function | `framer_options.h` | FramerContext, FramerMessage, FramerOptions, ConnectionParameters, Endpoint |
| `nw_framer_create_definition` | function | `framer_options.h` | FramerDefinition, FramerOptions, FramerContext, FramerMessage |
| `nw_framer_create_options` | function | `framer_options.h` | FramerDefinition, FramerOptions, FramerContext, FramerMessage |
| `nw_framer_deliver_input` | function | `framer_options.h` | FramerDefinition, FramerOptions, FramerContext, FramerMessage |
| `nw_framer_deliver_input_no_copy` | function | `framer_options.h` | FramerDefinition, FramerOptions, FramerContext, FramerMessage |
| `nw_framer_mark_failed_with_error` | function | `framer_options.h` | FramerDefinition, FramerOptions, FramerContext, FramerMessage |
| `nw_framer_mark_ready` | function | `framer_options.h` | FramerDefinition, FramerOptions, FramerContext, FramerMessage |
| `nw_framer_message_access_value` | function | `framer_options.h` | FramerDefinition, FramerOptions, FramerContext, FramerMessage |
| `nw_framer_message_copy_object_value` | function | `framer_options.h` | FramerContext, FramerMessage, FramerOptions, ConnectionParameters, Endpoint |
| `nw_framer_message_create` | function | `framer_options.h` | FramerDefinition, FramerOptions, FramerContext, FramerMessage |
| `nw_framer_message_set_object_value` | function | `framer_options.h` | FramerContext, FramerMessage, FramerOptions, ConnectionParameters, Endpoint |
| `nw_framer_message_set_value` | function | `framer_options.h` | FramerDefinition, FramerOptions, FramerContext, FramerMessage |
| `nw_framer_options_copy_object_value` | function | `framer_options.h` | FramerContext, FramerMessage, FramerOptions, ConnectionParameters, Endpoint |
| `nw_framer_options_set_object_value` | function | `framer_options.h` | FramerContext, FramerMessage, FramerOptions, ConnectionParameters, Endpoint |
| `nw_framer_parse_input` | function | `framer_options.h` | FramerDefinition, FramerOptions, FramerContext, FramerMessage |
| `nw_framer_parse_output` | function | `framer_options.h` | FramerDefinition, FramerOptions, FramerContext, FramerMessage |
| `nw_framer_pass_through_input` | function | `framer_options.h` | FramerDefinition, FramerOptions, FramerContext, FramerMessage |
| `nw_framer_pass_through_output` | function | `framer_options.h` | FramerDefinition, FramerOptions, FramerContext, FramerMessage |
| `nw_framer_prepend_application_protocol` | function | `framer_options.h` | FramerDefinition, FramerOptions, FramerContext, FramerMessage |
| `nw_framer_protocol_create_message` | function | `framer_options.h` | FramerDefinition, FramerOptions, FramerContext, FramerMessage |
| `nw_framer_schedule_wakeup` | function | `framer_options.h` | FramerDefinition, FramerOptions, FramerContext, FramerMessage |
| `nw_framer_set_cleanup_handler` | function | `framer_options.h` | FramerDefinition, FramerOptions, FramerContext, FramerMessage |
| `nw_framer_set_input_handler` | function | `framer_options.h` | FramerDefinition, FramerOptions, FramerContext, FramerMessage |
| `nw_framer_set_output_handler` | function | `framer_options.h` | FramerDefinition, FramerOptions, FramerContext, FramerMessage |
| `nw_framer_set_stop_handler` | function | `framer_options.h` | FramerDefinition, FramerOptions, FramerContext, FramerMessage |
| `nw_framer_set_wakeup_handler` | function | `framer_options.h` | FramerDefinition, FramerOptions, FramerContext, FramerMessage |
| `nw_framer_start_result_t` | enum | `framer_options.h` | FramerDefinition, FramerOptions, FramerContext, FramerMessage |
| `nw_framer_t` | type | `framer_options.h` | FramerDefinition, FramerOptions, FramerContext, FramerMessage |
| `nw_framer_write_output` | function | `framer_options.h` | FramerDefinition, FramerOptions, FramerContext, FramerMessage |
| `nw_framer_write_output_data` | function | `framer_options.h` | FramerContext, FramerMessage, FramerOptions, ConnectionParameters, Endpoint |
| `nw_framer_write_output_no_copy` | function | `framer_options.h` | FramerDefinition, FramerOptions, FramerContext, FramerMessage |
| `nw_group_descriptor_add_endpoint` | function | `group_descriptor.h` | ConnectionGroup, ConnectionGroupDescriptor |
| `nw_group_descriptor_create_multicast` | function | `group_descriptor.h` | ConnectionGroup, ConnectionGroupDescriptor |
| `nw_group_descriptor_create_multiplex` | function | `group_descriptor.h` | ConnectionGroup, ConnectionGroupDescriptor |
| `nw_group_descriptor_enumerate_endpoints` | function | `group_descriptor.h` | ConnectionGroupDescriptor, Endpoint |
| `nw_group_descriptor_t` | type | `group_descriptor.h` | ConnectionGroup, ConnectionGroupDescriptor |
| `nw_interface_get_index` | function | `interface.h` | NetworkInterface, list_interfaces, Path |
| `nw_interface_get_name` | function | `interface.h` | NetworkInterface, list_interfaces, Path |
| `nw_interface_get_type` | function | `interface.h` | NetworkInterface, list_interfaces, Path |
| `nw_interface_radio_type_t` | enum | `interface.h` | DataTransferPathReport, InterfaceRadioType |
| `nw_interface_t` | type | `interface.h` | NetworkInterface, list_interfaces, Path |
| `nw_interface_type_t` | enum | `interface.h` | ConnectionParameters, Path |
| `nw_ip_create_metadata` | function | `ip_options.h` | ProtocolOptions, ProtocolMetadata, IpVersion, IpEcnFlag, IpLocalAddressPreference, ServiceClass |
| `nw_ip_ecn_flag_t` | enum | `ip_options.h` | ProtocolOptions, ProtocolMetadata, IpVersion, IpEcnFlag, IpLocalAddressPreference, ServiceClass |
| `nw_ip_local_address_preference_t` | enum | `ip_options.h` | ProtocolOptions, ProtocolMetadata, IpVersion, IpEcnFlag, IpLocalAddressPreference, ServiceClass |
| `nw_ip_metadata_get_ecn_flag` | function | `ip_options.h` | ProtocolOptions, ProtocolMetadata, IpVersion, IpEcnFlag, IpLocalAddressPreference, ServiceClass |
| `nw_ip_metadata_get_receive_time` | function | `ip_options.h` | ProtocolOptions, ProtocolMetadata, IpVersion, IpEcnFlag, IpLocalAddressPreference, ServiceClass |
| `nw_ip_metadata_get_service_class` | function | `ip_options.h` | ProtocolOptions, ProtocolMetadata, IpVersion, IpEcnFlag, IpLocalAddressPreference, ServiceClass |
| `nw_ip_metadata_set_ecn_flag` | function | `ip_options.h` | ProtocolOptions, ProtocolMetadata, IpVersion, IpEcnFlag, IpLocalAddressPreference, ServiceClass |
| `nw_ip_metadata_set_service_class` | function | `ip_options.h` | ProtocolOptions, ProtocolMetadata, IpVersion, IpEcnFlag, IpLocalAddressPreference, ServiceClass |
| `nw_ip_options_set_calculate_receive_time` | function | `ip_options.h` | ProtocolOptions, ProtocolMetadata, IpVersion, IpEcnFlag, IpLocalAddressPreference, ServiceClass |
| `nw_ip_options_set_disable_fragmentation` | function | `ip_options.h` | ProtocolOptions, ProtocolMetadata, IpVersion, IpEcnFlag, IpLocalAddressPreference, ServiceClass |
| `nw_ip_options_set_disable_multicast_loopback` | function | `ip_options.h` | ProtocolOptions, ProtocolMetadata, IpVersion, IpEcnFlag, IpLocalAddressPreference, ServiceClass |
| `nw_ip_options_set_hop_limit` | function | `ip_options.h` | ProtocolOptions, ProtocolMetadata, IpVersion, IpEcnFlag, IpLocalAddressPreference, ServiceClass |
| `nw_ip_options_set_local_address_preference` | function | `ip_options.h` | ProtocolOptions, ProtocolMetadata, IpVersion, IpEcnFlag, IpLocalAddressPreference, ServiceClass |
| `nw_ip_options_set_use_minimum_mtu` | function | `ip_options.h` | ProtocolOptions, ProtocolMetadata, IpVersion, IpEcnFlag, IpLocalAddressPreference, ServiceClass |
| `nw_ip_options_set_version` | function | `ip_options.h` | ProtocolOptions, ProtocolMetadata, IpVersion, IpEcnFlag, IpLocalAddressPreference, ServiceClass |
| `nw_ip_version_t` | enum | `ip_options.h` | ProtocolOptions, ProtocolMetadata, IpVersion, IpEcnFlag, IpLocalAddressPreference, ServiceClass |
| `nw_link_quality_t` | enum | `path.h` | Path |
| `nw_listener_cancel` | function | `listener.h` | AdvertiseDescriptor, Advertiser, advertise_with_descriptor, Browser, BrowseDescriptor, start_browser_with_descriptor |
| `nw_listener_create` | function | `listener.h` | TcpListener, TcpClient, ConnectionParameters, ConnectionGroup, Endpoint |
| `nw_listener_create_with_connection` | function | `listener.h` | TcpListener, TcpClient, ConnectionParameters, ConnectionGroup, Endpoint |
| `nw_listener_create_with_launchd_key` | function | `listener.h` | TcpListener, TcpClient, ConnectionParameters, ConnectionGroup, Endpoint |
| `nw_listener_create_with_port` | function | `listener.h` | AdvertiseDescriptor, Advertiser, advertise_with_descriptor, Browser, BrowseDescriptor, start_browser_with_descriptor, TcpListener |
| `nw_listener_get_new_connection_limit` | function | `listener.h` | TcpListener, TcpClient, ConnectionParameters, ConnectionGroup, Endpoint |
| `nw_listener_get_port` | function | `listener.h` | TcpListener |
| `nw_listener_set_advertise_descriptor` | function | `listener.h` | AdvertiseDescriptor, Advertiser, advertise_with_descriptor, Browser, BrowseDescriptor, start_browser_with_descriptor |
| `nw_listener_set_advertised_endpoint_changed_handler` | function | `listener.h` | TcpListener, TcpClient, ConnectionParameters, ConnectionGroup, Endpoint |
| `nw_listener_set_new_connection_group_handler` | function | `listener.h` | TcpListener, TcpClient, ConnectionParameters, ConnectionGroup, Endpoint |
| `nw_listener_set_new_connection_handler` | function | `listener.h` | AdvertiseDescriptor, Advertiser, advertise_with_descriptor, Browser, BrowseDescriptor, start_browser_with_descriptor, TcpListener |
| `nw_listener_set_new_connection_limit` | function | `listener.h` | TcpListener, TcpClient, ConnectionParameters, ConnectionGroup, Endpoint |
| `nw_listener_set_queue` | function | `listener.h` | AdvertiseDescriptor, Advertiser, advertise_with_descriptor, Browser, BrowseDescriptor, start_browser_with_descriptor, TcpListener |
| `nw_listener_set_state_changed_handler` | function | `listener.h` | AdvertiseDescriptor, Advertiser, advertise_with_descriptor, Browser, BrowseDescriptor, start_browser_with_descriptor, TcpListener |
| `nw_listener_start` | function | `listener.h` | AdvertiseDescriptor, Advertiser, advertise_with_descriptor, Browser, BrowseDescriptor, start_browser_with_descriptor, TcpListener |
| `nw_listener_state_t` | enum | `listener.h` | AdvertiseDescriptor, Advertiser, advertise_with_descriptor, Browser, BrowseDescriptor, start_browser_with_descriptor, TcpListener |
| `nw_listener_t` | type | `listener.h` | AdvertiseDescriptor, Advertiser, advertise_with_descriptor, Browser, BrowseDescriptor, start_browser_with_descriptor, TcpListener |
| `nw_multicast_group_descriptor_get_disable_unicast_traffic` | function | `group_descriptor.h` | ConnectionGroupDescriptor, Endpoint |
| `nw_multicast_group_descriptor_set_disable_unicast_traffic` | function | `group_descriptor.h` | ConnectionGroupDescriptor, Endpoint |
| `nw_multicast_group_descriptor_set_specific_source` | function | `group_descriptor.h` | ConnectionGroupDescriptor, Endpoint |
| `nw_multipath_service_t` | enum | `parameters.h` | ConnectionParameters, ProtocolStack, ServiceClass, MultipathService, ExpiredDnsBehavior |
| `nw_multipath_version_t` | enum | `tcp_options.h` | ProtocolOptions, ProtocolMetadata, TcpMultipathVersion |
| `nw_object_t` | type | `nw_object.h` | AdvertiseDescriptor, Advertiser, advertise_with_descriptor, Browser, BrowseDescriptor, start_browser_with_descriptor, ConnectionGroup, ConnectionGroupDescriptor, ConnectionParameters, ContentContext, ReceivedContent, Endpoint, FramerDefinition, FramerOptions, FramerContext, FramerMessage, Path, PrivacyContext, ProxyConfig, RelayHop, ResolverConfig, ProtocolDefinition, ProtocolOptions, QuicConnection, QuicOptions |
| `nw_parameters_attribution_t` | enum | `parameters.h` | ConnectionParameters |
| `nw_parameters_clear_prohibited_interface_types` | function | `parameters.h` | ConnectionParameters, ProtocolStack, ServiceClass, MultipathService, ExpiredDnsBehavior |
| `nw_parameters_clear_prohibited_interfaces` | function | `parameters.h` | ConnectionParameters, ProtocolStack, ServiceClass, MultipathService, ExpiredDnsBehavior |
| `nw_parameters_copy` | function | `parameters.h` | Browser, BrowseDescriptor, start_browser_with_descriptor, ConnectionParameters |
| `nw_parameters_copy_default_protocol_stack` | function | `parameters.h` | ConnectionParameters, ProtocolDefinition, ProtocolOptions, QuicConnection, QuicOptions, WebSocket |
| `nw_parameters_copy_local_endpoint` | function | `parameters.h` | ConnectionParameters, ProtocolStack, ServiceClass, MultipathService, ExpiredDnsBehavior |
| `nw_parameters_copy_required_interface` | function | `parameters.h` | ConnectionParameters, ProtocolStack, ServiceClass, MultipathService, ExpiredDnsBehavior |
| `nw_parameters_create` | function | `parameters.h` | Browser, BrowseDescriptor, start_browser_with_descriptor, ConnectionParameters, ProtocolDefinition, ProtocolOptions |
| `nw_parameters_create_application_service` | function | `parameters.h` | ConnectionParameters |
| `nw_parameters_create_custom_ip` | function | `parameters.h` | ConnectionParameters |
| `nw_parameters_create_quic` | function | `parameters.h` | ConnectionParameters, ProtocolStack, ServiceClass, MultipathService, ExpiredDnsBehavior |
| `nw_parameters_create_secure_tcp` | function | `parameters.h` | AdvertiseDescriptor, Advertiser, advertise_with_descriptor, Browser, BrowseDescriptor, start_browser_with_descriptor, ConnectionParameters, TcpClient, TcpListener, WebSocket |
| `nw_parameters_create_secure_udp` | function | `parameters.h` | ConnectionParameters, QuicConnection, QuicOptions, UdpClient |
| `nw_parameters_expired_dns_behavior_t` | enum | `parameters.h` | ConnectionParameters, ProtocolStack, ServiceClass, MultipathService, ExpiredDnsBehavior |
| `nw_parameters_get_allow_ultra_constrained` | function | `parameters.h` | ConnectionParameters |
| `nw_parameters_get_attribution` | function | `parameters.h` | ConnectionParameters |
| `nw_parameters_get_expired_dns_behavior` | function | `parameters.h` | ConnectionParameters, ProtocolStack, ServiceClass, MultipathService, ExpiredDnsBehavior |
| `nw_parameters_get_fast_open_enabled` | function | `parameters.h` | ConnectionParameters, ProtocolStack, ServiceClass, MultipathService, ExpiredDnsBehavior |
| `nw_parameters_get_include_peer_to_peer` | function | `parameters.h` | ConnectionParameters, ProtocolStack, ServiceClass, MultipathService, ExpiredDnsBehavior |
| `nw_parameters_get_local_only` | function | `parameters.h` | ConnectionParameters, ProtocolStack, ServiceClass, MultipathService, ExpiredDnsBehavior |
| `nw_parameters_get_multipath_service` | function | `parameters.h` | ConnectionParameters, ProtocolStack, ServiceClass, MultipathService, ExpiredDnsBehavior |
| `nw_parameters_get_prefer_no_proxy` | function | `parameters.h` | ConnectionParameters, ProtocolStack, ServiceClass, MultipathService, ExpiredDnsBehavior |
| `nw_parameters_get_prohibit_constrained` | function | `parameters.h` | ConnectionParameters |
| `nw_parameters_get_prohibit_expensive` | function | `parameters.h` | ConnectionParameters |
| `nw_parameters_get_required_interface_type` | function | `parameters.h` | ConnectionParameters |
| `nw_parameters_get_reuse_local_address` | function | `parameters.h` | ConnectionParameters, ProtocolStack, ServiceClass, MultipathService, ExpiredDnsBehavior |
| `nw_parameters_get_service_class` | function | `parameters.h` | ConnectionParameters, ProtocolStack, ServiceClass, MultipathService, ExpiredDnsBehavior |
| `nw_parameters_iterate_prohibited_interface_types` | function | `parameters.h` | ConnectionParameters, ProtocolStack, ServiceClass, MultipathService, ExpiredDnsBehavior |
| `nw_parameters_iterate_prohibited_interfaces` | function | `parameters.h` | ConnectionParameters, ProtocolStack, ServiceClass, MultipathService, ExpiredDnsBehavior |
| `nw_parameters_prohibit_interface` | function | `parameters.h` | ConnectionParameters, ProtocolStack, ServiceClass, MultipathService, ExpiredDnsBehavior |
| `nw_parameters_prohibit_interface_type` | function | `parameters.h` | ConnectionParameters, ProtocolStack, ServiceClass, MultipathService, ExpiredDnsBehavior |
| `nw_parameters_require_interface` | function | `parameters.h` | ConnectionParameters, ProtocolStack, ServiceClass, MultipathService, ExpiredDnsBehavior |
| `nw_parameters_requires_dnssec_validation` | function | `parameters.h` | ConnectionParameters, ProtocolStack, ServiceClass, MultipathService, ExpiredDnsBehavior |
| `nw_parameters_set_allow_ultra_constrained` | function | `parameters.h` | ConnectionParameters |
| `nw_parameters_set_attribution` | function | `parameters.h` | ConnectionParameters |
| `nw_parameters_set_expired_dns_behavior` | function | `parameters.h` | ConnectionParameters, ProtocolStack, ServiceClass, MultipathService, ExpiredDnsBehavior |
| `nw_parameters_set_fast_open_enabled` | function | `parameters.h` | ConnectionParameters, ProtocolStack, ServiceClass, MultipathService, ExpiredDnsBehavior |
| `nw_parameters_set_include_peer_to_peer` | function | `parameters.h` | ConnectionParameters, ProtocolStack, ServiceClass, MultipathService, ExpiredDnsBehavior |
| `nw_parameters_set_local_endpoint` | function | `parameters.h` | ConnectionParameters, ProtocolStack, ServiceClass, MultipathService, ExpiredDnsBehavior |
| `nw_parameters_set_local_only` | function | `parameters.h` | ConnectionParameters, ProtocolStack, ServiceClass, MultipathService, ExpiredDnsBehavior |
| `nw_parameters_set_multipath_service` | function | `parameters.h` | ConnectionParameters, ProtocolStack, ServiceClass, MultipathService, ExpiredDnsBehavior |
| `nw_parameters_set_prefer_no_proxy` | function | `parameters.h` | ConnectionParameters |
| `nw_parameters_set_privacy_context` | function | `parameters.h` | ConnectionParameters |
| `nw_parameters_set_prohibit_constrained` | function | `parameters.h` | ConnectionParameters |
| `nw_parameters_set_prohibit_expensive` | function | `parameters.h` | ConnectionParameters |
| `nw_parameters_set_required_interface_type` | function | `parameters.h` | ConnectionParameters |
| `nw_parameters_set_requires_dnssec_validation` | function | `parameters.h` | ConnectionParameters, ProtocolStack, ServiceClass, MultipathService, ExpiredDnsBehavior |
| `nw_parameters_set_reuse_local_address` | function | `parameters.h` | ConnectionParameters, ProtocolStack, ServiceClass, MultipathService, ExpiredDnsBehavior |
| `nw_parameters_set_service_class` | function | `parameters.h` | ConnectionParameters, ProtocolStack, ServiceClass, MultipathService, ExpiredDnsBehavior |
| `nw_parameters_t` | type | `parameters.h` | AdvertiseDescriptor, Advertiser, advertise_with_descriptor, Browser, BrowseDescriptor, start_browser_with_descriptor, ConnectionGroup, ConnectionGroupDescriptor, ConnectionParameters, ProtocolDefinition, ProtocolOptions, QuicConnection, QuicOptions, TcpClient, TcpListener, UdpClient, WebSocket |
| `nw_path_copy_effective_local_endpoint` | function | `path.h` | Path |
| `nw_path_copy_effective_remote_endpoint` | function | `path.h` | Path |
| `nw_path_enumerate_gateways` | function | `path.h` | Path, PathMonitor, EthernetChannel, Endpoint |
| `nw_path_enumerate_interfaces` | function | `path.h` | NetworkInterface, list_interfaces, Path |
| `nw_path_get_link_quality` | function | `path.h` | Path |
| `nw_path_get_status` | function | `path.h` | Path, PathMonitor, start_path_monitor |
| `nw_path_get_unsatisfied_reason` | function | `path.h` | Path |
| `nw_path_has_dns` | function | `path.h` | Path |
| `nw_path_has_ipv4` | function | `path.h` | Path |
| `nw_path_has_ipv6` | function | `path.h` | Path |
| `nw_path_is_constrained` | function | `path.h` | Path |
| `nw_path_is_equal` | function | `path.h` | Path |
| `nw_path_is_expensive` | function | `path.h` | Path |
| `nw_path_is_ultra_constrained` | function | `path.h` | Path |
| `nw_path_monitor_cancel` | function | `path_monitor.h` | NetworkInterface, list_interfaces, PathMonitor, start_path_monitor |
| `nw_path_monitor_create` | function | `path_monitor.h` | NetworkInterface, list_interfaces, PathMonitor, start_path_monitor |
| `nw_path_monitor_create_for_ethernet_channel` | function | `path_monitor.h` | PathMonitor, EthernetChannel, InterfaceType |
| `nw_path_monitor_create_with_type` | function | `path_monitor.h` | PathMonitor, EthernetChannel, InterfaceType |
| `nw_path_monitor_prohibit_interface_type` | function | `path_monitor.h` | PathMonitor, EthernetChannel, InterfaceType |
| `nw_path_monitor_set_cancel_handler` | function | `path_monitor.h` | PathMonitor, EthernetChannel, InterfaceType |
| `nw_path_monitor_set_queue` | function | `path_monitor.h` | NetworkInterface, list_interfaces, PathMonitor, start_path_monitor |
| `nw_path_monitor_set_update_handler` | function | `path_monitor.h` | NetworkInterface, list_interfaces, PathMonitor, start_path_monitor |
| `nw_path_monitor_start` | function | `path_monitor.h` | NetworkInterface, list_interfaces, PathMonitor, start_path_monitor |
| `nw_path_monitor_t` | type | `path_monitor.h` | NetworkInterface, list_interfaces |
| `nw_path_status_t` | enum | `path.h` | Path |
| `nw_path_t` | type | `path.h` | NetworkInterface, list_interfaces, Path, PathMonitor, start_path_monitor |
| `nw_path_unsatisfied_reason_t` | enum | `path.h` | Path |
| `nw_path_uses_interface_type` | function | `path.h` | Path, PathMonitor, start_path_monitor |
| `nw_privacy_context_add_proxy` | function | `privacy_context.h` | PrivacyContext, ProxyConfig, RelayHop, ResolverConfig |
| `nw_privacy_context_clear_proxies` | function | `privacy_context.h` | PrivacyContext, ProxyConfig, RelayHop, ResolverConfig |
| `nw_privacy_context_create` | function | `privacy_context.h` | PrivacyContext, ProxyConfig, RelayHop, ResolverConfig |
| `nw_privacy_context_disable_logging` | function | `privacy_context.h` | PrivacyContext, ProxyConfig, RelayHop, ResolverConfig |
| `nw_privacy_context_flush_cache` | function | `privacy_context.h` | PrivacyContext, ProxyConfig, RelayHop, ResolverConfig |
| `nw_privacy_context_require_encrypted_name_resolution` | function | `privacy_context.h` | PrivacyContext, ProxyConfig, RelayHop, ResolverConfig |
| `nw_privacy_context_t` | type | `privacy_context.h` | ConnectionParameters, PrivacyContext, ProxyConfig, RelayHop, ResolverConfig |
| `nw_protocol_copy_ip_definition` | function | `ip_options.h` | ProtocolDefinition, ProtocolOptions |
| `nw_protocol_copy_quic_definition` | function | `quic_options.h` | ProtocolDefinition, ProtocolOptions |
| `nw_protocol_copy_tcp_definition` | function | `tcp_options.h` | ProtocolDefinition, ProtocolOptions |
| `nw_protocol_copy_tls_definition` | function | `tls_options.h` | ProtocolDefinition, ProtocolOptions |
| `nw_protocol_copy_udp_definition` | function | `udp_options.h` | ProtocolDefinition, ProtocolOptions |
| `nw_protocol_copy_ws_definition` | function | `ws_options.h` | ProtocolDefinition, ProtocolOptions, WebSocket |
| `nw_protocol_definition_is_equal` | function | `protocol_options.h` | ProtocolDefinition, ProtocolOptions |
| `nw_protocol_definition_t` | type | `protocol_options.h` | ContentContext, ReceivedContent, FramerDefinition, FramerOptions, FramerContext, FramerMessage, ProtocolDefinition, ProtocolOptions |
| `nw_protocol_metadata_copy_definition` | function | `protocol_options.h` | raw_ffi::nw_shim_protocol_metadata_copy_definition |
| `nw_protocol_metadata_is_framer_message` | function | `framer_options.h` | FramerDefinition, FramerOptions, FramerContext, FramerMessage |
| `nw_protocol_metadata_is_ip` | function | `ip_options.h` | ProtocolOptions, ProtocolMetadata, IpVersion, IpEcnFlag, IpLocalAddressPreference, ServiceClass |
| `nw_protocol_metadata_is_quic` | function | `quic_options.h` | QuicConnection, QuicMetadata, QuicOptions, QuicStreamType, SecurityProtocolMetadata, SecurityProtocolOptions |
| `nw_protocol_metadata_is_tcp` | function | `tcp_options.h` | ProtocolOptions, ProtocolMetadata, TcpMultipathVersion |
| `nw_protocol_metadata_is_tls` | function | `tls_options.h` | ProtocolOptions, ProtocolMetadata, SecurityProtocolMetadata, SecurityProtocolOptions |
| `nw_protocol_metadata_is_udp` | function | `udp_options.h` | ProtocolOptions, ProtocolMetadata |
| `nw_protocol_metadata_is_ws` | function | `ws_options.h` | ProtocolOptions, ProtocolMetadata, WsRequest, WsResponse, WsVersion, WsCloseCode, WsResponseStatus |
| `nw_protocol_metadata_t` | type | `protocol_options.h` | ContentContext, ReceivedContent, FramerDefinition, FramerOptions, FramerContext, FramerMessage, WebSocket |
| `nw_protocol_options_copy_definition` | function | `protocol_options.h` | ContentContext, ReceivedContent, FramerDefinition, FramerOptions, FramerContext, FramerMessage, ProtocolDefinition, ProtocolOptions |
| `nw_protocol_options_is_quic` | function | `quic_options.h` | ProtocolDefinition, ProtocolOptions |
| `nw_protocol_options_t` | type | `protocol_options.h` | ConnectionParameters, ContentContext, ReceivedContent, FramerDefinition, FramerOptions, FramerContext, FramerMessage, PrivacyContext, ProxyConfig, RelayHop, ResolverConfig, ProtocolDefinition, ProtocolOptions, QuicConnection, QuicOptions, WebSocket |
| `nw_protocol_stack_clear_application_protocols` | function | `parameters.h` | ConnectionParameters, ProtocolStack, ServiceClass, MultipathService, ExpiredDnsBehavior |
| `nw_protocol_stack_copy_internet_protocol` | function | `parameters.h` | ProtocolDefinition, ProtocolOptions |
| `nw_protocol_stack_copy_transport_protocol` | function | `parameters.h` | ConnectionParameters, ProtocolStack, ServiceClass, MultipathService, ExpiredDnsBehavior |
| `nw_protocol_stack_iterate_application_protocols` | function | `parameters.h` | ConnectionParameters, ProtocolStack, ServiceClass, MultipathService, ExpiredDnsBehavior |
| `nw_protocol_stack_prepend_application_protocol` | function | `parameters.h` | ConnectionParameters, QuicConnection, QuicOptions, WebSocket |
| `nw_protocol_stack_set_transport_protocol` | function | `parameters.h` | ConnectionParameters, ProtocolStack, ServiceClass, MultipathService, ExpiredDnsBehavior |
| `nw_protocol_stack_t` | type | `parameters.h` | ConnectionParameters, ProtocolDefinition, ProtocolOptions, QuicConnection, QuicOptions, WebSocket |
| `nw_proxy_config_add_excluded_domain` | function | `proxy_config.h` | PrivacyContext, ProxyConfig, RelayHop, ResolverConfig |
| `nw_proxy_config_add_match_domain` | function | `proxy_config.h` | PrivacyContext, ProxyConfig, RelayHop, ResolverConfig |
| `nw_proxy_config_clear_excluded_domains` | function | `proxy_config.h` | PrivacyContext, ProxyConfig, RelayHop, ResolverConfig |
| `nw_proxy_config_clear_match_domains` | function | `proxy_config.h` | PrivacyContext, ProxyConfig, RelayHop, ResolverConfig |
| `nw_proxy_config_create_http_connect` | function | `proxy_config.h` | PrivacyContext, ProxyConfig, RelayHop, ResolverConfig |
| `nw_proxy_config_create_oblivious_http` | function | `proxy_config.h` | PrivacyContext, ProxyConfig, RelayHop, ResolverConfig |
| `nw_proxy_config_create_relay` | function | `proxy_config.h` | PrivacyContext, ProxyConfig, RelayHop, ResolverConfig |
| `nw_proxy_config_create_socksv5` | function | `proxy_config.h` | PrivacyContext, ProxyConfig, RelayHop, ResolverConfig |
| `nw_proxy_config_enumerate_excluded_domains` | function | `proxy_config.h` | PrivacyContext, ProxyConfig, RelayHop, ResolverConfig |
| `nw_proxy_config_enumerate_match_domains` | function | `proxy_config.h` | PrivacyContext, ProxyConfig, RelayHop, ResolverConfig |
| `nw_proxy_config_get_failover_allowed` | function | `proxy_config.h` | PrivacyContext, ProxyConfig, RelayHop, ResolverConfig |
| `nw_proxy_config_set_failover_allowed` | function | `proxy_config.h` | PrivacyContext, ProxyConfig, RelayHop, ResolverConfig |
| `nw_proxy_config_set_username_and_password` | function | `proxy_config.h` | PrivacyContext, ProxyConfig, RelayHop, ResolverConfig |
| `nw_proxy_config_t` | type | `proxy_config.h` | PrivacyContext, ProxyConfig, RelayHop, ResolverConfig |
| `nw_quic_add_tls_application_protocol` | function | `quic_options.h` | ConnectionParameters, QuicConnection, QuicOptions |
| `nw_quic_copy_sec_protocol_metadata` | function | `quic_options.h` | QuicConnection, QuicMetadata, QuicOptions, QuicStreamType, SecurityProtocolMetadata, SecurityProtocolOptions |
| `nw_quic_copy_sec_protocol_options` | function | `quic_options.h` | QuicConnection, QuicMetadata, QuicOptions, QuicStreamType, SecurityProtocolMetadata, SecurityProtocolOptions |
| `nw_quic_create_options` | function | `quic_options.h` | ConnectionParameters, ProtocolDefinition, ProtocolOptions, QuicConnection, QuicOptions |
| `nw_quic_get_application_error` | function | `quic_options.h` | QuicConnection, QuicMetadata, QuicOptions, QuicStreamType, SecurityProtocolMetadata, SecurityProtocolOptions |
| `nw_quic_get_application_error_reason` | function | `quic_options.h` | QuicConnection, QuicMetadata, QuicOptions, QuicStreamType, SecurityProtocolMetadata, SecurityProtocolOptions |
| `nw_quic_get_idle_timeout` | function | `quic_options.h` | QuicConnection, QuicOptions |
| `nw_quic_get_initial_max_data` | function | `quic_options.h` | QuicConnection, QuicOptions |
| `nw_quic_get_initial_max_stream_data_bidirectional_local` | function | `quic_options.h` | QuicConnection, QuicMetadata, QuicOptions, QuicStreamType, SecurityProtocolMetadata, SecurityProtocolOptions |
| `nw_quic_get_initial_max_stream_data_bidirectional_remote` | function | `quic_options.h` | QuicConnection, QuicMetadata, QuicOptions, QuicStreamType, SecurityProtocolMetadata, SecurityProtocolOptions |
| `nw_quic_get_initial_max_stream_data_unidirectional` | function | `quic_options.h` | QuicConnection, QuicMetadata, QuicOptions, QuicStreamType, SecurityProtocolMetadata, SecurityProtocolOptions |
| `nw_quic_get_initial_max_streams_bidirectional` | function | `quic_options.h` | QuicConnection, QuicMetadata, QuicOptions, QuicStreamType, SecurityProtocolMetadata, SecurityProtocolOptions |
| `nw_quic_get_initial_max_streams_unidirectional` | function | `quic_options.h` | QuicConnection, QuicMetadata, QuicOptions, QuicStreamType, SecurityProtocolMetadata, SecurityProtocolOptions |
| `nw_quic_get_keepalive_interval` | function | `quic_options.h` | QuicConnection, QuicMetadata, QuicOptions, QuicStreamType, SecurityProtocolMetadata, SecurityProtocolOptions |
| `nw_quic_get_local_max_streams_bidirectional` | function | `quic_options.h` | QuicConnection, QuicMetadata, QuicOptions, QuicStreamType, SecurityProtocolMetadata, SecurityProtocolOptions |
| `nw_quic_get_local_max_streams_unidirectional` | function | `quic_options.h` | QuicConnection, QuicMetadata, QuicOptions, QuicStreamType, SecurityProtocolMetadata, SecurityProtocolOptions |
| `nw_quic_get_max_datagram_frame_size` | function | `quic_options.h` | QuicConnection, QuicMetadata, QuicOptions, QuicStreamType, SecurityProtocolMetadata, SecurityProtocolOptions |
| `nw_quic_get_max_udp_payload_size` | function | `quic_options.h` | QuicConnection, QuicOptions |
| `nw_quic_get_remote_idle_timeout` | function | `quic_options.h` | QuicConnection, QuicMetadata, QuicOptions, QuicStreamType, SecurityProtocolMetadata, SecurityProtocolOptions |
| `nw_quic_get_remote_max_streams_bidirectional` | function | `quic_options.h` | QuicConnection, QuicMetadata, QuicOptions, QuicStreamType, SecurityProtocolMetadata, SecurityProtocolOptions |
| `nw_quic_get_remote_max_streams_unidirectional` | function | `quic_options.h` | QuicConnection, QuicMetadata, QuicOptions, QuicStreamType, SecurityProtocolMetadata, SecurityProtocolOptions |
| `nw_quic_get_stream_application_error` | function | `quic_options.h` | QuicConnection, QuicMetadata, QuicOptions, QuicStreamType, SecurityProtocolMetadata, SecurityProtocolOptions |
| `nw_quic_get_stream_id` | function | `quic_options.h` | QuicConnection, QuicMetadata, QuicOptions, QuicStreamType, SecurityProtocolMetadata, SecurityProtocolOptions |
| `nw_quic_get_stream_is_datagram` | function | `quic_options.h` | QuicConnection, QuicOptions |
| `nw_quic_get_stream_is_unidirectional` | function | `quic_options.h` | QuicConnection, QuicOptions |
| `nw_quic_get_stream_type` | function | `quic_options.h` | QuicConnection, QuicMetadata, QuicOptions, QuicStreamType, SecurityProtocolMetadata, SecurityProtocolOptions |
| `nw_quic_get_stream_usable_datagram_frame_size` | function | `quic_options.h` | QuicConnection, QuicMetadata, QuicOptions, QuicStreamType, SecurityProtocolMetadata, SecurityProtocolOptions |
| `nw_quic_set_application_error` | function | `quic_options.h` | QuicConnection, QuicMetadata, QuicOptions, QuicStreamType, SecurityProtocolMetadata, SecurityProtocolOptions |
| `nw_quic_set_idle_timeout` | function | `quic_options.h` | QuicConnection, QuicOptions |
| `nw_quic_set_initial_max_data` | function | `quic_options.h` | QuicConnection, QuicOptions |
| `nw_quic_set_initial_max_stream_data_bidirectional_local` | function | `quic_options.h` | QuicConnection, QuicMetadata, QuicOptions, QuicStreamType, SecurityProtocolMetadata, SecurityProtocolOptions |
| `nw_quic_set_initial_max_stream_data_bidirectional_remote` | function | `quic_options.h` | QuicConnection, QuicMetadata, QuicOptions, QuicStreamType, SecurityProtocolMetadata, SecurityProtocolOptions |
| `nw_quic_set_initial_max_stream_data_unidirectional` | function | `quic_options.h` | QuicConnection, QuicMetadata, QuicOptions, QuicStreamType, SecurityProtocolMetadata, SecurityProtocolOptions |
| `nw_quic_set_initial_max_streams_bidirectional` | function | `quic_options.h` | QuicConnection, QuicMetadata, QuicOptions, QuicStreamType, SecurityProtocolMetadata, SecurityProtocolOptions |
| `nw_quic_set_initial_max_streams_unidirectional` | function | `quic_options.h` | QuicConnection, QuicMetadata, QuicOptions, QuicStreamType, SecurityProtocolMetadata, SecurityProtocolOptions |
| `nw_quic_set_keepalive_interval` | function | `quic_options.h` | QuicConnection, QuicMetadata, QuicOptions, QuicStreamType, SecurityProtocolMetadata, SecurityProtocolOptions |
| `nw_quic_set_local_max_streams_bidirectional` | function | `quic_options.h` | QuicConnection, QuicMetadata, QuicOptions, QuicStreamType, SecurityProtocolMetadata, SecurityProtocolOptions |
| `nw_quic_set_local_max_streams_unidirectional` | function | `quic_options.h` | QuicConnection, QuicMetadata, QuicOptions, QuicStreamType, SecurityProtocolMetadata, SecurityProtocolOptions |
| `nw_quic_set_max_datagram_frame_size` | function | `quic_options.h` | QuicConnection, QuicMetadata, QuicOptions, QuicStreamType, SecurityProtocolMetadata, SecurityProtocolOptions |
| `nw_quic_set_max_udp_payload_size` | function | `quic_options.h` | QuicConnection, QuicOptions |
| `nw_quic_set_stream_application_error` | function | `quic_options.h` | QuicConnection, QuicMetadata, QuicOptions, QuicStreamType, SecurityProtocolMetadata, SecurityProtocolOptions |
| `nw_quic_set_stream_is_datagram` | function | `quic_options.h` | QuicConnection, QuicOptions |
| `nw_quic_set_stream_is_unidirectional` | function | `quic_options.h` | QuicConnection, QuicOptions |
| `nw_quic_stream_type_t` | enum | `quic_options.h` | QuicConnection, QuicMetadata, QuicOptions, QuicStreamType, SecurityProtocolMetadata, SecurityProtocolOptions |
| `nw_relay_hop_add_additional_http_header_field` | function | `proxy_config.h` | PrivacyContext, ProxyConfig, RelayHop, ResolverConfig |
| `nw_relay_hop_create` | function | `proxy_config.h` | PrivacyContext, ProxyConfig, RelayHop, ResolverConfig |
| `nw_relay_hop_t` | type | `proxy_config.h` | PrivacyContext, ProxyConfig, RelayHop, ResolverConfig |
| `nw_release` | function | `nw_object.h` | AdvertiseDescriptor, Advertiser, advertise_with_descriptor, Browser, BrowseDescriptor, start_browser_with_descriptor, ConnectionGroup, ConnectionGroupDescriptor, ConnectionParameters, ContentContext, ReceivedContent, Endpoint, FramerDefinition, FramerOptions, FramerContext, FramerMessage, NetworkInterface, list_interfaces, Path, PathMonitor, start_path_monitor, PrivacyContext, ProxyConfig, RelayHop, ResolverConfig, ProtocolDefinition, ProtocolOptions, QuicConnection, QuicOptions, TcpClient, TcpListener, UdpClient, WebSocket |
| `nw_report_resolution_protocol_t` | enum | `connection_report.h` | DataTransferReport, EstablishmentReport, ResolutionReport, TcpClient, QuicConnection |
| `nw_report_resolution_source_t` | enum | `connection_report.h` | DataTransferReport, EstablishmentReport, ResolutionReport, TcpClient, QuicConnection |
| `nw_resolution_report_copy_preferred_endpoint` | function | `connection_report.h` | DataTransferReport, EstablishmentReport, ResolutionReport, TcpClient, QuicConnection |
| `nw_resolution_report_copy_successful_endpoint` | function | `connection_report.h` | DataTransferReport, EstablishmentReport, ResolutionReport, TcpClient, QuicConnection |
| `nw_resolution_report_get_endpoint_count` | function | `connection_report.h` | DataTransferReport, EstablishmentReport, ResolutionReport, TcpClient, QuicConnection |
| `nw_resolution_report_get_milliseconds` | function | `connection_report.h` | DataTransferReport, EstablishmentReport, ResolutionReport, TcpClient, QuicConnection |
| `nw_resolution_report_get_protocol` | function | `connection_report.h` | DataTransferReport, EstablishmentReport, ResolutionReport, TcpClient, QuicConnection |
| `nw_resolution_report_get_source` | function | `connection_report.h` | DataTransferReport, EstablishmentReport, ResolutionReport, TcpClient, QuicConnection |
| `nw_resolution_report_t` | type | `connection_report.h` | DataTransferReport, EstablishmentReport, ResolutionReport, TcpClient, QuicConnection |
| `nw_resolver_config_add_server_address` | function | `resolver_config.h` | PrivacyContext, ProxyConfig, RelayHop, ResolverConfig |
| `nw_resolver_config_create_https` | function | `resolver_config.h` | PrivacyContext, ProxyConfig, RelayHop, ResolverConfig |
| `nw_resolver_config_create_tls` | function | `resolver_config.h` | PrivacyContext, ProxyConfig, RelayHop, ResolverConfig |
| `nw_resolver_config_t` | type | `resolver_config.h` | PrivacyContext, ProxyConfig, RelayHop, ResolverConfig |
| `nw_retain` | function | `nw_object.h` | AdvertiseDescriptor, Advertiser, advertise_with_descriptor, Browser, BrowseDescriptor, start_browser_with_descriptor, ConnectionGroup, ConnectionGroupDescriptor, ContentContext, ReceivedContent, Endpoint, FramerDefinition, FramerOptions, FramerContext, FramerMessage, NetworkInterface, list_interfaces, Path, PathMonitor, start_path_monitor, PrivacyContext, ProxyConfig, RelayHop, ResolverConfig, ProtocolDefinition, ProtocolOptions, QuicConnection, QuicOptions, TcpClient, TcpListener, UdpClient |
| `nw_service_class_t` | enum | `parameters.h` | ConnectionParameters, ProtocolStack, ServiceClass, MultipathService, ExpiredDnsBehavior |
| `nw_tcp_create_options` | function | `tcp_options.h` | ProtocolDefinition, ProtocolOptions |
| `nw_tcp_get_available_receive_buffer` | function | `tcp_options.h` | ProtocolOptions, ProtocolMetadata, TcpMultipathVersion |
| `nw_tcp_get_available_send_buffer` | function | `tcp_options.h` | ProtocolOptions, ProtocolMetadata, TcpMultipathVersion |
| `nw_tcp_options_set_connection_timeout` | function | `tcp_options.h` | ProtocolOptions, ProtocolMetadata, TcpMultipathVersion |
| `nw_tcp_options_set_disable_ack_stretching` | function | `tcp_options.h` | ProtocolOptions, ProtocolMetadata, TcpMultipathVersion |
| `nw_tcp_options_set_disable_ecn` | function | `tcp_options.h` | ProtocolOptions, ProtocolMetadata, TcpMultipathVersion |
| `nw_tcp_options_set_enable_fast_open` | function | `tcp_options.h` | ProtocolOptions, ProtocolMetadata, TcpMultipathVersion |
| `nw_tcp_options_set_enable_keepalive` | function | `tcp_options.h` | ProtocolOptions, ProtocolMetadata, TcpMultipathVersion |
| `nw_tcp_options_set_keepalive_count` | function | `tcp_options.h` | ProtocolOptions, ProtocolMetadata, TcpMultipathVersion |
| `nw_tcp_options_set_keepalive_idle_time` | function | `tcp_options.h` | ProtocolOptions, ProtocolMetadata, TcpMultipathVersion |
| `nw_tcp_options_set_keepalive_interval` | function | `tcp_options.h` | ProtocolOptions, ProtocolMetadata, TcpMultipathVersion |
| `nw_tcp_options_set_maximum_segment_size` | function | `tcp_options.h` | ProtocolOptions, ProtocolMetadata, TcpMultipathVersion |
| `nw_tcp_options_set_multipath_force_version` | function | `tcp_options.h` | ProtocolOptions, ProtocolMetadata, TcpMultipathVersion |
| `nw_tcp_options_set_no_delay` | function | `tcp_options.h` | ProtocolOptions, ProtocolMetadata, TcpMultipathVersion |
| `nw_tcp_options_set_no_options` | function | `tcp_options.h` | ProtocolOptions, ProtocolMetadata, TcpMultipathVersion |
| `nw_tcp_options_set_no_push` | function | `tcp_options.h` | ProtocolOptions, ProtocolMetadata, TcpMultipathVersion |
| `nw_tcp_options_set_persist_timeout` | function | `tcp_options.h` | ProtocolOptions, ProtocolMetadata, TcpMultipathVersion |
| `nw_tcp_options_set_retransmit_connection_drop_time` | function | `tcp_options.h` | ProtocolOptions, ProtocolMetadata, TcpMultipathVersion |
| `nw_tcp_options_set_retransmit_fin_drop` | function | `tcp_options.h` | ProtocolOptions, ProtocolMetadata, TcpMultipathVersion |
| `nw_tls_copy_sec_protocol_metadata` | function | `tls_options.h` | ProtocolOptions, ProtocolMetadata, SecurityProtocolMetadata, SecurityProtocolOptions |
| `nw_tls_copy_sec_protocol_options` | function | `tls_options.h` | ProtocolOptions, ProtocolMetadata, SecurityProtocolMetadata, SecurityProtocolOptions |
| `nw_tls_create_options` | function | `tls_options.h` | PrivacyContext, ProxyConfig, RelayHop, ResolverConfig, ProtocolDefinition, ProtocolOptions |
| `nw_txt_record_access_bytes` | function | `txt_record.h` | Endpoint, TxtRecord |
| `nw_txt_record_access_key` | function | `txt_record.h` | Endpoint, TxtRecord |
| `nw_txt_record_apply` | function | `txt_record.h` | Endpoint, TxtRecord |
| `nw_txt_record_copy` | function | `txt_record.h` | Endpoint, TxtRecord |
| `nw_txt_record_create_dictionary` | function | `txt_record.h` | Endpoint, TxtRecord |
| `nw_txt_record_create_with_bytes` | function | `txt_record.h` | Endpoint, TxtRecord |
| `nw_txt_record_find_key` | function | `txt_record.h` | Endpoint, TxtRecord |
| `nw_txt_record_find_key_t` | enum | `txt_record.h` | Endpoint, TxtRecord |
| `nw_txt_record_get_key_count` | function | `txt_record.h` | Endpoint, TxtRecord |
| `nw_txt_record_is_dictionary` | function | `txt_record.h` | Endpoint, TxtRecord |
| `nw_txt_record_is_equal` | function | `txt_record.h` | Endpoint, TxtRecord |
| `nw_txt_record_remove_key` | function | `txt_record.h` | Endpoint, TxtRecord |
| `nw_txt_record_set_key` | function | `txt_record.h` | Endpoint, TxtRecord |
| `nw_txt_record_t` | type | `txt_record.h` | Endpoint, TxtRecord |
| `nw_udp_create_metadata` | function | `udp_options.h` | ProtocolOptions, ProtocolMetadata |
| `nw_udp_create_options` | function | `udp_options.h` | ProtocolDefinition, ProtocolOptions |
| `nw_udp_options_set_prefer_no_checksum` | function | `udp_options.h` | ProtocolOptions, ProtocolMetadata |
| `nw_ws_close_code_t` | enum | `ws_options.h` | ProtocolOptions, ProtocolMetadata, WsRequest, WsResponse, WsVersion, WsCloseCode, WsResponseStatus |
| `nw_ws_create_metadata` | function | `ws_options.h` | WebSocket |
| `nw_ws_create_options` | function | `ws_options.h` | ProtocolDefinition, ProtocolOptions, WebSocket |
| `nw_ws_metadata_copy_server_response` | function | `ws_options.h` | ProtocolOptions, ProtocolMetadata, WsRequest, WsResponse, WsVersion, WsCloseCode, WsResponseStatus |
| `nw_ws_metadata_get_close_code` | function | `ws_options.h` | ProtocolOptions, ProtocolMetadata, WsRequest, WsResponse, WsVersion, WsCloseCode, WsResponseStatus |
| `nw_ws_metadata_get_opcode` | function | `ws_options.h` | WebSocket |
| `nw_ws_metadata_set_close_code` | function | `ws_options.h` | ProtocolOptions, ProtocolMetadata, WsRequest, WsResponse, WsVersion, WsCloseCode, WsResponseStatus |
| `nw_ws_options_add_additional_header` | function | `ws_options.h` | ProtocolOptions, ProtocolMetadata, WsRequest, WsResponse, WsVersion, WsCloseCode, WsResponseStatus |
| `nw_ws_options_add_subprotocol` | function | `ws_options.h` | ProtocolOptions, ProtocolMetadata, WsRequest, WsResponse, WsVersion, WsCloseCode, WsResponseStatus |
| `nw_ws_options_set_auto_reply_ping` | function | `ws_options.h` | ProtocolOptions, ProtocolMetadata, WsRequest, WsResponse, WsVersion, WsCloseCode, WsResponseStatus |
| `nw_ws_options_set_client_request_handler` | function | `ws_options.h` | ProtocolOptions, ProtocolMetadata, WsRequest, WsResponse, WsVersion, WsCloseCode, WsResponseStatus |
| `nw_ws_options_set_maximum_message_size` | function | `ws_options.h` | ProtocolOptions, ProtocolMetadata, WsRequest, WsResponse, WsVersion, WsCloseCode, WsResponseStatus |
| `nw_ws_options_set_skip_handshake` | function | `ws_options.h` | ProtocolOptions, ProtocolMetadata, WsRequest, WsResponse, WsVersion, WsCloseCode, WsResponseStatus |
| `nw_ws_request_enumerate_additional_headers` | function | `ws_options.h` | ProtocolOptions, ProtocolMetadata, WsRequest, WsResponse, WsVersion, WsCloseCode, WsResponseStatus |
| `nw_ws_request_enumerate_subprotocols` | function | `ws_options.h` | ProtocolOptions, ProtocolMetadata, WsRequest, WsResponse, WsVersion, WsCloseCode, WsResponseStatus |
| `nw_ws_request_t` | type | `ws_options.h` | ProtocolOptions, ProtocolMetadata, WsRequest, WsResponse, WsVersion, WsCloseCode, WsResponseStatus |
| `nw_ws_response_add_additional_header` | function | `ws_options.h` | ProtocolOptions, ProtocolMetadata, WsRequest, WsResponse, WsVersion, WsCloseCode, WsResponseStatus |
| `nw_ws_response_create` | function | `ws_options.h` | ProtocolOptions, ProtocolMetadata, WsRequest, WsResponse, WsVersion, WsCloseCode, WsResponseStatus |
| `nw_ws_response_enumerate_additional_headers` | function | `ws_options.h` | ProtocolOptions, ProtocolMetadata, WsRequest, WsResponse, WsVersion, WsCloseCode, WsResponseStatus |
| `nw_ws_response_get_selected_subprotocol` | function | `ws_options.h` | ProtocolOptions, ProtocolMetadata, WsRequest, WsResponse, WsVersion, WsCloseCode, WsResponseStatus |
| `nw_ws_response_get_status` | function | `ws_options.h` | ProtocolOptions, ProtocolMetadata, WsRequest, WsResponse, WsVersion, WsCloseCode, WsResponseStatus |
| `nw_ws_response_status_t` | enum | `ws_options.h` | ProtocolOptions, ProtocolMetadata, WsRequest, WsResponse, WsVersion, WsCloseCode, WsResponseStatus |
| `nw_ws_response_t` | type | `ws_options.h` | ProtocolOptions, ProtocolMetadata, WsRequest, WsResponse, WsVersion, WsCloseCode, WsResponseStatus |
| `nw_ws_version_t` | enum | `ws_options.h` | ProtocolOptions, ProtocolMetadata, WsRequest, WsResponse, WsVersion, WsCloseCode, WsResponseStatus |

## 🔴 GAPS
| Symbol | Kind | Header | Why not wrapped |
| --- | --- | --- | --- |
| `nw_error_copy_cf_error` | function | `error.h` | The crate maps errors to NetworkError but does not expose nw_error domain/CFError utilities. |
| `nw_ws_metadata_set_pong_handler` | function | `ws_options.h` | WebSocket transport is wrapped, but request/response metadata and most WS option setters are missing. |
