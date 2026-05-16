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

#ifdef __cplusplus
}
#endif

#endif