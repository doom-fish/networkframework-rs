//! Raw FFI declarations matching `src/c-shim/network_shim.c`.

#![allow(missing_docs)]

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

pub type PathMonitorCallback = unsafe extern "C" fn(
    satisfied: c_int,
    interface_type: c_int,
    user_info: *mut c_void,
);

pub type InterfaceEnumerationCallback = unsafe extern "C" fn(
    name: *const c_char,
    interface_type: c_int,
    index: u32,
    user_info: *mut c_void,
) -> c_int;

pub type FramerCreateInstanceCallback = unsafe extern "C" fn(user_info: *mut c_void) -> *mut c_void;
pub type FramerDropInstanceCallback = unsafe extern "C" fn(instance: *mut c_void);
pub type FramerStartCallback = unsafe extern "C" fn(instance: *mut c_void, framer: *mut c_void) -> c_int;
pub type FramerInputCallback = unsafe extern "C" fn(instance: *mut c_void, framer: *mut c_void) -> usize;
pub type FramerOutputCallback = unsafe extern "C" fn(
    instance: *mut c_void,
    framer: *mut c_void,
    message: *mut c_void,
    message_length: usize,
    is_complete: c_int,
);
pub type FramerWakeupCallback = unsafe extern "C" fn(instance: *mut c_void, framer: *mut c_void);
pub type FramerStopCallback = unsafe extern "C" fn(instance: *mut c_void, framer: *mut c_void) -> c_int;
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

