#ifndef NETWORKFRAMEWORK_RS_NETWORK_SHIM_H
#define NETWORKFRAMEWORK_RS_NETWORK_SHIM_H

#include <stddef.h>
#include <stdint.h>
#include <sys/types.h>

#ifdef __cplusplus
extern "C" {
#endif

#define NW_OK 0
#define NW_INVALID_ARG -1
#define NW_CONNECT_FAILED -2
#define NW_SEND_FAILED -3
#define NW_RECV_FAILED -4
#define NW_LISTEN_FAILED -5
#define NW_CANCELLED -6
#define NW_TIMEOUT -7

typedef void (*BrowserServiceCallback)(
    const char *name,
    const char *service_type,
    const char *domain,
    void *user_info
);

typedef void (*PathMonitorCallback)(
    int satisfied,
    int interface_type,
    void *user_info
);

typedef int (*InterfaceEnumerationCallback)(
    const char *name,
    int interface_type,
    uint32_t index,
    void *user_info
);

typedef void (*StringEnumerationCallback)(
    const char *value,
    void *user_info
);

typedef void *(*FramerCreateInstanceCallback)(void *user_info);
typedef void (*FramerDropInstanceCallback)(void *instance);
typedef int (*FramerStartCallback)(void *instance, void *framer);
typedef size_t (*FramerInputCallback)(void *instance, void *framer);
typedef void (*FramerOutputCallback)(
    void *instance,
    void *framer,
    void *message,
    size_t message_length,
    int is_complete
);
typedef void (*FramerWakeupCallback)(void *instance, void *framer);
typedef int (*FramerStopCallback)(void *instance, void *framer);
typedef void (*FramerCleanupCallback)(void *instance, void *framer);
typedef size_t (*FramerParseCallback)(
    const uint8_t *buffer,
    size_t buffer_length,
    int is_complete,
    void *user_info
);
typedef void (*FramerAsyncCallback)(void *framer, void *user_info);

typedef void (*ConnectionGroupStateCallback)(int state, void *user_info);
typedef void (*ConnectionGroupReceiveCallback)(
    const uint8_t *data,
    size_t len,
    void *context,
    int is_complete,
    void *user_info
);

typedef int (*TxtRecordEntryCallback)(
    const char *key,
    int found,
    const uint8_t *value,
    size_t value_len,
    void *user_info
);

typedef void (*EthernetChannelStateCallback)(int state, void *user_info);
typedef void (*EthernetChannelReceiveCallback)(
    const uint8_t *data,
    size_t len,
    uint16_t vlan_tag,
    const uint8_t *local_address,
    const uint8_t *remote_address,
    void *user_info
);

typedef struct nw_shim_establishment_protocol_info {
    void *protocol_definition;
    uint64_t handshake_milliseconds;
    uint64_t handshake_rtt_milliseconds;
} nw_shim_establishment_protocol_info;

typedef struct nw_shim_resolution_step_info {
    int source;
    uint64_t milliseconds;
    uint32_t endpoint_count;
    void *successful_endpoint;
    void *preferred_endpoint;
} nw_shim_resolution_step_info;

void *nw_shim_tcp_connect(const char *host, uint16_t port, int use_tls, int *out_status);
int nw_shim_tcp_send(void *handle, const uint8_t *data, size_t len);
ssize_t nw_shim_tcp_receive(void *handle, uint8_t *out_buf, size_t max_len);
void nw_shim_tcp_close(void *handle);

void *nw_shim_listener_create(uint16_t port, int use_tls, int *out_status);
uint16_t nw_shim_listener_port(void *handle);
void *nw_shim_listener_accept(void *handle, int *out_status);
void nw_shim_listener_close(void *handle);

void *nw_shim_udp_connect(const char *host, uint16_t port, int *out_status);

void *nw_shim_path_monitor_start(PathMonitorCallback callback, void *user_info);
void nw_shim_path_monitor_stop(void *handle);
void *nw_shim_path_monitor_copy_latest_path(void *handle);
int nw_shim_path_monitor_enumerate_interfaces(
    void *handle,
    InterfaceEnumerationCallback callback,
    void *user_info
);
int nw_shim_list_interfaces(InterfaceEnumerationCallback callback, void *user_info);

void *nw_shim_browser_start(
    const char *service_type,
    const char *domain,
    BrowserServiceCallback found_callback,
    BrowserServiceCallback lost_callback,
    void *user_info
);
void *nw_shim_browser_start_with_descriptor(
    void *descriptor,
    void *parameters,
    BrowserServiceCallback found_callback,
    BrowserServiceCallback lost_callback,
    void *user_info
);
void nw_shim_browser_stop(void *handle);

void *nw_shim_ws_connect(
    const char *host,
    uint16_t port,
    const char *path,
    int use_tls,
    int *out_status
);
int nw_shim_ws_send(void *handle, const uint8_t *data, size_t len, int opcode);
ssize_t nw_shim_ws_receive(void *handle, uint8_t *out_buf, size_t max_len, int *out_opcode);

void *nw_shim_quic_connect(const char *host, uint16_t port, const char *alpn, int *out_status);

void *nw_shim_bonjour_advertise_start(
    const char *service_type,
    const char *service_name,
    const char *domain,
    uint16_t port,
    int *out_status
);
void *nw_shim_bonjour_advertise_start_with_descriptor(void *descriptor, uint16_t port, int *out_status);
void nw_shim_bonjour_advertise_stop(void *handle);

void *nw_shim_retain_object(void *handle);
void nw_shim_release_object(void *handle);
void nw_shim_free_buffer(void *buffer);

void *nw_shim_parameters_create(void);
void *nw_shim_parameters_create_application_service(void);
void *nw_shim_parameters_create_tcp(int use_tls);
void *nw_shim_parameters_create_udp(void);
void *nw_shim_parameters_create_quic(const char *alpn);
void *nw_shim_parameters_copy(void *handle);
int nw_shim_parameters_prepend_application_protocol(void *parameters, void *protocol_options);
void nw_shim_parameters_set_privacy_context(void *parameters, void *privacy_context);
void nw_shim_parameters_set_prefer_no_proxy(void *parameters, int prefer_no_proxy);
void nw_shim_parameters_set_attribution(void *parameters, int attribution);
int nw_shim_parameters_get_attribution(void *parameters);
void nw_shim_parameters_set_required_interface_type(void *parameters, int interface_type);
int nw_shim_parameters_get_required_interface_type(void *parameters);
void nw_shim_parameters_set_prohibit_expensive(void *parameters, int prohibit_expensive);
int nw_shim_parameters_get_prohibit_expensive(void *parameters);
void nw_shim_parameters_set_prohibit_constrained(void *parameters, int prohibit_constrained);
int nw_shim_parameters_get_prohibit_constrained(void *parameters);
void nw_shim_parameters_set_allow_ultra_constrained(void *parameters, int allow_ultra_constrained);
int nw_shim_parameters_get_allow_ultra_constrained(void *parameters);

void *nw_shim_connection_create_with_parameters(
    const char *host,
    uint16_t port,
    void *parameters,
    int *out_status
);
void *nw_shim_connection_copy_endpoint(void *handle);
void *nw_shim_connection_copy_parameters(void *handle);
void *nw_shim_connection_copy_current_path(void *handle);

void *nw_shim_listener_create_with_parameters(void *parameters, uint16_t port, int *out_status);

void *nw_shim_content_context_create(const char *identifier);
const char *nw_shim_content_context_get_identifier(void *context);
int nw_shim_content_context_get_is_final(void *context);
void nw_shim_content_context_set_is_final(void *context, int is_final);
uint64_t nw_shim_content_context_get_expiration_milliseconds(void *context);
void nw_shim_content_context_set_expiration_milliseconds(void *context, uint64_t expiration_milliseconds);
double nw_shim_content_context_get_relative_priority(void *context);
void nw_shim_content_context_set_relative_priority(void *context, double relative_priority);
void nw_shim_content_context_set_antecedent(void *context, void *antecedent);
void *nw_shim_content_context_copy_antecedent(void *context);
void nw_shim_content_context_set_protocol_metadata(void *context, void *metadata);
void *nw_shim_content_context_copy_protocol_metadata_for_options(void *context, void *protocol_options);

int nw_shim_connection_send_with_context(void *handle, const uint8_t *data, size_t len, void *context);
ssize_t nw_shim_connection_receive_with_context(
    void *handle,
    uint8_t *out_buf,
    size_t max_len,
    void **out_context,
    int *out_is_complete
);

void *nw_shim_privacy_context_create(const char *description);
void *nw_shim_privacy_context_copy_default(void);
void nw_shim_privacy_context_flush_cache(void *privacy_context);
void nw_shim_privacy_context_disable_logging(void *privacy_context);
void nw_shim_privacy_context_require_encrypted_name_resolution(
    void *privacy_context,
    int require_encrypted_name_resolution,
    void *fallback_resolver_config
);
void nw_shim_privacy_context_add_proxy(void *privacy_context, void *proxy_config);
void nw_shim_privacy_context_clear_proxies(void *privacy_context);

void *nw_shim_resolver_config_create_https(const char *url);
void *nw_shim_resolver_config_create_tls(const char *host, uint16_t port);
int nw_shim_resolver_config_add_server_address(void *resolver_config, const char *address, uint16_t port);

void *nw_shim_proxy_config_create_http_connect(const char *host, uint16_t port, int use_tls);
void *nw_shim_proxy_config_create_socksv5(const char *host, uint16_t port);
void *nw_shim_relay_hop_create(void *http3_endpoint, void *http2_endpoint, void *relay_tls_options);
void nw_shim_relay_hop_add_additional_http_header_field(
    void *relay_hop,
    const char *field_name,
    const char *field_value
);
void *nw_shim_proxy_config_create_relay(void *first_hop, void *second_hop);
void *nw_shim_proxy_config_create_oblivious_http(
    void *relay_hop,
    const char *relay_resource_path,
    const uint8_t *gateway_key_config,
    size_t gateway_key_config_length
);
void nw_shim_proxy_config_set_username_password(void *proxy_config, const char *username, const char *password);
void nw_shim_proxy_config_set_failover_allowed(void *proxy_config, int failover_allowed);
int nw_shim_proxy_config_get_failover_allowed(void *proxy_config);
void nw_shim_proxy_config_add_match_domain(void *proxy_config, const char *domain);
void nw_shim_proxy_config_clear_match_domains(void *proxy_config);
void nw_shim_proxy_config_add_excluded_domain(void *proxy_config, const char *domain);
void nw_shim_proxy_config_clear_excluded_domains(void *proxy_config);
void nw_shim_proxy_config_enumerate_match_domains(
    void *proxy_config,
    StringEnumerationCallback callback,
    void *user_info
);
void nw_shim_proxy_config_enumerate_excluded_domains(
    void *proxy_config,
    StringEnumerationCallback callback,
    void *user_info
);

void *nw_shim_framer_definition_create(
    const char *identifier,
    uint32_t flags,
    FramerCreateInstanceCallback create_instance,
    FramerDropInstanceCallback drop_instance,
    FramerStartCallback start_callback,
    FramerInputCallback input_callback,
    FramerOutputCallback output_callback,
    FramerWakeupCallback wakeup_callback,
    FramerStopCallback stop_callback,
    FramerCleanupCallback cleanup_callback,
    void *user_info
);
void *nw_shim_framer_create_options(void *definition);
void *nw_shim_framer_message_create_from_options(void *protocol_options);
void *nw_shim_framer_message_create_for_instance(void *framer);
int nw_shim_framer_message_set_u64(void *message, const char *key, uint64_t value);
int nw_shim_framer_message_get_u64(void *message, const char *key, uint64_t *out_value);
int nw_shim_framer_parse_input(
    void *framer,
    size_t minimum_incomplete_length,
    size_t maximum_length,
    uint8_t *temp_buffer,
    FramerParseCallback parse_callback,
    void *user_info
);
int nw_shim_framer_parse_output(
    void *framer,
    size_t minimum_incomplete_length,
    size_t maximum_length,
    uint8_t *temp_buffer,
    FramerParseCallback parse_callback,
    void *user_info
);
void nw_shim_framer_mark_ready(void *framer);
int nw_shim_framer_prepend_application_protocol(void *framer, void *protocol_options);
void nw_shim_framer_mark_failed_with_error(void *framer, int error_code);
int nw_shim_framer_pass_input_data(void *framer, size_t input_length, void *message, int is_complete);
void nw_shim_framer_deliver_input_data(
    void *framer,
    const uint8_t *input_buffer,
    size_t input_length,
    void *message,
    int is_complete
);
void nw_shim_framer_pass_through_input(void *framer);
int nw_shim_framer_pass_output_data(void *framer, size_t output_length);
void nw_shim_framer_write_output_data(void *framer, const uint8_t *output_buffer, size_t output_length);
void nw_shim_framer_pass_through_output(void *framer);
void nw_shim_framer_schedule_wakeup(void *framer, uint64_t milliseconds);
void nw_shim_framer_async(void *framer, FramerAsyncCallback async_callback, void *user_info);

void *nw_shim_group_descriptor_create_multiplex(const char *host, uint16_t port);
void *nw_shim_group_descriptor_create_multicast(const char *group_address, uint16_t port);
int nw_shim_group_descriptor_add_endpoint(void *descriptor, const char *host, uint16_t port);

void *nw_shim_connection_group_create(void *descriptor, void *parameters);
void nw_shim_connection_group_set_state_changed_handler(
    void *handle,
    ConnectionGroupStateCallback state_callback,
    void *user_info
);
void nw_shim_connection_group_set_receive_handler(
    void *handle,
    uint32_t maximum_message_size,
    int reject_oversized_messages,
    ConnectionGroupReceiveCallback receive_callback,
    void *user_info
);
int nw_shim_connection_group_start(void *handle);
void nw_shim_connection_group_cancel(void *handle);
int nw_shim_connection_group_send(
    void *handle,
    const uint8_t *data,
    size_t len,
    const char *host,
    uint16_t port,
    void *context
);
void nw_shim_connection_group_release(void *handle);

void *nw_shim_endpoint_create_host(const char *host, uint16_t port);
void *nw_shim_endpoint_create_address(const char *address, uint16_t port);
void *nw_shim_endpoint_create_bonjour_service(const char *name, const char *type, const char *domain);
void *nw_shim_endpoint_create_url(const char *url);
int nw_shim_endpoint_get_type(void *endpoint);
char *nw_shim_endpoint_copy_hostname(void *endpoint);
char *nw_shim_endpoint_copy_port_string(void *endpoint);
uint16_t nw_shim_endpoint_get_port(void *endpoint);
char *nw_shim_endpoint_copy_address_string(void *endpoint);
char *nw_shim_endpoint_copy_bonjour_service_name(void *endpoint);
char *nw_shim_endpoint_copy_bonjour_service_type(void *endpoint);
char *nw_shim_endpoint_copy_bonjour_service_domain(void *endpoint);
char *nw_shim_endpoint_copy_url(void *endpoint);
uint8_t *nw_shim_endpoint_copy_signature(void *endpoint, size_t *out_signature_length);

int nw_shim_path_get_status(void *path);
int nw_shim_path_get_unsatisfied_reason(void *path);
int nw_shim_path_is_equal(void *path, void *other_path);
int nw_shim_path_is_expensive(void *path);
int nw_shim_path_is_constrained(void *path);
int nw_shim_path_is_ultra_constrained(void *path);
int nw_shim_path_has_ipv4(void *path);
int nw_shim_path_has_ipv6(void *path);
int nw_shim_path_has_dns(void *path);
int nw_shim_path_uses_interface_type(void *path, int interface_type);
void *nw_shim_path_copy_effective_local_endpoint(void *path);
void *nw_shim_path_copy_effective_remote_endpoint(void *path);
int nw_shim_path_get_link_quality(void *path);
int nw_shim_path_enumerate_interfaces(void *path, InterfaceEnumerationCallback callback, void *user_info);

void *nw_shim_browse_descriptor_create_bonjour_service(const char *type, const char *domain);
char *nw_shim_browse_descriptor_copy_bonjour_service_type(void *descriptor);
char *nw_shim_browse_descriptor_copy_bonjour_service_domain(void *descriptor);
void nw_shim_browse_descriptor_set_include_txt_record(void *descriptor, int include_txt_record);
int nw_shim_browse_descriptor_get_include_txt_record(void *descriptor);
void *nw_shim_browse_descriptor_create_application_service(const char *application_service_name);
char *nw_shim_browse_descriptor_copy_application_service_name(void *descriptor);

void *nw_shim_advertise_descriptor_create_bonjour_service(const char *name, const char *type, const char *domain);
void *nw_shim_advertise_descriptor_create_application_service(const char *application_service_name);
void nw_shim_advertise_descriptor_set_txt_record(void *descriptor, const uint8_t *txt_record, size_t txt_length);
void nw_shim_advertise_descriptor_set_no_auto_rename(void *descriptor, int no_auto_rename);
int nw_shim_advertise_descriptor_get_no_auto_rename(void *descriptor);
char *nw_shim_advertise_descriptor_copy_application_service_name(void *descriptor);

void *nw_shim_protocol_copy_tcp_definition(void);
void *nw_shim_protocol_copy_udp_definition(void);
void *nw_shim_protocol_copy_tls_definition(void);
void *nw_shim_protocol_copy_ip_definition(void);
void *nw_shim_protocol_copy_ws_definition(void);
void *nw_shim_protocol_copy_quic_definition(void);
void *nw_shim_protocol_create_tcp_options(void);
void *nw_shim_protocol_create_udp_options(void);
void *nw_shim_protocol_create_tls_options(void);
void *nw_shim_protocol_create_ip_options(void);
void *nw_shim_protocol_create_ws_options(void);
void *nw_shim_protocol_create_quic_options(void);
int nw_shim_protocol_definition_is_equal(void *definition1, void *definition2);
void *nw_shim_protocol_options_copy_definition(void *options);
void *nw_shim_protocol_metadata_copy_definition(void *metadata);
int nw_shim_protocol_options_is_quic(void *options);

void nw_shim_quic_add_tls_application_protocol(void *options, const char *application_protocol);
int nw_shim_quic_get_stream_is_unidirectional(void *options);
void nw_shim_quic_set_stream_is_unidirectional(void *options, int is_unidirectional);
int nw_shim_quic_get_stream_is_datagram(void *options);
void nw_shim_quic_set_stream_is_datagram(void *options, int is_datagram);
uint64_t nw_shim_quic_get_initial_max_data(void *options);
void nw_shim_quic_set_initial_max_data(void *options, uint64_t initial_max_data);
uint16_t nw_shim_quic_get_max_udp_payload_size(void *options);
void nw_shim_quic_set_max_udp_payload_size(void *options, uint16_t max_udp_payload_size);
uint32_t nw_shim_quic_get_idle_timeout(void *options);
void nw_shim_quic_set_idle_timeout(void *options, uint32_t idle_timeout);
uint64_t nw_shim_quic_get_initial_max_streams_bidirectional(void *options);
void nw_shim_quic_set_initial_max_streams_bidirectional(void *options, uint64_t initial_max_streams_bidirectional);
uint64_t nw_shim_quic_get_initial_max_streams_unidirectional(void *options);
void nw_shim_quic_set_initial_max_streams_unidirectional(void *options, uint64_t initial_max_streams_unidirectional);
uint64_t nw_shim_quic_get_initial_max_stream_data_bidirectional_local(void *options);
void nw_shim_quic_set_initial_max_stream_data_bidirectional_local(void *options, uint64_t initial_max_stream_data_bidirectional_local);
uint64_t nw_shim_quic_get_initial_max_stream_data_bidirectional_remote(void *options);
void nw_shim_quic_set_initial_max_stream_data_bidirectional_remote(void *options, uint64_t initial_max_stream_data_bidirectional_remote);
uint64_t nw_shim_quic_get_initial_max_stream_data_unidirectional(void *options);
void nw_shim_quic_set_initial_max_stream_data_unidirectional(void *options, uint64_t initial_max_stream_data_unidirectional);
uint16_t nw_shim_quic_get_max_datagram_frame_size(void *options);
void nw_shim_quic_set_max_datagram_frame_size(void *options, uint16_t max_datagram_frame_size);
void *nw_shim_connection_copy_quic_metadata(void *handle);
void *nw_shim_content_context_copy_quic_metadata(void *context);
int nw_shim_protocol_metadata_is_quic(void *metadata);
void *nw_shim_quic_copy_sec_protocol_options(void *options);
void *nw_shim_quic_copy_sec_protocol_metadata(void *metadata);
uint64_t nw_shim_quic_get_stream_id(void *metadata);
int nw_shim_quic_get_stream_type(void *metadata);
uint64_t nw_shim_quic_get_stream_application_error(void *metadata);
void nw_shim_quic_set_stream_application_error(void *metadata, uint64_t application_error);
uint64_t nw_shim_quic_get_local_max_streams_bidirectional(void *metadata);
void nw_shim_quic_set_local_max_streams_bidirectional(void *metadata, uint64_t max_streams_bidirectional);
uint64_t nw_shim_quic_get_local_max_streams_unidirectional(void *metadata);
void nw_shim_quic_set_local_max_streams_unidirectional(void *metadata, uint64_t max_streams_unidirectional);
uint64_t nw_shim_quic_get_remote_max_streams_bidirectional(void *metadata);
uint64_t nw_shim_quic_get_remote_max_streams_unidirectional(void *metadata);
uint16_t nw_shim_quic_get_stream_usable_datagram_frame_size(void *metadata);
uint64_t nw_shim_quic_get_application_error(void *metadata);
char *nw_shim_quic_copy_application_error_reason(void *metadata);
void nw_shim_quic_set_application_error(void *metadata, uint64_t application_error, const char *reason);
uint16_t nw_shim_quic_get_keepalive_interval(void *metadata);
void nw_shim_quic_set_keepalive_interval(void *metadata, uint16_t keepalive_interval);
uint64_t nw_shim_quic_get_remote_idle_timeout(void *metadata);

void *nw_shim_sec_retain(void *object);
void nw_shim_sec_release(void *object);

char *nw_shim_interface_copy_name(void *interface);
int nw_shim_interface_get_type(void *interface);
uint32_t nw_shim_interface_get_index(void *interface);

void nw_shim_parameters_require_interface(void *parameters, const char *name, int interface_type, uint32_t index);
int nw_shim_parameters_copy_required_interface(void *parameters, char **out_name, int *out_type, uint32_t *out_index);
void nw_shim_parameters_prohibit_interface(void *parameters, const char *name, int interface_type, uint32_t index);
void nw_shim_parameters_clear_prohibited_interfaces(void *parameters);
void **nw_shim_parameters_copy_prohibited_interfaces(void *parameters, size_t *out_count);
void nw_shim_parameters_prohibit_interface_type(void *parameters, int interface_type);
void nw_shim_parameters_clear_prohibited_interface_types(void *parameters);
int *nw_shim_parameters_copy_prohibited_interface_types(void *parameters, size_t *out_count);
void nw_shim_parameters_set_reuse_local_address(void *parameters, int reuse_local_address);
int nw_shim_parameters_get_reuse_local_address(void *parameters);
void nw_shim_parameters_set_local_endpoint(void *parameters, void *local_endpoint);
void *nw_shim_parameters_copy_local_endpoint(void *parameters);
void nw_shim_parameters_set_include_peer_to_peer(void *parameters, int include_peer_to_peer);
int nw_shim_parameters_get_include_peer_to_peer(void *parameters);
void nw_shim_parameters_set_fast_open_enabled(void *parameters, int fast_open_enabled);
int nw_shim_parameters_get_fast_open_enabled(void *parameters);
void nw_shim_parameters_set_service_class(void *parameters, int service_class);
int nw_shim_parameters_get_service_class(void *parameters);
void nw_shim_parameters_set_multipath_service(void *parameters, int multipath_service);
int nw_shim_parameters_get_multipath_service(void *parameters);
void *nw_shim_parameters_copy_default_protocol_stack(void *parameters);
void nw_shim_protocol_stack_clear_application_protocols(void *stack);
void **nw_shim_protocol_stack_copy_application_protocols(void *stack, size_t *out_count);
void *nw_shim_protocol_stack_copy_transport_protocol(void *stack);
void nw_shim_protocol_stack_set_transport_protocol(void *stack, void *protocol);
void *nw_shim_protocol_stack_copy_internet_protocol(void *stack);
void nw_shim_parameters_set_local_only(void *parameters, int local_only);
int nw_shim_parameters_get_local_only(void *parameters);
int nw_shim_parameters_get_prefer_no_proxy(void *parameters);
void nw_shim_parameters_set_expired_dns_behavior(void *parameters, int expired_dns_behavior);
int nw_shim_parameters_get_expired_dns_behavior(void *parameters);
void nw_shim_parameters_set_requires_dnssec_validation(void *parameters, int requires_dnssec_validation);
int nw_shim_parameters_requires_dnssec_validation(void *parameters);

void *nw_shim_connection_copy_establishment_report(void *handle);
uint64_t nw_shim_establishment_report_get_duration_milliseconds(void *report);
uint64_t nw_shim_establishment_report_get_attempt_started_after_milliseconds(void *report);
uint32_t nw_shim_establishment_report_get_previous_attempt_count(void *report);
int nw_shim_establishment_report_get_used_proxy(void *report);
int nw_shim_establishment_report_get_proxy_configured(void *report);
void *nw_shim_establishment_report_copy_proxy_endpoint(void *report);
nw_shim_establishment_protocol_info *nw_shim_establishment_report_copy_protocols(void *report, size_t *out_count);
nw_shim_resolution_step_info *nw_shim_establishment_report_copy_resolutions(void *report, size_t *out_count);
void **nw_shim_establishment_report_copy_resolution_reports(void *report, size_t *out_count);
int nw_shim_resolution_report_get_source(void *report);
uint64_t nw_shim_resolution_report_get_milliseconds(void *report);
uint32_t nw_shim_resolution_report_get_endpoint_count(void *report);
void *nw_shim_resolution_report_copy_successful_endpoint(void *report);
void *nw_shim_resolution_report_copy_preferred_endpoint(void *report);
int nw_shim_resolution_report_get_protocol(void *report);
void *nw_shim_connection_create_data_transfer_report(void *handle);
int nw_shim_data_transfer_report_collect(void *report);
int nw_shim_data_transfer_report_get_state(void *report);
uint32_t nw_shim_data_transfer_report_all_paths(void);
uint64_t nw_shim_data_transfer_report_get_duration_milliseconds(void *report);
uint32_t nw_shim_data_transfer_report_get_path_count(void *report);
uint64_t nw_shim_data_transfer_report_get_received_ip_packet_count(void *report, uint32_t path_index);
uint64_t nw_shim_data_transfer_report_get_sent_ip_packet_count(void *report, uint32_t path_index);
uint64_t nw_shim_data_transfer_report_get_received_transport_byte_count(void *report, uint32_t path_index);
uint64_t nw_shim_data_transfer_report_get_received_transport_duplicate_byte_count(void *report, uint32_t path_index);
uint64_t nw_shim_data_transfer_report_get_received_transport_out_of_order_byte_count(void *report, uint32_t path_index);
uint64_t nw_shim_data_transfer_report_get_sent_transport_byte_count(void *report, uint32_t path_index);
uint64_t nw_shim_data_transfer_report_get_sent_transport_retransmitted_byte_count(void *report, uint32_t path_index);
uint64_t nw_shim_data_transfer_report_get_transport_smoothed_rtt_milliseconds(void *report, uint32_t path_index);
uint64_t nw_shim_data_transfer_report_get_transport_minimum_rtt_milliseconds(void *report, uint32_t path_index);
uint64_t nw_shim_data_transfer_report_get_transport_rtt_variance(void *report, uint32_t path_index);
uint64_t nw_shim_data_transfer_report_get_received_application_byte_count(void *report, uint32_t path_index);
uint64_t nw_shim_data_transfer_report_get_sent_application_byte_count(void *report, uint32_t path_index);
void *nw_shim_data_transfer_report_copy_path_interface(void *report, uint32_t path_index);
int nw_shim_data_transfer_report_get_path_radio_type(void *report, uint32_t path_index);

size_t nw_shim_endpoint_copy_address(void *endpoint, void *out_buffer, size_t out_buffer_length);
void *nw_shim_endpoint_copy_txt_record(void *endpoint);

void *nw_shim_txt_record_create_with_bytes(const uint8_t *txt_bytes, size_t txt_length);
void *nw_shim_txt_record_create_dictionary(void);
void *nw_shim_txt_record_copy(void *txt_record);
int nw_shim_txt_record_find_key(void *txt_record, const char *key);
uint8_t *nw_shim_txt_record_copy_value(void *txt_record, const char *key, size_t *out_value_length, int *out_found);
int nw_shim_txt_record_set_key(void *txt_record, const char *key, const uint8_t *value, size_t value_length);
int nw_shim_txt_record_remove_key(void *txt_record, const char *key);
size_t nw_shim_txt_record_get_key_count(void *txt_record);
uint8_t *nw_shim_txt_record_copy_bytes(void *txt_record, size_t *out_length);
int nw_shim_txt_record_apply(void *txt_record, TxtRecordEntryCallback callback, void *user_info);
int nw_shim_txt_record_is_dictionary(void *txt_record);
int nw_shim_txt_record_is_equal(void *txt_record, void *other_txt_record);

void *nw_shim_ethernet_channel_create(uint16_t ether_type, const char *name, int interface_type, uint32_t index);
void *nw_shim_ethernet_channel_create_with_parameters(uint16_t ether_type, const char *name, int interface_type, uint32_t index, void *parameters);
void nw_shim_ethernet_channel_set_state_changed_handler(void *handle, EthernetChannelStateCallback callback, void *user_info);
void nw_shim_ethernet_channel_set_receive_handler(void *handle, EthernetChannelReceiveCallback callback, void *user_info);
uint32_t nw_shim_ethernet_channel_get_maximum_payload_size(void *handle);
void nw_shim_ethernet_channel_start(void *handle);
void nw_shim_ethernet_channel_cancel(void *handle);
int nw_shim_ethernet_channel_send(void *handle, const uint8_t *data, size_t len, uint16_t vlan_tag, const uint8_t *remote_address);
void nw_shim_ethernet_channel_release(void *handle);


typedef void (*BrowserStateChangedCallback)(int state, void *error, void *user_info);
typedef void (*BrowseResultChangedCallback)(
    void *old_result,
    void *new_result,
    uint64_t changes,
    int batch_complete,
    void *user_info
);
typedef void (*ConnectionStateCallback)(int state, void *error, void *user_info);
typedef void (*ListenerStateCallback)(int state, void *error, void *user_info);
typedef void (*ListenerNewConnectionCallback)(void *connection_handle, void *user_info);
typedef void (*ConnectionBooleanCallback)(int value, void *user_info);
typedef void (*ConnectionPathCallback)(void *path, void *user_info);
typedef void (*ConnectionBatchCallback)(void *user_info);
typedef int (*ProtocolMetadataEnumerationCallback)(void *definition, void *metadata, void *user_info);
typedef int (*EndpointEnumerationCallback)(void *endpoint, void *user_info);
typedef int (*HeaderEnumerationCallback)(const char *name, const char *value, void *user_info);
typedef void (*PathMonitorCancelCallback)(void *user_info);
typedef void (*ListenerAdvertisedEndpointChangedCallback)(void *endpoint, int added, void *user_info);
typedef void (*ListenerNewConnectionGroupCallback)(void *group, void *user_info);
typedef void (*ConnectionGroupNewConnectionCallback)(void *connection, void *user_info);
typedef void (*WsPongCallback)(void *error, void *user_info);
typedef void *(*WsClientRequestCallback)(void *request, void *user_info);

void nw_shim_framer_message_set_object_value(void *message, const char *key, void *value);
void *nw_shim_framer_message_copy_object_value(void *message, const char *key);
void nw_shim_framer_options_set_object_value(void *options, const char *key, void *value);
void *nw_shim_framer_options_copy_object_value(void *options, const char *key);
void nw_shim_ws_options_set_client_request_handler(
    void *options,
    WsClientRequestCallback callback,
    void *user_info
);
void *nw_shim_url_session_configuration_default(void);
void *nw_shim_url_session_configuration_ephemeral(void);
void nw_shim_url_session_configuration_release(void *configuration);
void nw_shim_url_session_configuration_set_proxy_configurations(
    void *configuration,
    void *const *items,
    size_t count
);
void **nw_shim_url_session_configuration_copy_proxy_configurations(
    void *configuration,
    size_t *out_count
);

void nw_shim_advertise_descriptor_set_txt_record_object(void *descriptor, void *txt_record);
void *nw_shim_advertise_descriptor_copy_txt_record_object(void *descriptor);

void *nw_shim_browser_start_results_with_descriptor(
    void *descriptor,
    void *parameters,
    BrowseResultChangedCallback callback,
    void *user_info
);
void *nw_shim_browser_copy_browse_descriptor(void *handle);
void *nw_shim_browser_copy_parameters(void *handle);
void nw_shim_browser_set_state_changed_handler(
    void *handle,
    BrowserStateChangedCallback callback,
    void *user_info
);
void nw_shim_browser_set_browse_results_changed_handler(
    void *handle,
    BrowseResultChangedCallback callback,
    void *user_info
);
void nw_shim_browser_drain_queue(void *handle);
uint64_t nw_shim_browse_result_get_changes(void *old_result, void *new_result);
void *nw_shim_browse_result_copy_endpoint(void *result);
size_t nw_shim_browse_result_get_interfaces_count(void *result);
void *nw_shim_browse_result_copy_txt_record_object(void *result);
int nw_shim_browse_result_enumerate_interfaces(
    void *result,
    InterfaceEnumerationCallback callback,
    void *user_info
);

void nw_shim_connection_set_state_changed_handler(
    void *handle,
    ConnectionStateCallback callback,
    void *user_info
);
void nw_shim_connection_drain_queue(void *handle);
void nw_shim_connection_set_viability_changed_handler(
    void *handle,
    ConnectionBooleanCallback callback,
    void *user_info
);
void nw_shim_connection_set_better_path_available_handler(
    void *handle,
    ConnectionBooleanCallback callback,
    void *user_info
);
void nw_shim_connection_set_path_changed_handler(
    void *handle,
    ConnectionPathCallback callback,
    void *user_info
);
void nw_shim_connection_restart(void *handle);
void nw_shim_connection_force_cancel(void *handle);
void nw_shim_connection_cancel_current_endpoint(void *handle);
void nw_shim_connection_batch(void *handle, ConnectionBatchCallback callback, void *user_info);
char *nw_shim_connection_copy_description(void *handle);
void *nw_shim_connection_copy_protocol_metadata(void *handle, void *definition);
uint32_t nw_shim_connection_get_maximum_datagram_size(void *handle);
void nw_shim_connection_release_without_cancel(void *handle);

void nw_shim_content_context_foreach_protocol_metadata(
    void *context,
    ProtocolMetadataEnumerationCallback callback,
    void *user_info
);

void *nw_shim_framer_copy_remote_endpoint(void *framer);
void *nw_shim_framer_copy_local_endpoint(void *framer);
void *nw_shim_framer_copy_parameters(void *framer);
void *nw_shim_framer_copy_options(void *framer);

int nw_shim_group_descriptor_enumerate_endpoints(
    void *descriptor,
    EndpointEnumerationCallback callback,
    void *user_info
);
void nw_shim_multicast_group_descriptor_set_specific_source(void *descriptor, void *endpoint);
int nw_shim_multicast_group_descriptor_get_disable_unicast_traffic(void *descriptor);
void nw_shim_multicast_group_descriptor_set_disable_unicast_traffic(void *descriptor, int disable_unicast_traffic);

void *nw_shim_connection_group_copy_descriptor(void *handle);
void *nw_shim_connection_group_copy_parameters(void *handle);
void *nw_shim_connection_group_copy_remote_endpoint_for_message(void *handle, void *context);
void *nw_shim_connection_group_copy_local_endpoint_for_message(void *handle, void *context);
void *nw_shim_connection_group_copy_path_for_message(void *handle, void *context);
void *nw_shim_connection_group_copy_protocol_metadata_for_message(
    void *handle,
    void *context,
    void *definition
);
void *nw_shim_connection_group_copy_protocol_metadata(void *handle, void *definition);
void *nw_shim_connection_group_extract_connection_for_message(
    void *handle,
    void *context,
    int *out_status
);
void *nw_shim_connection_group_extract_connection(
    void *handle,
    void *endpoint,
    void *protocol_options,
    int *out_status
);
int nw_shim_connection_group_reinsert_extracted_connection(void *handle, void *connection_handle);
int nw_shim_connection_group_reply(
    void *handle,
    void *inbound_message,
    void *outbound_message,
    const uint8_t *data,
    size_t len
);
void nw_shim_connection_group_set_new_connection_handler(
    void *handle,
    ConnectionGroupNewConnectionCallback callback,
    void *user_info
);

int nw_shim_error_get_domain(void *error);
int nw_shim_error_get_code(void *error);
void *nw_shim_error_copy_cf_error(void *error);
char *nw_shim_error_copy_cf_error_domain(void *error);
char *nw_shim_error_copy_cf_error_description(void *error);
char *nw_shim_error_copy_posix_domain(void);
char *nw_shim_error_copy_dns_domain(void);
char *nw_shim_error_copy_tls_domain(void);
char *nw_shim_error_copy_wifi_aware_domain(void);

void *nw_shim_parameters_create_custom_ip(uint8_t protocol_number);

void *nw_shim_listener_create_direct(void *parameters, int *out_status);
void *nw_shim_listener_create_with_connection(void *connection_handle, void *parameters, int *out_status);
void *nw_shim_listener_create_with_launchd_key(void *parameters, const char *launchd_key, int *out_status);
uint32_t nw_shim_listener_get_new_connection_limit(void *handle);
void nw_shim_listener_set_new_connection_limit(void *handle, uint32_t new_connection_limit);
void nw_shim_listener_set_advertised_endpoint_changed_handler(
    void *handle,
    ListenerAdvertisedEndpointChangedCallback callback,
    void *user_info
);
void nw_shim_listener_set_new_connection_group_handler(
    void *handle,
    ListenerNewConnectionGroupCallback callback,
    void *user_info
);
void nw_shim_listener_set_state_changed_handler(
    void *handle,
    ListenerStateCallback callback,
    void *user_info
);
void nw_shim_listener_set_new_connection_handler(
    void *handle,
    ListenerNewConnectionCallback callback,
    void *user_info
);
void nw_shim_listener_drain_queue(void *handle);

int nw_shim_path_enumerate_gateways(void *path, EndpointEnumerationCallback callback, void *user_info);
void *nw_shim_path_monitor_start_with_type(
    int interface_type,
    PathMonitorCallback callback,
    void *user_info
);
void *nw_shim_path_monitor_start_for_ethernet_channel(
    PathMonitorCallback callback,
    void *user_info
);
void nw_shim_path_monitor_prohibit_interface_type(void *handle, int interface_type);
void nw_shim_path_monitor_set_cancel_handler(
    void *handle,
    PathMonitorCancelCallback callback,
    void *user_info
);
void nw_shim_path_monitor_set_update_handler(
    void *handle,
    ConnectionPathCallback callback,
    void *user_info
);
void nw_shim_path_monitor_drain_queue(void *handle);

void *nw_shim_protocol_create_ip_metadata(void);
void *nw_shim_protocol_create_udp_metadata(void);
void *nw_shim_protocol_create_ws_options_with_version(int version);
void *nw_shim_protocol_create_ws_metadata(int opcode);
int nw_shim_protocol_metadata_is_ip(void *metadata);
int nw_shim_protocol_metadata_is_tcp(void *metadata);
int nw_shim_protocol_metadata_is_tls(void *metadata);
int nw_shim_protocol_metadata_is_udp(void *metadata);
int nw_shim_protocol_metadata_is_ws(void *metadata);

void nw_shim_ip_options_set_version(void *options, int version);
void nw_shim_ip_options_set_hop_limit(void *options, uint8_t hop_limit);
void nw_shim_ip_options_set_use_minimum_mtu(void *options, int use_minimum_mtu);
void nw_shim_ip_options_set_disable_fragmentation(void *options, int disable_fragmentation);
void nw_shim_ip_options_set_calculate_receive_time(void *options, int calculate_receive_time);
void nw_shim_ip_options_set_local_address_preference(void *options, int preference);
void nw_shim_ip_options_set_disable_multicast_loopback(void *options, int disable_multicast_loopback);
void nw_shim_ip_metadata_set_ecn_flag(void *metadata, int ecn_flag);
int nw_shim_ip_metadata_get_ecn_flag(void *metadata);
void nw_shim_ip_metadata_set_service_class(void *metadata, int service_class);
int nw_shim_ip_metadata_get_service_class(void *metadata);
uint64_t nw_shim_ip_metadata_get_receive_time(void *metadata);

void nw_shim_tcp_options_set_no_delay(void *options, int no_delay);
void nw_shim_tcp_options_set_no_push(void *options, int no_push);
void nw_shim_tcp_options_set_no_options(void *options, int no_options);
void nw_shim_tcp_options_set_enable_keepalive(void *options, int enable_keepalive);
void nw_shim_tcp_options_set_keepalive_count(void *options, uint32_t keepalive_count);
void nw_shim_tcp_options_set_keepalive_idle_time(void *options, uint32_t keepalive_idle_time);
void nw_shim_tcp_options_set_keepalive_interval(void *options, uint32_t keepalive_interval);
void nw_shim_tcp_options_set_maximum_segment_size(void *options, uint32_t maximum_segment_size);
void nw_shim_tcp_options_set_connection_timeout(void *options, uint32_t connection_timeout);
void nw_shim_tcp_options_set_persist_timeout(void *options, uint32_t persist_timeout);
void nw_shim_tcp_options_set_retransmit_connection_drop_time(void *options, uint32_t value);
void nw_shim_tcp_options_set_retransmit_fin_drop(void *options, int retransmit_fin_drop);
void nw_shim_tcp_options_set_disable_ack_stretching(void *options, int disable_ack_stretching);
void nw_shim_tcp_options_set_enable_fast_open(void *options, int enable_fast_open);
void nw_shim_tcp_options_set_disable_ecn(void *options, int disable_ecn);
void nw_shim_tcp_options_set_multipath_force_version(void *options, int multipath_force_version);
uint32_t nw_shim_tcp_get_available_receive_buffer(void *metadata);
uint32_t nw_shim_tcp_get_available_send_buffer(void *metadata);

void *nw_shim_tls_copy_sec_protocol_options(void *options);
void *nw_shim_tls_copy_sec_protocol_metadata(void *metadata);

void nw_shim_udp_options_set_prefer_no_checksum(void *options, int prefer_no_checksum);

void nw_shim_ws_options_add_additional_header(void *options, const char *name, const char *value);
void nw_shim_ws_options_add_subprotocol(void *options, const char *subprotocol);
void nw_shim_ws_options_set_auto_reply_ping(void *options, int auto_reply_ping);
void nw_shim_ws_options_set_skip_handshake(void *options, int skip_handshake);
void nw_shim_ws_options_set_maximum_message_size(void *options, size_t maximum_message_size);
void nw_shim_ws_metadata_set_close_code(void *metadata, int close_code);
int nw_shim_ws_metadata_get_close_code(void *metadata);
void *nw_shim_ws_metadata_copy_server_response(void *metadata);
void nw_shim_ws_metadata_set_pong_handler(
    void *metadata,
    WsPongCallback callback,
    void *user_info
);
int nw_shim_ws_request_enumerate_subprotocols(
    void *request,
    StringEnumerationCallback callback,
    void *user_info
);
int nw_shim_ws_request_enumerate_additional_headers(
    void *request,
    HeaderEnumerationCallback callback,
    void *user_info
);
void *nw_shim_ws_response_create(int status, const char *selected_subprotocol);
int nw_shim_ws_response_get_status(void *response);
char *nw_shim_ws_response_get_selected_subprotocol(void *response);
void nw_shim_ws_response_add_additional_header(void *response, const char *name, const char *value);
int nw_shim_ws_response_enumerate_additional_headers(
    void *response,
    HeaderEnumerationCallback callback,
    void *user_info
);

#ifdef __cplusplus
}
#endif

#endif