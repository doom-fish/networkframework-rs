//! Swift-bridge FFI declarations.

#![allow(missing_docs)]
#![cfg_attr(not(feature = "raw-ffi"), allow(dead_code))]

use core::ffi::{c_char, c_int, c_void};

pub const NW_OK: c_int = 0;
pub const NW_INVALID_ARG: c_int = -1;
pub const NW_CONNECT_FAILED: c_int = -2;
pub const NW_SEND_FAILED: c_int = -3;
pub const NW_RECV_FAILED: c_int = -4;
pub const NW_LISTEN_FAILED: c_int = -5;
pub const NW_CANCELLED: c_int = -6;
pub const NW_TIMEOUT: c_int = -7;

pub const NW_FRAMER_START_READY: c_int = 1;
pub const NW_FRAMER_START_WILL_MARK_READY: c_int = 2;

pub type BrowserServiceCallback = unsafe extern "C" fn(
    name: *const c_char,
    service_type: *const c_char,
    domain: *const c_char,
    user_info: *mut c_void,
);

pub type PathMonitorCallback =
    unsafe extern "C" fn(satisfied: c_int, interface_type: c_int, user_info: *mut c_void);

pub type InterfaceEnumerationCallback = unsafe extern "C" fn(
    name: *const c_char,
    interface_type: c_int,
    index: u32,
    user_info: *mut c_void,
) -> c_int;

pub type StringEnumerationCallback =
    unsafe extern "C" fn(value: *const c_char, user_info: *mut c_void);

pub type FramerCreateInstanceCallback = unsafe extern "C" fn(user_info: *mut c_void) -> *mut c_void;
pub type FramerDropInstanceCallback = unsafe extern "C" fn(instance: *mut c_void);
pub type FramerStartCallback =
    unsafe extern "C" fn(instance: *mut c_void, framer: *mut c_void) -> c_int;
pub type FramerInputCallback =
    unsafe extern "C" fn(instance: *mut c_void, framer: *mut c_void) -> usize;
pub type FramerOutputCallback = unsafe extern "C" fn(
    instance: *mut c_void,
    framer: *mut c_void,
    message: *mut c_void,
    message_length: usize,
    is_complete: c_int,
);
pub type FramerWakeupCallback = unsafe extern "C" fn(instance: *mut c_void, framer: *mut c_void);
pub type FramerStopCallback =
    unsafe extern "C" fn(instance: *mut c_void, framer: *mut c_void) -> c_int;
pub type FramerCleanupCallback = unsafe extern "C" fn(instance: *mut c_void, framer: *mut c_void);
pub type FramerParseCallback = unsafe extern "C" fn(
    buffer: *const u8,
    buffer_length: usize,
    is_complete: c_int,
    user_info: *mut c_void,
) -> usize;
pub type FramerAsyncCallback = unsafe extern "C" fn(framer: *mut c_void, user_info: *mut c_void);

pub type ConnectionGroupStateCallback = unsafe extern "C" fn(state: c_int, user_info: *mut c_void);
pub type ConnectionGroupReceiveCallback = unsafe extern "C" fn(
    data: *const u8,
    len: usize,
    context: *mut c_void,
    is_complete: c_int,
    user_info: *mut c_void,
);

mod advanced;

pub use advanced::*;