extern "C" {
    pub fn nw_shim_tcp_connect(
        host: *const c_char,
        port: u16,
        use_tls: c_int,
        out_status: *mut c_int,
    ) -> *mut c_void;

    pub fn nw_shim_tcp_send(handle: *mut c_void, data: *const u8, len: usize) -> c_int;

    pub fn nw_shim_tcp_receive(handle: *mut c_void, out_buf: *mut u8, max_len: usize) -> isize;

    pub fn nw_shim_tcp_close(handle: *mut c_void);

    pub fn nw_shim_listener_create(
        port: u16,
        use_tls: c_int,
        out_status: *mut c_int,
    ) -> *mut c_void;

    pub fn nw_shim_listener_port(handle: *mut c_void) -> u16;

    pub fn nw_shim_listener_accept(handle: *mut c_void, out_status: *mut c_int) -> *mut c_void;

    pub fn nw_shim_listener_close(handle: *mut c_void);

    pub fn nw_shim_udp_connect(
        host: *const c_char,
        port: u16,
        out_status: *mut c_int,
    ) -> *mut c_void;

    pub fn nw_shim_path_monitor_start(
        callback: PathMonitorCallback,
        user_info: *mut c_void,
    ) -> *mut c_void;

    pub fn nw_shim_path_monitor_stop(handle: *mut c_void);

    pub fn nw_shim_path_monitor_enumerate_interfaces(
        handle: *mut c_void,
        callback: InterfaceEnumerationCallback,
        user_info: *mut c_void,
    ) -> c_int;

    pub fn nw_shim_list_interfaces(
        callback: InterfaceEnumerationCallback,
        user_info: *mut c_void,
    ) -> c_int;

    pub fn nw_shim_browser_start(
        service_type: *const c_char,
        domain: *const c_char,
        found_callback: BrowserServiceCallback,
        lost_callback: BrowserServiceCallback,
        user_info: *mut c_void,
    ) -> *mut c_void;

    pub fn nw_shim_browser_stop(handle: *mut c_void);

    pub fn nw_shim_ws_connect(
        host: *const c_char,
        port: u16,
        path: *const c_char,
        use_tls: c_int,
        out_status: *mut c_int,
    ) -> *mut c_void;

    pub fn nw_shim_ws_send(
        handle: *mut c_void,
        data: *const u8,
        len: usize,
        opcode: c_int,
    ) -> c_int;

    pub fn nw_shim_ws_receive(
        handle: *mut c_void,
        out_buf: *mut u8,
        max_len: usize,
        out_opcode: *mut c_int,
    ) -> isize;

    pub fn nw_shim_quic_connect(
        host: *const c_char,
        port: u16,
        alpn: *const c_char,
        out_status: *mut c_int,
    ) -> *mut c_void;

    pub fn nw_shim_bonjour_advertise_start(
        service_type: *const c_char,
        service_name: *const c_char,
        domain: *const c_char,
        port: u16,
        out_status: *mut c_int,
    ) -> *mut c_void;

    pub fn nw_shim_bonjour_advertise_stop(handle: *mut c_void);

    pub fn nw_shim_retain_object(handle: *mut c_void) -> *mut c_void;
    pub fn nw_shim_release_object(handle: *mut c_void);

    pub fn nw_shim_parameters_create_tcp(use_tls: c_int) -> *mut c_void;
    pub fn nw_shim_parameters_create_udp() -> *mut c_void;
    pub fn nw_shim_parameters_create_quic(alpn: *const c_char) -> *mut c_void;
    pub fn nw_shim_parameters_copy(handle: *mut c_void) -> *mut c_void;
    pub fn nw_shim_parameters_prepend_application_protocol(
        parameters: *mut c_void,
        protocol_options: *mut c_void,
    ) -> c_int;
    pub fn nw_shim_parameters_set_privacy_context(parameters: *mut c_void, privacy_context: *mut c_void);
    pub fn nw_shim_parameters_set_prefer_no_proxy(parameters: *mut c_void, prefer_no_proxy: c_int);

    pub fn nw_shim_connection_create_with_parameters(
        host: *const c_char,
        port: u16,
        parameters: *mut c_void,
        out_status: *mut c_int,
    ) -> *mut c_void;

    pub fn nw_shim_listener_create_with_parameters(
        parameters: *mut c_void,
        port: u16,
        out_status: *mut c_int,
    ) -> *mut c_void;

    pub fn nw_shim_content_context_create(identifier: *const c_char) -> *mut c_void;
    pub fn nw_shim_content_context_get_identifier(context: *mut c_void) -> *const c_char;
    pub fn nw_shim_content_context_get_is_final(context: *mut c_void) -> c_int;
    pub fn nw_shim_content_context_set_is_final(context: *mut c_void, is_final: c_int);
    pub fn nw_shim_content_context_get_expiration_milliseconds(context: *mut c_void) -> u64;
    pub fn nw_shim_content_context_set_expiration_milliseconds(
        context: *mut c_void,
        expiration_milliseconds: u64,
    );
    pub fn nw_shim_content_context_get_relative_priority(context: *mut c_void) -> f64;
    pub fn nw_shim_content_context_set_relative_priority(context: *mut c_void, relative_priority: f64);
    pub fn nw_shim_content_context_set_antecedent(context: *mut c_void, antecedent: *mut c_void);
    pub fn nw_shim_content_context_copy_antecedent(context: *mut c_void) -> *mut c_void;
    pub fn nw_shim_content_context_set_protocol_metadata(context: *mut c_void, metadata: *mut c_void);
    pub fn nw_shim_content_context_copy_protocol_metadata_for_options(
        context: *mut c_void,
        protocol_options: *mut c_void,
    ) -> *mut c_void;

    pub fn nw_shim_connection_send_with_context(
        handle: *mut c_void,
        data: *const u8,
        len: usize,
        context: *mut c_void,
    ) -> c_int;
    pub fn nw_shim_connection_receive_with_context(
        handle: *mut c_void,
        out_buf: *mut u8,
        max_len: usize,
        out_context: *mut *mut c_void,
        out_is_complete: *mut c_int,
    ) -> isize;

    pub fn nw_shim_privacy_context_create(description: *const c_char) -> *mut c_void;
    pub fn nw_shim_privacy_context_flush_cache(privacy_context: *mut c_void);
    pub fn nw_shim_privacy_context_disable_logging(privacy_context: *mut c_void);
    pub fn nw_shim_privacy_context_require_encrypted_name_resolution(
        privacy_context: *mut c_void,
        require_encrypted_name_resolution: c_int,
        fallback_resolver_config: *mut c_void,
    );
    pub fn nw_shim_privacy_context_add_proxy(privacy_context: *mut c_void, proxy_config: *mut c_void);
    pub fn nw_shim_privacy_context_clear_proxies(privacy_context: *mut c_void);

    pub fn nw_shim_resolver_config_create_https(url: *const c_char) -> *mut c_void;
    pub fn nw_shim_resolver_config_create_tls(host: *const c_char, port: u16) -> *mut c_void;
    pub fn nw_shim_resolver_config_add_server_address(
        resolver_config: *mut c_void,
        address: *const c_char,
        port: u16,
    ) -> c_int;

    pub fn nw_shim_proxy_config_create_http_connect(
        host: *const c_char,
        port: u16,
        use_tls: c_int,
    ) -> *mut c_void;
    pub fn nw_shim_proxy_config_create_socksv5(host: *const c_char, port: u16) -> *mut c_void;
    pub fn nw_shim_proxy_config_set_username_password(
        proxy_config: *mut c_void,
        username: *const c_char,
        password: *const c_char,
    );
    pub fn nw_shim_proxy_config_set_failover_allowed(proxy_config: *mut c_void, failover_allowed: c_int);
    pub fn nw_shim_proxy_config_get_failover_allowed(proxy_config: *mut c_void) -> c_int;
    pub fn nw_shim_proxy_config_add_match_domain(proxy_config: *mut c_void, domain: *const c_char);
    pub fn nw_shim_proxy_config_clear_match_domains(proxy_config: *mut c_void);
    pub fn nw_shim_proxy_config_add_excluded_domain(proxy_config: *mut c_void, domain: *const c_char);
    pub fn nw_shim_proxy_config_clear_excluded_domains(proxy_config: *mut c_void);

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
    pub fn nw_shim_framer_create_options(definition: *mut c_void) -> *mut c_void;
    pub fn nw_shim_framer_message_create_from_options(protocol_options: *mut c_void) -> *mut c_void;
    pub fn nw_shim_framer_message_create_for_instance(framer: *mut c_void) -> *mut c_void;
    pub fn nw_shim_framer_message_set_u64(message: *mut c_void, key: *const c_char, value: u64) -> c_int;
    pub fn nw_shim_framer_message_get_u64(
        message: *mut c_void,
        key: *const c_char,
        out_value: *mut u64,
    ) -> c_int;
    pub fn nw_shim_framer_parse_input(
        framer: *mut c_void,
        minimum_incomplete_length: usize,
        maximum_length: usize,
        temp_buffer: *mut u8,
        parse_callback: FramerParseCallback,
        user_info: *mut c_void,
    ) -> c_int;
    pub fn nw_shim_framer_parse_output(
        framer: *mut c_void,
        minimum_incomplete_length: usize,
        maximum_length: usize,
        temp_buffer: *mut u8,
        parse_callback: FramerParseCallback,
        user_info: *mut c_void,
    ) -> c_int;
    pub fn nw_shim_framer_mark_ready(framer: *mut c_void);
    pub fn nw_shim_framer_prepend_application_protocol(
        framer: *mut c_void,
        protocol_options: *mut c_void,
    ) -> c_int;
    pub fn nw_shim_framer_mark_failed_with_error(framer: *mut c_void, error_code: c_int);
    pub fn nw_shim_framer_pass_input_data(
        framer: *mut c_void,
        input_length: usize,
        message: *mut c_void,
        is_complete: c_int,
    ) -> c_int;
    pub fn nw_shim_framer_deliver_input_data(
        framer: *mut c_void,
        input_buffer: *const u8,
        input_length: usize,
        message: *mut c_void,
        is_complete: c_int,
    );
    pub fn nw_shim_framer_pass_through_input(framer: *mut c_void);
    pub fn nw_shim_framer_pass_output_data(framer: *mut c_void, output_length: usize) -> c_int;
    pub fn nw_shim_framer_write_output_data(
        framer: *mut c_void,
        output_buffer: *const u8,
        output_length: usize,
    );
    pub fn nw_shim_framer_pass_through_output(framer: *mut c_void);
    pub fn nw_shim_framer_schedule_wakeup(framer: *mut c_void, milliseconds: u64);
    pub fn nw_shim_framer_async(
        framer: *mut c_void,
        async_callback: FramerAsyncCallback,
        user_info: *mut c_void,
    );

    pub fn nw_shim_group_descriptor_create_multiplex(host: *const c_char, port: u16) -> *mut c_void;
    pub fn nw_shim_group_descriptor_create_multicast(group_address: *const c_char, port: u16) -> *mut c_void;
    pub fn nw_shim_group_descriptor_add_endpoint(
        descriptor: *mut c_void,
        host: *const c_char,
        port: u16,
    ) -> c_int;

    pub fn nw_shim_connection_group_create(
        descriptor: *mut c_void,
        parameters: *mut c_void,
    ) -> *mut c_void;
    pub fn nw_shim_connection_group_set_state_changed_handler(
        handle: *mut c_void,
        state_callback: Option<ConnectionGroupStateCallback>,
        user_info: *mut c_void,
    );
    pub fn nw_shim_connection_group_set_receive_handler(
        handle: *mut c_void,
        maximum_message_size: u32,
        reject_oversized_messages: c_int,
        receive_callback: Option<ConnectionGroupReceiveCallback>,
        user_info: *mut c_void,
    );
    pub fn nw_shim_connection_group_start(handle: *mut c_void) -> c_int;
    pub fn nw_shim_connection_group_cancel(handle: *mut c_void);
    pub fn nw_shim_connection_group_send(
        handle: *mut c_void,
        data: *const u8,
        len: usize,
        host: *const c_char,
        port: u16,
        context: *mut c_void,
    ) -> c_int;
    pub fn nw_shim_connection_group_release(handle: *mut c_void);
}
