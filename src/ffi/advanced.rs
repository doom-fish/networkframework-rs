use super::{c_char, c_int, c_void};

pub type TxtRecordEntryCallback = unsafe extern "C" fn(
    key: *const c_char,
    found: c_int,
    value: *const u8,
    value_len: usize,
    user_info: *mut c_void,
) -> c_int;

pub type EthernetChannelStateCallback =
    unsafe extern "C" fn(state: c_int, user_info: *mut c_void);
pub type EthernetChannelReceiveCallback = unsafe extern "C" fn(
    data: *const u8,
    len: usize,
    vlan_tag: u16,
    local_address: *const u8,
    remote_address: *const u8,
    user_info: *mut c_void,
);

#[repr(C)]
pub struct NwShimEstablishmentProtocolInfo {
    pub protocol_definition: *mut c_void,
    pub handshake_milliseconds: u64,
    pub handshake_rtt_milliseconds: u64,
}

#[repr(C)]
pub struct NwShimResolutionStepInfo {
    pub source: c_int,
    pub milliseconds: u64,
    pub endpoint_count: u32,
    pub successful_endpoint: *mut c_void,
    pub preferred_endpoint: *mut c_void,
}

unsafe extern "C" {
    #[link_name = "nfw_sec_retain"]
    pub fn nw_shim_sec_retain(object: *mut c_void) -> *mut c_void;
    #[link_name = "nfw_sec_release"]
    pub fn nw_shim_sec_release(object: *mut c_void);

    #[link_name = "nfw_interface_copy_name"]
    pub fn nw_shim_interface_copy_name(interface: *mut c_void) -> *mut c_char;
    #[link_name = "nfw_interface_get_type"]
    pub fn nw_shim_interface_get_type(interface: *mut c_void) -> c_int;
    #[link_name = "nfw_interface_get_index"]
    pub fn nw_shim_interface_get_index(interface: *mut c_void) -> u32;

    #[link_name = "nfw_parameters_require_interface"]
    pub fn nw_shim_parameters_require_interface(
        parameters: *mut c_void,
        name: *const c_char,
        interface_type: c_int,
        index: u32,
    );
    #[link_name = "nfw_parameters_copy_required_interface"]
    pub fn nw_shim_parameters_copy_required_interface(
        parameters: *mut c_void,
        out_name: *mut *mut c_char,
        out_type: *mut c_int,
        out_index: *mut u32,
    ) -> c_int;
    #[link_name = "nfw_parameters_prohibit_interface"]
    pub fn nw_shim_parameters_prohibit_interface(
        parameters: *mut c_void,
        name: *const c_char,
        interface_type: c_int,
        index: u32,
    );
    #[link_name = "nfw_parameters_clear_prohibited_interfaces"]
    pub fn nw_shim_parameters_clear_prohibited_interfaces(parameters: *mut c_void);
    #[link_name = "nfw_parameters_copy_prohibited_interfaces"]
    pub fn nw_shim_parameters_copy_prohibited_interfaces(
        parameters: *mut c_void,
        out_count: *mut usize,
    ) -> *mut *mut c_void;
    #[link_name = "nfw_parameters_prohibit_interface_type"]
    pub fn nw_shim_parameters_prohibit_interface_type(parameters: *mut c_void, interface_type: c_int);
    #[link_name = "nfw_parameters_clear_prohibited_interface_types"]
    pub fn nw_shim_parameters_clear_prohibited_interface_types(parameters: *mut c_void);
    #[link_name = "nfw_parameters_copy_prohibited_interface_types"]
    pub fn nw_shim_parameters_copy_prohibited_interface_types(
        parameters: *mut c_void,
        out_count: *mut usize,
    ) -> *mut c_int;
    #[link_name = "nfw_parameters_set_reuse_local_address"]
    pub fn nw_shim_parameters_set_reuse_local_address(parameters: *mut c_void, reuse_local_address: c_int);
    #[link_name = "nfw_parameters_get_reuse_local_address"]
    pub fn nw_shim_parameters_get_reuse_local_address(parameters: *mut c_void) -> c_int;
    #[link_name = "nfw_parameters_set_local_endpoint"]
    pub fn nw_shim_parameters_set_local_endpoint(parameters: *mut c_void, local_endpoint: *mut c_void);
    #[link_name = "nfw_parameters_copy_local_endpoint"]
    pub fn nw_shim_parameters_copy_local_endpoint(parameters: *mut c_void) -> *mut c_void;
    #[link_name = "nfw_parameters_set_include_peer_to_peer"]
    pub fn nw_shim_parameters_set_include_peer_to_peer(
        parameters: *mut c_void,
        include_peer_to_peer: c_int,
    );
    #[link_name = "nfw_parameters_get_include_peer_to_peer"]
    pub fn nw_shim_parameters_get_include_peer_to_peer(parameters: *mut c_void) -> c_int;
    #[link_name = "nfw_parameters_set_fast_open_enabled"]
    pub fn nw_shim_parameters_set_fast_open_enabled(parameters: *mut c_void, fast_open_enabled: c_int);
    #[link_name = "nfw_parameters_get_fast_open_enabled"]
    pub fn nw_shim_parameters_get_fast_open_enabled(parameters: *mut c_void) -> c_int;
    #[link_name = "nfw_parameters_set_service_class"]
    pub fn nw_shim_parameters_set_service_class(parameters: *mut c_void, service_class: c_int);
    #[link_name = "nfw_parameters_get_service_class"]
    pub fn nw_shim_parameters_get_service_class(parameters: *mut c_void) -> c_int;
    #[link_name = "nfw_parameters_set_multipath_service"]
    pub fn nw_shim_parameters_set_multipath_service(parameters: *mut c_void, multipath_service: c_int);
    #[link_name = "nfw_parameters_get_multipath_service"]
    pub fn nw_shim_parameters_get_multipath_service(parameters: *mut c_void) -> c_int;
    #[link_name = "nfw_parameters_copy_default_protocol_stack"]
    pub fn nw_shim_parameters_copy_default_protocol_stack(parameters: *mut c_void) -> *mut c_void;
    #[link_name = "nfw_protocol_stack_clear_application_protocols"]
    pub fn nw_shim_protocol_stack_clear_application_protocols(stack: *mut c_void);
    #[link_name = "nfw_protocol_stack_copy_application_protocols"]
    pub fn nw_shim_protocol_stack_copy_application_protocols(
        stack: *mut c_void,
        out_count: *mut usize,
    ) -> *mut *mut c_void;
    #[link_name = "nfw_protocol_stack_copy_transport_protocol"]
    pub fn nw_shim_protocol_stack_copy_transport_protocol(stack: *mut c_void) -> *mut c_void;
    #[link_name = "nfw_protocol_stack_set_transport_protocol"]
    pub fn nw_shim_protocol_stack_set_transport_protocol(stack: *mut c_void, protocol: *mut c_void);
    #[link_name = "nfw_protocol_stack_copy_internet_protocol"]
    pub fn nw_shim_protocol_stack_copy_internet_protocol(stack: *mut c_void) -> *mut c_void;
    #[link_name = "nfw_parameters_set_local_only"]
    pub fn nw_shim_parameters_set_local_only(parameters: *mut c_void, local_only: c_int);
    #[link_name = "nfw_parameters_get_local_only"]
    pub fn nw_shim_parameters_get_local_only(parameters: *mut c_void) -> c_int;
    #[link_name = "nfw_parameters_get_prefer_no_proxy"]
    pub fn nw_shim_parameters_get_prefer_no_proxy(parameters: *mut c_void) -> c_int;
    #[link_name = "nfw_parameters_set_expired_dns_behavior"]
    pub fn nw_shim_parameters_set_expired_dns_behavior(
        parameters: *mut c_void,
        expired_dns_behavior: c_int,
    );
    #[link_name = "nfw_parameters_get_expired_dns_behavior"]
    pub fn nw_shim_parameters_get_expired_dns_behavior(parameters: *mut c_void) -> c_int;
    #[link_name = "nfw_parameters_set_requires_dnssec_validation"]
    pub fn nw_shim_parameters_set_requires_dnssec_validation(
        parameters: *mut c_void,
        requires_dnssec_validation: c_int,
    );
    #[link_name = "nfw_parameters_requires_dnssec_validation"]
    pub fn nw_shim_parameters_requires_dnssec_validation(parameters: *mut c_void) -> c_int;

    #[link_name = "nfw_connection_copy_establishment_report"]
    pub fn nw_shim_connection_copy_establishment_report(handle: *mut c_void) -> *mut c_void;
    #[link_name = "nfw_establishment_report_get_duration_milliseconds"]
    pub fn nw_shim_establishment_report_get_duration_milliseconds(report: *mut c_void) -> u64;
    #[link_name = "nfw_establishment_report_get_attempt_started_after_milliseconds"]
    pub fn nw_shim_establishment_report_get_attempt_started_after_milliseconds(report: *mut c_void) -> u64;
    #[link_name = "nfw_establishment_report_get_previous_attempt_count"]
    pub fn nw_shim_establishment_report_get_previous_attempt_count(report: *mut c_void) -> u32;
    #[link_name = "nfw_establishment_report_get_used_proxy"]
    pub fn nw_shim_establishment_report_get_used_proxy(report: *mut c_void) -> c_int;
    #[link_name = "nfw_establishment_report_get_proxy_configured"]
    pub fn nw_shim_establishment_report_get_proxy_configured(report: *mut c_void) -> c_int;
    #[link_name = "nfw_establishment_report_copy_proxy_endpoint"]
    pub fn nw_shim_establishment_report_copy_proxy_endpoint(report: *mut c_void) -> *mut c_void;
    #[link_name = "nfw_establishment_report_copy_protocols"]
    pub fn nw_shim_establishment_report_copy_protocols(
        report: *mut c_void,
        out_count: *mut usize,
    ) -> *mut NwShimEstablishmentProtocolInfo;
    #[link_name = "nfw_establishment_report_copy_resolutions"]
    pub fn nw_shim_establishment_report_copy_resolutions(
        report: *mut c_void,
        out_count: *mut usize,
    ) -> *mut NwShimResolutionStepInfo;
    #[link_name = "nfw_establishment_report_copy_resolution_reports"]
    pub fn nw_shim_establishment_report_copy_resolution_reports(
        report: *mut c_void,
        out_count: *mut usize,
    ) -> *mut *mut c_void;
    #[link_name = "nfw_resolution_report_get_source"]
    pub fn nw_shim_resolution_report_get_source(report: *mut c_void) -> c_int;
    #[link_name = "nfw_resolution_report_get_milliseconds"]
    pub fn nw_shim_resolution_report_get_milliseconds(report: *mut c_void) -> u64;
    #[link_name = "nfw_resolution_report_get_endpoint_count"]
    pub fn nw_shim_resolution_report_get_endpoint_count(report: *mut c_void) -> u32;
    #[link_name = "nfw_resolution_report_copy_successful_endpoint"]
    pub fn nw_shim_resolution_report_copy_successful_endpoint(report: *mut c_void) -> *mut c_void;
    #[link_name = "nfw_resolution_report_copy_preferred_endpoint"]
    pub fn nw_shim_resolution_report_copy_preferred_endpoint(report: *mut c_void) -> *mut c_void;
    #[link_name = "nfw_resolution_report_get_protocol"]
    pub fn nw_shim_resolution_report_get_protocol(report: *mut c_void) -> c_int;

    #[link_name = "nfw_connection_create_data_transfer_report"]
    pub fn nw_shim_connection_create_data_transfer_report(handle: *mut c_void) -> *mut c_void;
    #[link_name = "nfw_data_transfer_report_collect"]
    pub fn nw_shim_data_transfer_report_collect(report: *mut c_void) -> c_int;
    #[link_name = "nfw_data_transfer_report_get_state"]
    pub fn nw_shim_data_transfer_report_get_state(report: *mut c_void) -> c_int;
    #[link_name = "nfw_data_transfer_report_all_paths"]
    pub fn nw_shim_data_transfer_report_all_paths() -> u32;
    #[link_name = "nfw_data_transfer_report_get_duration_milliseconds"]
    pub fn nw_shim_data_transfer_report_get_duration_milliseconds(report: *mut c_void) -> u64;
    #[link_name = "nfw_data_transfer_report_get_path_count"]
    pub fn nw_shim_data_transfer_report_get_path_count(report: *mut c_void) -> u32;
    #[link_name = "nfw_data_transfer_report_get_received_ip_packet_count"]
    pub fn nw_shim_data_transfer_report_get_received_ip_packet_count(
        report: *mut c_void,
        path_index: u32,
    ) -> u64;
    #[link_name = "nfw_data_transfer_report_get_sent_ip_packet_count"]
    pub fn nw_shim_data_transfer_report_get_sent_ip_packet_count(report: *mut c_void, path_index: u32)
        -> u64;
    #[link_name = "nfw_data_transfer_report_get_received_transport_byte_count"]
    pub fn nw_shim_data_transfer_report_get_received_transport_byte_count(
        report: *mut c_void,
        path_index: u32,
    ) -> u64;
    #[link_name = "nfw_data_transfer_report_get_received_transport_duplicate_byte_count"]
    pub fn nw_shim_data_transfer_report_get_received_transport_duplicate_byte_count(
        report: *mut c_void,
        path_index: u32,
    ) -> u64;
    #[link_name = "nfw_data_transfer_report_get_received_transport_out_of_order_byte_count"]
    pub fn nw_shim_data_transfer_report_get_received_transport_out_of_order_byte_count(
        report: *mut c_void,
        path_index: u32,
    ) -> u64;
    #[link_name = "nfw_data_transfer_report_get_sent_transport_byte_count"]
    pub fn nw_shim_data_transfer_report_get_sent_transport_byte_count(report: *mut c_void, path_index: u32)
        -> u64;
    #[link_name = "nfw_data_transfer_report_get_sent_transport_retransmitted_byte_count"]
    pub fn nw_shim_data_transfer_report_get_sent_transport_retransmitted_byte_count(
        report: *mut c_void,
        path_index: u32,
    ) -> u64;
    #[link_name = "nfw_data_transfer_report_get_transport_smoothed_rtt_milliseconds"]
    pub fn nw_shim_data_transfer_report_get_transport_smoothed_rtt_milliseconds(
        report: *mut c_void,
        path_index: u32,
    ) -> u64;
    #[link_name = "nfw_data_transfer_report_get_transport_minimum_rtt_milliseconds"]
    pub fn nw_shim_data_transfer_report_get_transport_minimum_rtt_milliseconds(
        report: *mut c_void,
        path_index: u32,
    ) -> u64;
    #[link_name = "nfw_data_transfer_report_get_transport_rtt_variance"]
    pub fn nw_shim_data_transfer_report_get_transport_rtt_variance(report: *mut c_void, path_index: u32)
        -> u64;
    #[link_name = "nfw_data_transfer_report_get_received_application_byte_count"]
    pub fn nw_shim_data_transfer_report_get_received_application_byte_count(
        report: *mut c_void,
        path_index: u32,
    ) -> u64;
    #[link_name = "nfw_data_transfer_report_get_sent_application_byte_count"]
    pub fn nw_shim_data_transfer_report_get_sent_application_byte_count(report: *mut c_void, path_index: u32)
        -> u64;
    #[link_name = "nfw_data_transfer_report_copy_path_interface"]
    pub fn nw_shim_data_transfer_report_copy_path_interface(report: *mut c_void, path_index: u32)
        -> *mut c_void;
    #[link_name = "nfw_data_transfer_report_get_path_radio_type"]
    pub fn nw_shim_data_transfer_report_get_path_radio_type(report: *mut c_void, path_index: u32)
        -> c_int;

    #[link_name = "nfw_endpoint_copy_address"]
    pub fn nw_shim_endpoint_copy_address(
        endpoint: *mut c_void,
        out_buffer: *mut c_void,
        out_buffer_length: usize,
    ) -> usize;
    #[link_name = "nfw_endpoint_copy_txt_record"]
    pub fn nw_shim_endpoint_copy_txt_record(endpoint: *mut c_void) -> *mut c_void;

    #[link_name = "nfw_txt_record_create_with_bytes"]
    pub fn nw_shim_txt_record_create_with_bytes(txt_bytes: *const u8, txt_length: usize) -> *mut c_void;
    #[link_name = "nfw_txt_record_create_dictionary"]
    pub fn nw_shim_txt_record_create_dictionary() -> *mut c_void;
    #[link_name = "nfw_txt_record_copy"]
    pub fn nw_shim_txt_record_copy(txt_record: *mut c_void) -> *mut c_void;
    #[link_name = "nfw_txt_record_find_key"]
    pub fn nw_shim_txt_record_find_key(txt_record: *mut c_void, key: *const c_char) -> c_int;
    #[link_name = "nfw_txt_record_copy_value"]
    pub fn nw_shim_txt_record_copy_value(
        txt_record: *mut c_void,
        key: *const c_char,
        out_value_length: *mut usize,
        out_found: *mut c_int,
    ) -> *mut u8;
    #[link_name = "nfw_txt_record_set_key"]
    pub fn nw_shim_txt_record_set_key(
        txt_record: *mut c_void,
        key: *const c_char,
        value: *const u8,
        value_length: usize,
    ) -> c_int;
    #[link_name = "nfw_txt_record_remove_key"]
    pub fn nw_shim_txt_record_remove_key(txt_record: *mut c_void, key: *const c_char) -> c_int;
    #[link_name = "nfw_txt_record_get_key_count"]
    pub fn nw_shim_txt_record_get_key_count(txt_record: *mut c_void) -> usize;
    #[link_name = "nfw_txt_record_copy_bytes"]
    pub fn nw_shim_txt_record_copy_bytes(txt_record: *mut c_void, out_length: *mut usize) -> *mut u8;
    #[link_name = "nfw_txt_record_apply"]
    pub fn nw_shim_txt_record_apply(
        txt_record: *mut c_void,
        callback: Option<TxtRecordEntryCallback>,
        user_info: *mut c_void,
    ) -> c_int;
    #[link_name = "nfw_txt_record_is_dictionary"]
    pub fn nw_shim_txt_record_is_dictionary(txt_record: *mut c_void) -> c_int;
    #[link_name = "nfw_txt_record_is_equal"]
    pub fn nw_shim_txt_record_is_equal(txt_record: *mut c_void, other_txt_record: *mut c_void) -> c_int;

    #[link_name = "nfw_connection_copy_quic_metadata"]
    pub fn nw_shim_connection_copy_quic_metadata(handle: *mut c_void) -> *mut c_void;
    #[link_name = "nfw_content_context_copy_quic_metadata"]
    pub fn nw_shim_content_context_copy_quic_metadata(context: *mut c_void) -> *mut c_void;
    #[link_name = "nfw_protocol_metadata_is_quic"]
    pub fn nw_shim_protocol_metadata_is_quic(metadata: *mut c_void) -> c_int;
    #[link_name = "nfw_quic_copy_sec_protocol_options"]
    pub fn nw_shim_quic_copy_sec_protocol_options(options: *mut c_void) -> *mut c_void;
    #[link_name = "nfw_quic_copy_sec_protocol_metadata"]
    pub fn nw_shim_quic_copy_sec_protocol_metadata(metadata: *mut c_void) -> *mut c_void;
    #[link_name = "nfw_quic_get_initial_max_streams_bidirectional"]
    pub fn nw_shim_quic_get_initial_max_streams_bidirectional(options: *mut c_void) -> u64;
    #[link_name = "nfw_quic_set_initial_max_streams_bidirectional"]
    pub fn nw_shim_quic_set_initial_max_streams_bidirectional(
        options: *mut c_void,
        initial_max_streams_bidirectional: u64,
    );
    #[link_name = "nfw_quic_get_initial_max_streams_unidirectional"]
    pub fn nw_shim_quic_get_initial_max_streams_unidirectional(options: *mut c_void) -> u64;
    #[link_name = "nfw_quic_set_initial_max_streams_unidirectional"]
    pub fn nw_shim_quic_set_initial_max_streams_unidirectional(
        options: *mut c_void,
        initial_max_streams_unidirectional: u64,
    );
    #[link_name = "nfw_quic_get_initial_max_stream_data_bidirectional_local"]
    pub fn nw_shim_quic_get_initial_max_stream_data_bidirectional_local(options: *mut c_void) -> u64;
    #[link_name = "nfw_quic_set_initial_max_stream_data_bidirectional_local"]
    pub fn nw_shim_quic_set_initial_max_stream_data_bidirectional_local(
        options: *mut c_void,
        initial_max_stream_data_bidirectional_local: u64,
    );
    #[link_name = "nfw_quic_get_initial_max_stream_data_bidirectional_remote"]
    pub fn nw_shim_quic_get_initial_max_stream_data_bidirectional_remote(options: *mut c_void) -> u64;
    #[link_name = "nfw_quic_set_initial_max_stream_data_bidirectional_remote"]
    pub fn nw_shim_quic_set_initial_max_stream_data_bidirectional_remote(
        options: *mut c_void,
        initial_max_stream_data_bidirectional_remote: u64,
    );
    #[link_name = "nfw_quic_get_initial_max_stream_data_unidirectional"]
    pub fn nw_shim_quic_get_initial_max_stream_data_unidirectional(options: *mut c_void) -> u64;
    #[link_name = "nfw_quic_set_initial_max_stream_data_unidirectional"]
    pub fn nw_shim_quic_set_initial_max_stream_data_unidirectional(
        options: *mut c_void,
        initial_max_stream_data_unidirectional: u64,
    );
    #[link_name = "nfw_quic_get_max_datagram_frame_size"]
    pub fn nw_shim_quic_get_max_datagram_frame_size(options: *mut c_void) -> u16;
    #[link_name = "nfw_quic_set_max_datagram_frame_size"]
    pub fn nw_shim_quic_set_max_datagram_frame_size(options: *mut c_void, max_datagram_frame_size: u16);
    #[link_name = "nfw_quic_get_stream_id"]
    pub fn nw_shim_quic_get_stream_id(metadata: *mut c_void) -> u64;
    #[link_name = "nfw_quic_get_stream_type"]
    pub fn nw_shim_quic_get_stream_type(metadata: *mut c_void) -> c_int;
    #[link_name = "nfw_quic_get_stream_application_error"]
    pub fn nw_shim_quic_get_stream_application_error(metadata: *mut c_void) -> u64;
    #[link_name = "nfw_quic_set_stream_application_error"]
    pub fn nw_shim_quic_set_stream_application_error(metadata: *mut c_void, application_error: u64);
    #[link_name = "nfw_quic_get_local_max_streams_bidirectional"]
    pub fn nw_shim_quic_get_local_max_streams_bidirectional(metadata: *mut c_void) -> u64;
    #[link_name = "nfw_quic_set_local_max_streams_bidirectional"]
    pub fn nw_shim_quic_set_local_max_streams_bidirectional(metadata: *mut c_void, max_streams_bidirectional: u64);
    #[link_name = "nfw_quic_get_local_max_streams_unidirectional"]
    pub fn nw_shim_quic_get_local_max_streams_unidirectional(metadata: *mut c_void) -> u64;
    #[link_name = "nfw_quic_set_local_max_streams_unidirectional"]
    pub fn nw_shim_quic_set_local_max_streams_unidirectional(metadata: *mut c_void, max_streams_unidirectional: u64);
    #[link_name = "nfw_quic_get_remote_max_streams_bidirectional"]
    pub fn nw_shim_quic_get_remote_max_streams_bidirectional(metadata: *mut c_void) -> u64;
    #[link_name = "nfw_quic_get_remote_max_streams_unidirectional"]
    pub fn nw_shim_quic_get_remote_max_streams_unidirectional(metadata: *mut c_void) -> u64;
    #[link_name = "nfw_quic_get_stream_usable_datagram_frame_size"]
    pub fn nw_shim_quic_get_stream_usable_datagram_frame_size(metadata: *mut c_void) -> u16;
    #[link_name = "nfw_quic_get_application_error"]
    pub fn nw_shim_quic_get_application_error(metadata: *mut c_void) -> u64;
    #[link_name = "nfw_quic_copy_application_error_reason"]
    pub fn nw_shim_quic_copy_application_error_reason(metadata: *mut c_void) -> *mut c_char;
    #[link_name = "nfw_quic_set_application_error"]
    pub fn nw_shim_quic_set_application_error(
        metadata: *mut c_void,
        application_error: u64,
        reason: *const c_char,
    );
    #[link_name = "nfw_quic_get_keepalive_interval"]
    pub fn nw_shim_quic_get_keepalive_interval(metadata: *mut c_void) -> u16;
    #[link_name = "nfw_quic_set_keepalive_interval"]
    pub fn nw_shim_quic_set_keepalive_interval(metadata: *mut c_void, keepalive_interval: u16);
    #[link_name = "nfw_quic_get_remote_idle_timeout"]
    pub fn nw_shim_quic_get_remote_idle_timeout(metadata: *mut c_void) -> u64;

    #[link_name = "nfw_ethernet_channel_create"]
    pub fn nw_shim_ethernet_channel_create(
        ether_type: u16,
        name: *const c_char,
        interface_type: c_int,
        index: u32,
    ) -> *mut c_void;
    #[link_name = "nfw_ethernet_channel_create_with_parameters"]
    pub fn nw_shim_ethernet_channel_create_with_parameters(
        ether_type: u16,
        name: *const c_char,
        interface_type: c_int,
        index: u32,
        parameters: *mut c_void,
    ) -> *mut c_void;
    #[link_name = "nfw_ethernet_channel_set_state_changed_handler"]
    pub fn nw_shim_ethernet_channel_set_state_changed_handler(
        handle: *mut c_void,
        callback: Option<EthernetChannelStateCallback>,
        user_info: *mut c_void,
    );
    #[link_name = "nfw_ethernet_channel_set_receive_handler"]
    pub fn nw_shim_ethernet_channel_set_receive_handler(
        handle: *mut c_void,
        callback: Option<EthernetChannelReceiveCallback>,
        user_info: *mut c_void,
    );
    #[link_name = "nfw_ethernet_channel_get_maximum_payload_size"]
    pub fn nw_shim_ethernet_channel_get_maximum_payload_size(handle: *mut c_void) -> u32;
    #[link_name = "nfw_ethernet_channel_start"]
    pub fn nw_shim_ethernet_channel_start(handle: *mut c_void);
    #[link_name = "nfw_ethernet_channel_cancel"]
    pub fn nw_shim_ethernet_channel_cancel(handle: *mut c_void);
    #[link_name = "nfw_ethernet_channel_send"]
    pub fn nw_shim_ethernet_channel_send(
        handle: *mut c_void,
        data: *const u8,
        len: usize,
        vlan_tag: u16,
        remote_address: *const u8,
    ) -> c_int;
    #[link_name = "nfw_ethernet_channel_release"]
    pub fn nw_shim_ethernet_channel_release(handle: *mut c_void);
}