unsafe extern "C" {
    #[link_name = "nfw_tcp_connect"]
    pub fn nw_shim_tcp_connect(
        host: *const c_char,
        port: u16,
        use_tls: c_int,
        out_status: *mut c_int,
    ) -> *mut c_void;
    #[link_name = "nfw_tcp_send"]
    pub fn nw_shim_tcp_send(handle: *mut c_void, data: *const u8, len: usize) -> c_int;
    #[link_name = "nfw_tcp_receive"]
    pub fn nw_shim_tcp_receive(handle: *mut c_void, out_buf: *mut u8, max_len: usize) -> isize;
    #[link_name = "nfw_tcp_close"]
    pub fn nw_shim_tcp_close(handle: *mut c_void);

    #[link_name = "nfw_listener_create"]
    pub fn nw_shim_listener_create(
        port: u16,
        use_tls: c_int,
        out_status: *mut c_int,
    ) -> *mut c_void;
    #[link_name = "nfw_listener_port"]
    pub fn nw_shim_listener_port(handle: *mut c_void) -> u16;
    #[link_name = "nfw_listener_accept"]
    pub fn nw_shim_listener_accept(handle: *mut c_void, out_status: *mut c_int) -> *mut c_void;
    #[link_name = "nfw_listener_close"]
    pub fn nw_shim_listener_close(handle: *mut c_void);
    #[link_name = "nfw_listener_create_with_parameters"]
    pub fn nw_shim_listener_create_with_parameters(
        parameters: *mut c_void,
        port: u16,
        out_status: *mut c_int,
    ) -> *mut c_void;

    #[link_name = "nfw_udp_connect"]
    pub fn nw_shim_udp_connect(
        host: *const c_char,
        port: u16,
        out_status: *mut c_int,
    ) -> *mut c_void;

    #[link_name = "nfw_path_monitor_start"]
    pub fn nw_shim_path_monitor_start(
        callback: PathMonitorCallback,
        user_info: *mut c_void,
    ) -> *mut c_void;
    #[link_name = "nfw_path_monitor_stop"]
    pub fn nw_shim_path_monitor_stop(handle: *mut c_void);
    #[link_name = "nfw_path_monitor_copy_latest_path"]
    pub fn nw_shim_path_monitor_copy_latest_path(handle: *mut c_void) -> *mut c_void;
    #[link_name = "nfw_path_monitor_enumerate_interfaces"]
    pub fn nw_shim_path_monitor_enumerate_interfaces(
        handle: *mut c_void,
        callback: InterfaceEnumerationCallback,
        user_info: *mut c_void,
    ) -> c_int;
    #[link_name = "nfw_list_interfaces"]
    pub fn nw_shim_list_interfaces(
        callback: InterfaceEnumerationCallback,
        user_info: *mut c_void,
    ) -> c_int;

    #[link_name = "nfw_browser_start"]
    pub fn nw_shim_browser_start(
        service_type: *const c_char,
        domain: *const c_char,
        found_callback: BrowserServiceCallback,
        lost_callback: BrowserServiceCallback,
        user_info: *mut c_void,
    ) -> *mut c_void;
    #[link_name = "nfw_browser_start_with_descriptor"]
    pub fn nw_shim_browser_start_with_descriptor(
        descriptor: *mut c_void,
        parameters: *mut c_void,
        found_callback: BrowserServiceCallback,
        lost_callback: BrowserServiceCallback,
        user_info: *mut c_void,
    ) -> *mut c_void;
    #[link_name = "nfw_browser_stop"]
    pub fn nw_shim_browser_stop(handle: *mut c_void);

    #[link_name = "nfw_ws_connect"]
    pub fn nw_shim_ws_connect(
        host: *const c_char,
        port: u16,
        path: *const c_char,
        use_tls: c_int,
        out_status: *mut c_int,
    ) -> *mut c_void;
    #[link_name = "nfw_ws_send"]
    pub fn nw_shim_ws_send(
        handle: *mut c_void,
        data: *const u8,
        len: usize,
        opcode: c_int,
    ) -> c_int;
    #[link_name = "nfw_ws_receive"]
    pub fn nw_shim_ws_receive(
        handle: *mut c_void,
        out_buf: *mut u8,
        max_len: usize,
        out_opcode: *mut c_int,
    ) -> isize;

    #[link_name = "nfw_quic_connect"]
    pub fn nw_shim_quic_connect(
        host: *const c_char,
        port: u16,
        alpn: *const c_char,
        out_status: *mut c_int,
    ) -> *mut c_void;

    #[link_name = "nfw_bonjour_advertise_start"]
    pub fn nw_shim_bonjour_advertise_start(
        service_type: *const c_char,
        service_name: *const c_char,
        domain: *const c_char,
        port: u16,
        out_status: *mut c_int,
    ) -> *mut c_void;
    #[link_name = "nfw_bonjour_advertise_start_with_descriptor"]
    pub fn nw_shim_bonjour_advertise_start_with_descriptor(
        descriptor: *mut c_void,
        port: u16,
        out_status: *mut c_int,
    ) -> *mut c_void;
    #[link_name = "nfw_bonjour_advertise_stop"]
    pub fn nw_shim_bonjour_advertise_stop(handle: *mut c_void);

    #[link_name = "nfw_retain_object"]
    pub fn nw_shim_retain_object(handle: *mut c_void) -> *mut c_void;
    #[link_name = "nfw_release_object"]
    pub fn nw_shim_release_object(handle: *mut c_void);
    #[link_name = "nfw_free_buffer"]
    pub fn nw_shim_free_buffer(handle: *mut c_void);

    #[link_name = "nfw_parameters_create"]
    pub fn nw_shim_parameters_create() -> *mut c_void;
    #[link_name = "nfw_parameters_create_application_service"]
    pub fn nw_shim_parameters_create_application_service() -> *mut c_void;
    #[link_name = "nfw_parameters_create_tcp"]
    pub fn nw_shim_parameters_create_tcp(use_tls: c_int) -> *mut c_void;
    #[link_name = "nfw_parameters_create_udp"]
    pub fn nw_shim_parameters_create_udp() -> *mut c_void;
    #[link_name = "nfw_parameters_create_quic"]
    pub fn nw_shim_parameters_create_quic(alpn: *const c_char) -> *mut c_void;
    #[link_name = "nfw_parameters_copy"]
    pub fn nw_shim_parameters_copy(handle: *mut c_void) -> *mut c_void;
    #[link_name = "nfw_parameters_prepend_application_protocol"]
    pub fn nw_shim_parameters_prepend_application_protocol(
        parameters: *mut c_void,
        protocol_options: *mut c_void,
    ) -> c_int;
    #[link_name = "nfw_parameters_set_privacy_context"]
    pub fn nw_shim_parameters_set_privacy_context(
        parameters: *mut c_void,
        privacy_context: *mut c_void,
    );
    #[link_name = "nfw_parameters_set_prefer_no_proxy"]
    pub fn nw_shim_parameters_set_prefer_no_proxy(parameters: *mut c_void, prefer_no_proxy: c_int);
    #[link_name = "nfw_parameters_set_attribution"]
    pub fn nw_shim_parameters_set_attribution(parameters: *mut c_void, attribution: c_int);
    #[link_name = "nfw_parameters_get_attribution"]
    pub fn nw_shim_parameters_get_attribution(parameters: *mut c_void) -> c_int;
    #[link_name = "nfw_parameters_set_required_interface_type"]
    pub fn nw_shim_parameters_set_required_interface_type(
        parameters: *mut c_void,
        interface_type: c_int,
    );
    #[link_name = "nfw_parameters_get_required_interface_type"]
    pub fn nw_shim_parameters_get_required_interface_type(parameters: *mut c_void) -> c_int;
    #[link_name = "nfw_parameters_set_prohibit_expensive"]
    pub fn nw_shim_parameters_set_prohibit_expensive(
        parameters: *mut c_void,
        prohibit_expensive: c_int,
    );
    #[link_name = "nfw_parameters_get_prohibit_expensive"]
    pub fn nw_shim_parameters_get_prohibit_expensive(parameters: *mut c_void) -> c_int;
    #[link_name = "nfw_parameters_set_prohibit_constrained"]
    pub fn nw_shim_parameters_set_prohibit_constrained(
        parameters: *mut c_void,
        prohibit_constrained: c_int,
    );
    #[link_name = "nfw_parameters_get_prohibit_constrained"]
    pub fn nw_shim_parameters_get_prohibit_constrained(parameters: *mut c_void) -> c_int;
    #[link_name = "nfw_parameters_set_allow_ultra_constrained"]
    pub fn nw_shim_parameters_set_allow_ultra_constrained(
        parameters: *mut c_void,
        allow_ultra_constrained: c_int,
    );
    #[link_name = "nfw_parameters_get_allow_ultra_constrained"]
    pub fn nw_shim_parameters_get_allow_ultra_constrained(parameters: *mut c_void) -> c_int;

    #[link_name = "nfw_connection_create_with_parameters"]
    pub fn nw_shim_connection_create_with_parameters(
        host: *const c_char,
        port: u16,
        parameters: *mut c_void,
        out_status: *mut c_int,
    ) -> *mut c_void;
    #[link_name = "nfw_connection_copy_endpoint"]
    pub fn nw_shim_connection_copy_endpoint(handle: *mut c_void) -> *mut c_void;
    #[link_name = "nfw_connection_copy_parameters"]
    pub fn nw_shim_connection_copy_parameters(handle: *mut c_void) -> *mut c_void;
    #[link_name = "nfw_connection_copy_current_path"]
    pub fn nw_shim_connection_copy_current_path(handle: *mut c_void) -> *mut c_void;

    #[link_name = "nfw_content_context_create"]
    pub fn nw_shim_content_context_create(identifier: *const c_char) -> *mut c_void;
    #[link_name = "nfw_content_context_get_identifier"]
    pub fn nw_shim_content_context_get_identifier(context: *mut c_void) -> *const c_char;
    #[link_name = "nfw_content_context_get_is_final"]
    pub fn nw_shim_content_context_get_is_final(context: *mut c_void) -> c_int;
    #[link_name = "nfw_content_context_set_is_final"]
    pub fn nw_shim_content_context_set_is_final(context: *mut c_void, is_final: c_int);
    #[link_name = "nfw_content_context_get_expiration_milliseconds"]
    pub fn nw_shim_content_context_get_expiration_milliseconds(context: *mut c_void) -> u64;
    #[link_name = "nfw_content_context_set_expiration_milliseconds"]
    pub fn nw_shim_content_context_set_expiration_milliseconds(
        context: *mut c_void,
        expiration_milliseconds: u64,
    );
    #[link_name = "nfw_content_context_get_relative_priority"]
    pub fn nw_shim_content_context_get_relative_priority(context: *mut c_void) -> f64;
    #[link_name = "nfw_content_context_set_relative_priority"]
    pub fn nw_shim_content_context_set_relative_priority(
        context: *mut c_void,
        relative_priority: f64,
    );
    #[link_name = "nfw_content_context_set_antecedent"]
    pub fn nw_shim_content_context_set_antecedent(context: *mut c_void, antecedent: *mut c_void);
    #[link_name = "nfw_content_context_copy_antecedent"]
    pub fn nw_shim_content_context_copy_antecedent(context: *mut c_void) -> *mut c_void;
    #[link_name = "nfw_content_context_set_protocol_metadata"]
    pub fn nw_shim_content_context_set_protocol_metadata(
        context: *mut c_void,
        metadata: *mut c_void,
    );
    #[link_name = "nfw_content_context_copy_protocol_metadata_for_options"]
    pub fn nw_shim_content_context_copy_protocol_metadata_for_options(
        context: *mut c_void,
        protocol_options: *mut c_void,
    ) -> *mut c_void;

    #[link_name = "nfw_connection_send_with_context"]
    pub fn nw_shim_connection_send_with_context(
        handle: *mut c_void,
        data: *const u8,
        len: usize,
        context: *mut c_void,
    ) -> c_int;
    #[link_name = "nfw_connection_receive_with_context"]
    pub fn nw_shim_connection_receive_with_context(
        handle: *mut c_void,
        out_buf: *mut u8,
        max_len: usize,
        out_context: *mut *mut c_void,
        out_is_complete: *mut c_int,
    ) -> isize;

    #[link_name = "nfw_privacy_context_create"]
    pub fn nw_shim_privacy_context_create(description: *const c_char) -> *mut c_void;
    #[link_name = "nfw_privacy_context_copy_default"]
    pub fn nw_shim_privacy_context_copy_default() -> *mut c_void;
    #[link_name = "nfw_privacy_context_flush_cache"]
    pub fn nw_shim_privacy_context_flush_cache(privacy_context: *mut c_void);
    #[link_name = "nfw_privacy_context_disable_logging"]
    pub fn nw_shim_privacy_context_disable_logging(privacy_context: *mut c_void);
    #[link_name = "nfw_privacy_context_require_encrypted_name_resolution"]
    pub fn nw_shim_privacy_context_require_encrypted_name_resolution(
        privacy_context: *mut c_void,
        require_encrypted_name_resolution: c_int,
        fallback_resolver_config: *mut c_void,
    );
    #[link_name = "nfw_privacy_context_add_proxy"]
    pub fn nw_shim_privacy_context_add_proxy(
        privacy_context: *mut c_void,
        proxy_config: *mut c_void,
    );
    #[link_name = "nfw_privacy_context_clear_proxies"]
    pub fn nw_shim_privacy_context_clear_proxies(privacy_context: *mut c_void);

    #[link_name = "nfw_resolver_config_create_https"]
    pub fn nw_shim_resolver_config_create_https(url: *const c_char) -> *mut c_void;
    #[link_name = "nfw_resolver_config_create_tls"]
    pub fn nw_shim_resolver_config_create_tls(host: *const c_char, port: u16) -> *mut c_void;
    #[link_name = "nfw_resolver_config_add_server_address"]
    pub fn nw_shim_resolver_config_add_server_address(
        resolver_config: *mut c_void,
        address: *const c_char,
        port: u16,
    ) -> c_int;

    #[link_name = "nfw_proxy_config_create_http_connect"]
    pub fn nw_shim_proxy_config_create_http_connect(
        host: *const c_char,
        port: u16,
        use_tls: c_int,
    ) -> *mut c_void;
    #[link_name = "nfw_proxy_config_create_socksv5"]
    pub fn nw_shim_proxy_config_create_socksv5(host: *const c_char, port: u16) -> *mut c_void;
    #[link_name = "nfw_relay_hop_create"]
    pub fn nw_shim_relay_hop_create(
        http3_endpoint: *mut c_void,
        http2_endpoint: *mut c_void,
        relay_tls_options: *mut c_void,
    ) -> *mut c_void;
    #[link_name = "nfw_relay_hop_add_additional_http_header_field"]
    pub fn nw_shim_relay_hop_add_additional_http_header_field(
        relay_hop: *mut c_void,
        field_name: *const c_char,
        field_value: *const c_char,
    );
    #[link_name = "nfw_proxy_config_create_relay"]
    pub fn nw_shim_proxy_config_create_relay(
        first_hop: *mut c_void,
        second_hop: *mut c_void,
    ) -> *mut c_void;
    #[link_name = "nfw_proxy_config_create_oblivious_http"]
    pub fn nw_shim_proxy_config_create_oblivious_http(
        relay_hop: *mut c_void,
        relay_resource_path: *const c_char,
        gateway_key_config: *const u8,
        gateway_key_config_length: usize,
    ) -> *mut c_void;
    #[link_name = "nfw_proxy_config_set_username_password"]
    pub fn nw_shim_proxy_config_set_username_password(
        proxy_config: *mut c_void,
        username: *const c_char,
        password: *const c_char,
    );
    #[link_name = "nfw_proxy_config_set_failover_allowed"]
    pub fn nw_shim_proxy_config_set_failover_allowed(
        proxy_config: *mut c_void,
        failover_allowed: c_int,
    );
    #[link_name = "nfw_proxy_config_get_failover_allowed"]
    pub fn nw_shim_proxy_config_get_failover_allowed(proxy_config: *mut c_void) -> c_int;
    #[link_name = "nfw_proxy_config_add_match_domain"]
    pub fn nw_shim_proxy_config_add_match_domain(proxy_config: *mut c_void, domain: *const c_char);
    #[link_name = "nfw_proxy_config_clear_match_domains"]
    pub fn nw_shim_proxy_config_clear_match_domains(proxy_config: *mut c_void);
    #[link_name = "nfw_proxy_config_add_excluded_domain"]
    pub fn nw_shim_proxy_config_add_excluded_domain(
        proxy_config: *mut c_void,
        domain: *const c_char,
    );
    #[link_name = "nfw_proxy_config_clear_excluded_domains"]
    pub fn nw_shim_proxy_config_clear_excluded_domains(proxy_config: *mut c_void);
    #[link_name = "nfw_proxy_config_enumerate_match_domains"]
    pub fn nw_shim_proxy_config_enumerate_match_domains(
        proxy_config: *mut c_void,
        callback: StringEnumerationCallback,
        user_info: *mut c_void,
    );
    #[link_name = "nfw_proxy_config_enumerate_excluded_domains"]
    pub fn nw_shim_proxy_config_enumerate_excluded_domains(
        proxy_config: *mut c_void,
        callback: StringEnumerationCallback,
        user_info: *mut c_void,
    );

    #[link_name = "nfw_framer_definition_create"]
    pub fn nw_shim_framer_definition_create(
        identifier: *const c_char,
        flags: u32,
        create_instance: FramerCreateInstanceCallback,
        drop_instance: FramerDropInstanceCallback,
        start_callback: FramerStartCallback,
        input_callback: FramerInputCallback,
        output_callback: FramerOutputCallback,
        wakeup_callback: Option<FramerWakeupCallback>,
        stop_callback: Option<FramerStopCallback>,
        cleanup_callback: Option<FramerCleanupCallback>,
        user_info: *mut c_void,
    ) -> *mut c_void;
    #[link_name = "nfw_framer_create_options"]
    pub fn nw_shim_framer_create_options(definition: *mut c_void) -> *mut c_void;
    #[link_name = "nfw_framer_message_create_from_options"]
    pub fn nw_shim_framer_message_create_from_options(protocol_options: *mut c_void)
        -> *mut c_void;
    #[link_name = "nfw_framer_message_create_for_instance"]
    pub fn nw_shim_framer_message_create_for_instance(framer: *mut c_void) -> *mut c_void;
    #[link_name = "nfw_framer_message_set_u64"]
    pub fn nw_shim_framer_message_set_u64(
        message: *mut c_void,
        key: *const c_char,
        value: u64,
    ) -> c_int;
    #[link_name = "nfw_framer_message_get_u64"]
    pub fn nw_shim_framer_message_get_u64(
        message: *mut c_void,
        key: *const c_char,
        out_value: *mut u64,
    ) -> c_int;
    #[link_name = "nfw_framer_parse_input"]
    pub fn nw_shim_framer_parse_input(
        framer: *mut c_void,
        minimum_incomplete_length: usize,
        maximum_length: usize,
        temp_buffer: *mut u8,
        parse_callback: FramerParseCallback,
        user_info: *mut c_void,
    ) -> c_int;
    #[link_name = "nfw_framer_parse_output"]
    pub fn nw_shim_framer_parse_output(
        framer: *mut c_void,
        minimum_incomplete_length: usize,
        maximum_length: usize,
        temp_buffer: *mut u8,
        parse_callback: FramerParseCallback,
        user_info: *mut c_void,
    ) -> c_int;
    #[link_name = "nfw_framer_mark_ready"]
    pub fn nw_shim_framer_mark_ready(framer: *mut c_void);
    #[link_name = "nfw_framer_prepend_application_protocol"]
    pub fn nw_shim_framer_prepend_application_protocol(
        framer: *mut c_void,
        protocol_options: *mut c_void,
    ) -> c_int;
    #[link_name = "nfw_framer_mark_failed_with_error"]
    pub fn nw_shim_framer_mark_failed_with_error(framer: *mut c_void, error_code: c_int);
    #[link_name = "nfw_framer_pass_input_data"]
    pub fn nw_shim_framer_pass_input_data(
        framer: *mut c_void,
        input_length: usize,
        message: *mut c_void,
        is_complete: c_int,
    ) -> c_int;
    #[link_name = "nfw_framer_deliver_input_data"]
    pub fn nw_shim_framer_deliver_input_data(
        framer: *mut c_void,
        input_buffer: *const u8,
        input_length: usize,
        message: *mut c_void,
        is_complete: c_int,
    );
    #[link_name = "nfw_framer_pass_through_input"]
    pub fn nw_shim_framer_pass_through_input(framer: *mut c_void);
    #[link_name = "nfw_framer_pass_output_data"]
    pub fn nw_shim_framer_pass_output_data(framer: *mut c_void, output_length: usize) -> c_int;
    #[link_name = "nfw_framer_write_output_data"]
    pub fn nw_shim_framer_write_output_data(
        framer: *mut c_void,
        output_buffer: *const u8,
        output_length: usize,
    );
    #[link_name = "nfw_framer_pass_through_output"]
    pub fn nw_shim_framer_pass_through_output(framer: *mut c_void);
    #[link_name = "nfw_framer_schedule_wakeup"]
    pub fn nw_shim_framer_schedule_wakeup(framer: *mut c_void, milliseconds: u64);
    #[link_name = "nfw_framer_async"]
    pub fn nw_shim_framer_async(
        framer: *mut c_void,
        async_callback: FramerAsyncCallback,
        user_info: *mut c_void,
    );

    #[link_name = "nfw_group_descriptor_create_multiplex"]
    pub fn nw_shim_group_descriptor_create_multiplex(host: *const c_char, port: u16)
        -> *mut c_void;
    #[link_name = "nfw_group_descriptor_create_multicast"]
    pub fn nw_shim_group_descriptor_create_multicast(
        group_address: *const c_char,
        port: u16,
    ) -> *mut c_void;
    #[link_name = "nfw_group_descriptor_add_endpoint"]
    pub fn nw_shim_group_descriptor_add_endpoint(
        descriptor: *mut c_void,
        host: *const c_char,
        port: u16,
    ) -> c_int;
    #[link_name = "nfw_connection_group_create"]
    pub fn nw_shim_connection_group_create(
        descriptor: *mut c_void,
        parameters: *mut c_void,
    ) -> *mut c_void;
    #[link_name = "nfw_connection_group_set_state_changed_handler"]
    pub fn nw_shim_connection_group_set_state_changed_handler(
        handle: *mut c_void,
        state_callback: Option<ConnectionGroupStateCallback>,
        user_info: *mut c_void,
    );
    #[link_name = "nfw_connection_group_set_receive_handler"]
    pub fn nw_shim_connection_group_set_receive_handler(
        handle: *mut c_void,
        maximum_message_size: u32,
        reject_oversized_messages: c_int,
        receive_callback: Option<ConnectionGroupReceiveCallback>,
        user_info: *mut c_void,
    );
    #[link_name = "nfw_connection_group_start"]
    pub fn nw_shim_connection_group_start(handle: *mut c_void) -> c_int;
    #[link_name = "nfw_connection_group_cancel"]
    pub fn nw_shim_connection_group_cancel(handle: *mut c_void);
    #[link_name = "nfw_connection_group_send"]
    pub fn nw_shim_connection_group_send(
        handle: *mut c_void,
        data: *const u8,
        len: usize,
        host: *const c_char,
        port: u16,
        context: *mut c_void,
    ) -> c_int;
    #[link_name = "nfw_connection_group_release"]
    pub fn nw_shim_connection_group_release(handle: *mut c_void);

    #[link_name = "nfw_endpoint_create_host"]
    pub fn nw_shim_endpoint_create_host(host: *const c_char, port: u16) -> *mut c_void;
    #[link_name = "nfw_endpoint_create_address"]
    pub fn nw_shim_endpoint_create_address(address: *const c_char, port: u16) -> *mut c_void;
    #[link_name = "nfw_endpoint_create_bonjour_service"]
    pub fn nw_shim_endpoint_create_bonjour_service(
        name: *const c_char,
        service_type: *const c_char,
        domain: *const c_char,
    ) -> *mut c_void;
    #[link_name = "nfw_endpoint_create_url"]
    pub fn nw_shim_endpoint_create_url(url: *const c_char) -> *mut c_void;
    #[link_name = "nfw_endpoint_get_type"]
    pub fn nw_shim_endpoint_get_type(endpoint: *mut c_void) -> c_int;
    #[link_name = "nfw_endpoint_copy_hostname"]
    pub fn nw_shim_endpoint_copy_hostname(endpoint: *mut c_void) -> *mut c_char;
    #[link_name = "nfw_endpoint_copy_port_string"]
    pub fn nw_shim_endpoint_copy_port_string(endpoint: *mut c_void) -> *mut c_char;
    #[link_name = "nfw_endpoint_get_port"]
    pub fn nw_shim_endpoint_get_port(endpoint: *mut c_void) -> u16;
    #[link_name = "nfw_endpoint_copy_address_string"]
    pub fn nw_shim_endpoint_copy_address_string(endpoint: *mut c_void) -> *mut c_char;
    #[link_name = "nfw_endpoint_copy_bonjour_service_name"]
    pub fn nw_shim_endpoint_copy_bonjour_service_name(endpoint: *mut c_void) -> *mut c_char;
    #[link_name = "nfw_endpoint_copy_bonjour_service_type"]
    pub fn nw_shim_endpoint_copy_bonjour_service_type(endpoint: *mut c_void) -> *mut c_char;
    #[link_name = "nfw_endpoint_copy_bonjour_service_domain"]
    pub fn nw_shim_endpoint_copy_bonjour_service_domain(endpoint: *mut c_void) -> *mut c_char;
    #[link_name = "nfw_endpoint_copy_url"]
    pub fn nw_shim_endpoint_copy_url(endpoint: *mut c_void) -> *mut c_char;
    #[link_name = "nfw_endpoint_copy_signature"]
    pub fn nw_shim_endpoint_copy_signature(
        endpoint: *mut c_void,
        out_signature_length: *mut usize,
    ) -> *mut u8;

    #[link_name = "nfw_path_get_status"]
    pub fn nw_shim_path_get_status(path: *mut c_void) -> c_int;
    #[link_name = "nfw_path_get_unsatisfied_reason"]
    pub fn nw_shim_path_get_unsatisfied_reason(path: *mut c_void) -> c_int;
    #[link_name = "nfw_path_is_equal"]
    pub fn nw_shim_path_is_equal(path: *mut c_void, other_path: *mut c_void) -> c_int;
    #[link_name = "nfw_path_is_expensive"]
    pub fn nw_shim_path_is_expensive(path: *mut c_void) -> c_int;
    #[link_name = "nfw_path_is_constrained"]
    pub fn nw_shim_path_is_constrained(path: *mut c_void) -> c_int;
    #[link_name = "nfw_path_is_ultra_constrained"]
    pub fn nw_shim_path_is_ultra_constrained(path: *mut c_void) -> c_int;
    #[link_name = "nfw_path_has_ipv4"]
    pub fn nw_shim_path_has_ipv4(path: *mut c_void) -> c_int;
    #[link_name = "nfw_path_has_ipv6"]
    pub fn nw_shim_path_has_ipv6(path: *mut c_void) -> c_int;
    #[link_name = "nfw_path_has_dns"]
    pub fn nw_shim_path_has_dns(path: *mut c_void) -> c_int;
    #[link_name = "nfw_path_uses_interface_type"]
    pub fn nw_shim_path_uses_interface_type(path: *mut c_void, interface_type: c_int) -> c_int;
    #[link_name = "nfw_path_copy_effective_local_endpoint"]
    pub fn nw_shim_path_copy_effective_local_endpoint(path: *mut c_void) -> *mut c_void;
    #[link_name = "nfw_path_copy_effective_remote_endpoint"]
    pub fn nw_shim_path_copy_effective_remote_endpoint(path: *mut c_void) -> *mut c_void;
    #[link_name = "nfw_path_get_link_quality"]
    pub fn nw_shim_path_get_link_quality(path: *mut c_void) -> c_int;
    #[link_name = "nfw_path_enumerate_interfaces"]
    pub fn nw_shim_path_enumerate_interfaces(
        path: *mut c_void,
        callback: InterfaceEnumerationCallback,
        user_info: *mut c_void,
    ) -> c_int;

    #[link_name = "nfw_advertise_descriptor_create_bonjour_service"]
    pub fn nw_shim_advertise_descriptor_create_bonjour_service(
        name: *const c_char,
        service_type: *const c_char,
        domain: *const c_char,
    ) -> *mut c_void;
    #[link_name = "nfw_advertise_descriptor_create_application_service"]
    pub fn nw_shim_advertise_descriptor_create_application_service(
        application_service_name: *const c_char,
    ) -> *mut c_void;
    #[link_name = "nfw_advertise_descriptor_set_txt_record"]
    pub fn nw_shim_advertise_descriptor_set_txt_record(
        descriptor: *mut c_void,
        txt_record: *const u8,
        txt_length: usize,
    );
    #[link_name = "nfw_advertise_descriptor_set_no_auto_rename"]
    pub fn nw_shim_advertise_descriptor_set_no_auto_rename(
        descriptor: *mut c_void,
        no_auto_rename: c_int,
    );
    #[link_name = "nfw_advertise_descriptor_get_no_auto_rename"]
    pub fn nw_shim_advertise_descriptor_get_no_auto_rename(descriptor: *mut c_void) -> c_int;
    #[link_name = "nfw_advertise_descriptor_copy_application_service_name"]
    pub fn nw_shim_advertise_descriptor_copy_application_service_name(
        descriptor: *mut c_void,
    ) -> *mut c_char;

    #[link_name = "nfw_browse_descriptor_create_bonjour_service"]
    pub fn nw_shim_browse_descriptor_create_bonjour_service(
        service_type: *const c_char,
        domain: *const c_char,
    ) -> *mut c_void;
    #[link_name = "nfw_browse_descriptor_copy_bonjour_service_type"]
    pub fn nw_shim_browse_descriptor_copy_bonjour_service_type(
        descriptor: *mut c_void,
    ) -> *mut c_char;
    #[link_name = "nfw_browse_descriptor_copy_bonjour_service_domain"]
    pub fn nw_shim_browse_descriptor_copy_bonjour_service_domain(
        descriptor: *mut c_void,
    ) -> *mut c_char;
    #[link_name = "nfw_browse_descriptor_set_include_txt_record"]
    pub fn nw_shim_browse_descriptor_set_include_txt_record(
        descriptor: *mut c_void,
        include_txt_record: c_int,
    );
    #[link_name = "nfw_browse_descriptor_get_include_txt_record"]
    pub fn nw_shim_browse_descriptor_get_include_txt_record(descriptor: *mut c_void) -> c_int;
    #[link_name = "nfw_browse_descriptor_create_application_service"]
    pub fn nw_shim_browse_descriptor_create_application_service(
        application_service_name: *const c_char,
    ) -> *mut c_void;
    #[link_name = "nfw_browse_descriptor_copy_application_service_name"]
    pub fn nw_shim_browse_descriptor_copy_application_service_name(
        descriptor: *mut c_void,
    ) -> *mut c_char;

    #[link_name = "nfw_protocol_copy_tcp_definition"]
    pub fn nw_shim_protocol_copy_tcp_definition() -> *mut c_void;
    #[link_name = "nfw_protocol_copy_udp_definition"]
    pub fn nw_shim_protocol_copy_udp_definition() -> *mut c_void;
    #[link_name = "nfw_protocol_copy_tls_definition"]
    pub fn nw_shim_protocol_copy_tls_definition() -> *mut c_void;
    #[link_name = "nfw_protocol_copy_ip_definition"]
    pub fn nw_shim_protocol_copy_ip_definition() -> *mut c_void;
    #[link_name = "nfw_protocol_copy_ws_definition"]
    pub fn nw_shim_protocol_copy_ws_definition() -> *mut c_void;
    #[link_name = "nfw_protocol_copy_quic_definition"]
    pub fn nw_shim_protocol_copy_quic_definition() -> *mut c_void;
    #[link_name = "nfw_protocol_create_tcp_options"]
    pub fn nw_shim_protocol_create_tcp_options() -> *mut c_void;
    #[link_name = "nfw_protocol_create_udp_options"]
    pub fn nw_shim_protocol_create_udp_options() -> *mut c_void;
    #[link_name = "nfw_protocol_create_tls_options"]
    pub fn nw_shim_protocol_create_tls_options() -> *mut c_void;
    #[link_name = "nfw_protocol_create_ip_options"]
    pub fn nw_shim_protocol_create_ip_options() -> *mut c_void;
    #[link_name = "nfw_protocol_create_ws_options"]
    pub fn nw_shim_protocol_create_ws_options() -> *mut c_void;
    #[link_name = "nfw_protocol_create_quic_options"]
    pub fn nw_shim_protocol_create_quic_options() -> *mut c_void;
    #[link_name = "nfw_protocol_definition_is_equal"]
    pub fn nw_shim_protocol_definition_is_equal(
        definition1: *mut c_void,
        definition2: *mut c_void,
    ) -> c_int;
    #[link_name = "nfw_protocol_options_copy_definition"]
    pub fn nw_shim_protocol_options_copy_definition(options: *mut c_void) -> *mut c_void;
    #[link_name = "nfw_protocol_metadata_copy_definition"]
    pub fn nw_shim_protocol_metadata_copy_definition(metadata: *mut c_void) -> *mut c_void;
    #[link_name = "nfw_protocol_options_is_quic"]
    pub fn nw_shim_protocol_options_is_quic(options: *mut c_void) -> c_int;

    #[link_name = "nfw_quic_add_tls_application_protocol"]
    pub fn nw_shim_quic_add_tls_application_protocol(
        options: *mut c_void,
        application_protocol: *const c_char,
    );
    #[link_name = "nfw_quic_get_stream_is_unidirectional"]
    pub fn nw_shim_quic_get_stream_is_unidirectional(options: *mut c_void) -> c_int;
    #[link_name = "nfw_quic_set_stream_is_unidirectional"]
    pub fn nw_shim_quic_set_stream_is_unidirectional(
        options: *mut c_void,
        is_unidirectional: c_int,
    );
    #[link_name = "nfw_quic_get_stream_is_datagram"]
    pub fn nw_shim_quic_get_stream_is_datagram(options: *mut c_void) -> c_int;
    #[link_name = "nfw_quic_set_stream_is_datagram"]
    pub fn nw_shim_quic_set_stream_is_datagram(options: *mut c_void, is_datagram: c_int);
    #[link_name = "nfw_quic_get_initial_max_data"]
    pub fn nw_shim_quic_get_initial_max_data(options: *mut c_void) -> u64;
    #[link_name = "nfw_quic_set_initial_max_data"]
    pub fn nw_shim_quic_set_initial_max_data(options: *mut c_void, initial_max_data: u64);
    #[link_name = "nfw_quic_get_max_udp_payload_size"]
    pub fn nw_shim_quic_get_max_udp_payload_size(options: *mut c_void) -> u16;
    #[link_name = "nfw_quic_set_max_udp_payload_size"]
    pub fn nw_shim_quic_set_max_udp_payload_size(options: *mut c_void, max_udp_payload_size: u16);
    #[link_name = "nfw_quic_get_idle_timeout"]
    pub fn nw_shim_quic_get_idle_timeout(options: *mut c_void) -> u32;
    #[link_name = "nfw_quic_set_idle_timeout"]
    pub fn nw_shim_quic_set_idle_timeout(options: *mut c_void, idle_timeout: u32);
}
