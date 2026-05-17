use super::{
    c_char, c_int, c_void, InterfaceEnumerationCallback, PathMonitorCallback,
    StringEnumerationCallback,
};

pub type TxtRecordEntryCallback = unsafe extern "C" fn(
    key: *const c_char,
    found: c_int,
    value: *const u8,
    value_len: usize,
    user_info: *mut c_void,
) -> c_int;

pub type EthernetChannelStateCallback = unsafe extern "C" fn(state: c_int, user_info: *mut c_void);
pub type EthernetChannelReceiveCallback = unsafe extern "C" fn(
    data: *const u8,
    len: usize,
    vlan_tag: u16,
    local_address: *const u8,
    remote_address: *const u8,
    user_info: *mut c_void,
);

pub type BrowserStateChangedCallback =
    unsafe extern "C" fn(state: c_int, error: *mut c_void, user_info: *mut c_void);
pub type BrowseResultChangedCallback = unsafe extern "C" fn(
    old_result: *mut c_void,
    new_result: *mut c_void,
    changes: u64,
    batch_complete: c_int,
    user_info: *mut c_void,
);
pub type ConnectionStateCallback =
    unsafe extern "C" fn(state: c_int, error: *mut c_void, user_info: *mut c_void);
pub type ListenerStateCallback =
    unsafe extern "C" fn(state: c_int, error: *mut c_void, user_info: *mut c_void);
pub type ListenerNewConnectionCallback =
    unsafe extern "C" fn(connection_handle: *mut c_void, user_info: *mut c_void);
pub type ConnectionBooleanCallback = unsafe extern "C" fn(value: c_int, user_info: *mut c_void);
pub type ConnectionPathCallback = unsafe extern "C" fn(path: *mut c_void, user_info: *mut c_void);
pub type ConnectionBatchCallback = unsafe extern "C" fn(user_info: *mut c_void);
pub type ProtocolMetadataEnumerationCallback = unsafe extern "C" fn(
    definition: *mut c_void,
    metadata: *mut c_void,
    user_info: *mut c_void,
) -> c_int;
pub type EndpointEnumerationCallback =
    unsafe extern "C" fn(endpoint: *mut c_void, user_info: *mut c_void) -> c_int;
pub type HeaderEnumerationCallback = unsafe extern "C" fn(
    name: *const c_char,
    value: *const c_char,
    user_info: *mut c_void,
) -> c_int;
pub type PathMonitorCancelCallback = unsafe extern "C" fn(user_info: *mut c_void);
pub type ListenerAdvertisedEndpointChangedCallback =
    unsafe extern "C" fn(endpoint: *mut c_void, added: c_int, user_info: *mut c_void);
pub type ListenerNewConnectionGroupCallback =
    unsafe extern "C" fn(group: *mut c_void, user_info: *mut c_void);
pub type ConnectionGroupNewConnectionCallback =
    unsafe extern "C" fn(connection: *mut c_void, user_info: *mut c_void);
pub type WsPongCallback = unsafe extern "C" fn(error: *mut c_void, user_info: *mut c_void);
pub type WsClientRequestCallback =
    unsafe extern "C" fn(request: *mut c_void, user_info: *mut c_void) -> *mut c_void;

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

    #[link_name = "nw_shim_interface_copy_name"]
    pub fn nw_shim_interface_copy_name(interface: *mut c_void) -> *mut c_char;
    #[link_name = "nw_shim_interface_get_type"]
    pub fn nw_shim_interface_get_type(interface: *mut c_void) -> c_int;
    #[link_name = "nw_shim_interface_get_index"]
    pub fn nw_shim_interface_get_index(interface: *mut c_void) -> u32;

    #[link_name = "nw_shim_parameters_require_interface"]
    pub fn nw_shim_parameters_require_interface(
        parameters: *mut c_void,
        name: *const c_char,
        interface_type: c_int,
        index: u32,
    );
    #[link_name = "nw_shim_parameters_copy_required_interface"]
    pub fn nw_shim_parameters_copy_required_interface(
        parameters: *mut c_void,
        out_name: *mut *mut c_char,
        out_type: *mut c_int,
        out_index: *mut u32,
    ) -> c_int;
    #[link_name = "nw_shim_parameters_prohibit_interface"]
    pub fn nw_shim_parameters_prohibit_interface(
        parameters: *mut c_void,
        name: *const c_char,
        interface_type: c_int,
        index: u32,
    );
    #[link_name = "nw_shim_parameters_clear_prohibited_interfaces"]
    pub fn nw_shim_parameters_clear_prohibited_interfaces(parameters: *mut c_void);
    #[link_name = "nw_shim_parameters_copy_prohibited_interfaces"]
    pub fn nw_shim_parameters_copy_prohibited_interfaces(
        parameters: *mut c_void,
        out_count: *mut usize,
    ) -> *mut *mut c_void;
    #[link_name = "nw_shim_parameters_prohibit_interface_type"]
    pub fn nw_shim_parameters_prohibit_interface_type(
        parameters: *mut c_void,
        interface_type: c_int,
    );
    #[link_name = "nw_shim_parameters_clear_prohibited_interface_types"]
    pub fn nw_shim_parameters_clear_prohibited_interface_types(parameters: *mut c_void);
    #[link_name = "nw_shim_parameters_copy_prohibited_interface_types"]
    pub fn nw_shim_parameters_copy_prohibited_interface_types(
        parameters: *mut c_void,
        out_count: *mut usize,
    ) -> *mut c_int;
    #[link_name = "nw_shim_parameters_set_reuse_local_address"]
    pub fn nw_shim_parameters_set_reuse_local_address(
        parameters: *mut c_void,
        reuse_local_address: c_int,
    );
    #[link_name = "nw_shim_parameters_get_reuse_local_address"]
    pub fn nw_shim_parameters_get_reuse_local_address(parameters: *mut c_void) -> c_int;
    #[link_name = "nw_shim_parameters_set_local_endpoint"]
    pub fn nw_shim_parameters_set_local_endpoint(
        parameters: *mut c_void,
        local_endpoint: *mut c_void,
    );
    #[link_name = "nw_shim_parameters_copy_local_endpoint"]
    pub fn nw_shim_parameters_copy_local_endpoint(parameters: *mut c_void) -> *mut c_void;
    #[link_name = "nw_shim_parameters_set_include_peer_to_peer"]
    pub fn nw_shim_parameters_set_include_peer_to_peer(
        parameters: *mut c_void,
        include_peer_to_peer: c_int,
    );
    #[link_name = "nw_shim_parameters_get_include_peer_to_peer"]
    pub fn nw_shim_parameters_get_include_peer_to_peer(parameters: *mut c_void) -> c_int;
    #[link_name = "nw_shim_parameters_set_fast_open_enabled"]
    pub fn nw_shim_parameters_set_fast_open_enabled(
        parameters: *mut c_void,
        fast_open_enabled: c_int,
    );
    #[link_name = "nw_shim_parameters_get_fast_open_enabled"]
    pub fn nw_shim_parameters_get_fast_open_enabled(parameters: *mut c_void) -> c_int;
    #[link_name = "nw_shim_parameters_set_service_class"]
    pub fn nw_shim_parameters_set_service_class(parameters: *mut c_void, service_class: c_int);
    #[link_name = "nw_shim_parameters_get_service_class"]
    pub fn nw_shim_parameters_get_service_class(parameters: *mut c_void) -> c_int;
    #[link_name = "nw_shim_parameters_set_multipath_service"]
    pub fn nw_shim_parameters_set_multipath_service(
        parameters: *mut c_void,
        multipath_service: c_int,
    );
    #[link_name = "nw_shim_parameters_get_multipath_service"]
    pub fn nw_shim_parameters_get_multipath_service(parameters: *mut c_void) -> c_int;
    #[link_name = "nw_shim_parameters_copy_default_protocol_stack"]
    pub fn nw_shim_parameters_copy_default_protocol_stack(parameters: *mut c_void) -> *mut c_void;
    #[link_name = "nw_shim_protocol_stack_clear_application_protocols"]
    pub fn nw_shim_protocol_stack_clear_application_protocols(stack: *mut c_void);
    #[link_name = "nw_shim_protocol_stack_copy_application_protocols"]
    pub fn nw_shim_protocol_stack_copy_application_protocols(
        stack: *mut c_void,
        out_count: *mut usize,
    ) -> *mut *mut c_void;
    #[link_name = "nw_shim_protocol_stack_copy_transport_protocol"]
    pub fn nw_shim_protocol_stack_copy_transport_protocol(stack: *mut c_void) -> *mut c_void;
    #[link_name = "nw_shim_protocol_stack_set_transport_protocol"]
    pub fn nw_shim_protocol_stack_set_transport_protocol(stack: *mut c_void, protocol: *mut c_void);
    #[link_name = "nw_shim_protocol_stack_copy_internet_protocol"]
    pub fn nw_shim_protocol_stack_copy_internet_protocol(stack: *mut c_void) -> *mut c_void;
    #[link_name = "nw_shim_parameters_set_local_only"]
    pub fn nw_shim_parameters_set_local_only(parameters: *mut c_void, local_only: c_int);
    #[link_name = "nw_shim_parameters_get_local_only"]
    pub fn nw_shim_parameters_get_local_only(parameters: *mut c_void) -> c_int;
    #[link_name = "nw_shim_parameters_get_prefer_no_proxy"]
    pub fn nw_shim_parameters_get_prefer_no_proxy(parameters: *mut c_void) -> c_int;
    #[link_name = "nw_shim_parameters_set_expired_dns_behavior"]
    pub fn nw_shim_parameters_set_expired_dns_behavior(
        parameters: *mut c_void,
        expired_dns_behavior: c_int,
    );
    #[link_name = "nw_shim_parameters_get_expired_dns_behavior"]
    pub fn nw_shim_parameters_get_expired_dns_behavior(parameters: *mut c_void) -> c_int;
    #[link_name = "nw_shim_parameters_set_requires_dnssec_validation"]
    pub fn nw_shim_parameters_set_requires_dnssec_validation(
        parameters: *mut c_void,
        requires_dnssec_validation: c_int,
    );
    #[link_name = "nw_shim_parameters_requires_dnssec_validation"]
    pub fn nw_shim_parameters_requires_dnssec_validation(parameters: *mut c_void) -> c_int;

    #[link_name = "nw_shim_connection_copy_establishment_report"]
    pub fn nw_shim_connection_copy_establishment_report(handle: *mut c_void) -> *mut c_void;
    #[link_name = "nw_shim_establishment_report_get_duration_milliseconds"]
    pub fn nw_shim_establishment_report_get_duration_milliseconds(report: *mut c_void) -> u64;
    #[link_name = "nw_shim_establishment_report_get_attempt_started_after_milliseconds"]
    pub fn nw_shim_establishment_report_get_attempt_started_after_milliseconds(
        report: *mut c_void,
    ) -> u64;
    #[link_name = "nw_shim_establishment_report_get_previous_attempt_count"]
    pub fn nw_shim_establishment_report_get_previous_attempt_count(report: *mut c_void) -> u32;
    #[link_name = "nw_shim_establishment_report_get_used_proxy"]
    pub fn nw_shim_establishment_report_get_used_proxy(report: *mut c_void) -> c_int;
    #[link_name = "nw_shim_establishment_report_get_proxy_configured"]
    pub fn nw_shim_establishment_report_get_proxy_configured(report: *mut c_void) -> c_int;
    #[link_name = "nw_shim_establishment_report_copy_proxy_endpoint"]
    pub fn nw_shim_establishment_report_copy_proxy_endpoint(report: *mut c_void) -> *mut c_void;
    #[link_name = "nw_shim_establishment_report_copy_protocols"]
    pub fn nw_shim_establishment_report_copy_protocols(
        report: *mut c_void,
        out_count: *mut usize,
    ) -> *mut NwShimEstablishmentProtocolInfo;
    #[link_name = "nw_shim_establishment_report_copy_resolutions"]
    pub fn nw_shim_establishment_report_copy_resolutions(
        report: *mut c_void,
        out_count: *mut usize,
    ) -> *mut NwShimResolutionStepInfo;
    #[link_name = "nw_shim_establishment_report_copy_resolution_reports"]
    pub fn nw_shim_establishment_report_copy_resolution_reports(
        report: *mut c_void,
        out_count: *mut usize,
    ) -> *mut *mut c_void;
    #[link_name = "nw_shim_resolution_report_get_source"]
    pub fn nw_shim_resolution_report_get_source(report: *mut c_void) -> c_int;
    #[link_name = "nw_shim_resolution_report_get_milliseconds"]
    pub fn nw_shim_resolution_report_get_milliseconds(report: *mut c_void) -> u64;
    #[link_name = "nw_shim_resolution_report_get_endpoint_count"]
    pub fn nw_shim_resolution_report_get_endpoint_count(report: *mut c_void) -> u32;
    #[link_name = "nw_shim_resolution_report_copy_successful_endpoint"]
    pub fn nw_shim_resolution_report_copy_successful_endpoint(report: *mut c_void) -> *mut c_void;
    #[link_name = "nw_shim_resolution_report_copy_preferred_endpoint"]
    pub fn nw_shim_resolution_report_copy_preferred_endpoint(report: *mut c_void) -> *mut c_void;
    #[link_name = "nw_shim_resolution_report_get_protocol"]
    pub fn nw_shim_resolution_report_get_protocol(report: *mut c_void) -> c_int;

    #[link_name = "nw_shim_connection_create_data_transfer_report"]
    pub fn nw_shim_connection_create_data_transfer_report(handle: *mut c_void) -> *mut c_void;
    #[link_name = "nw_shim_data_transfer_report_collect"]
    pub fn nw_shim_data_transfer_report_collect(report: *mut c_void) -> c_int;
    #[link_name = "nw_shim_data_transfer_report_get_state"]
    pub fn nw_shim_data_transfer_report_get_state(report: *mut c_void) -> c_int;
    #[link_name = "nw_shim_data_transfer_report_all_paths"]
    pub fn nw_shim_data_transfer_report_all_paths() -> u32;
    #[link_name = "nw_shim_data_transfer_report_get_duration_milliseconds"]
    pub fn nw_shim_data_transfer_report_get_duration_milliseconds(report: *mut c_void) -> u64;
    #[link_name = "nw_shim_data_transfer_report_get_path_count"]
    pub fn nw_shim_data_transfer_report_get_path_count(report: *mut c_void) -> u32;
    #[link_name = "nw_shim_data_transfer_report_get_received_ip_packet_count"]
    pub fn nw_shim_data_transfer_report_get_received_ip_packet_count(
        report: *mut c_void,
        path_index: u32,
    ) -> u64;
    #[link_name = "nw_shim_data_transfer_report_get_sent_ip_packet_count"]
    pub fn nw_shim_data_transfer_report_get_sent_ip_packet_count(
        report: *mut c_void,
        path_index: u32,
    ) -> u64;
    #[link_name = "nw_shim_data_transfer_report_get_received_transport_byte_count"]
    pub fn nw_shim_data_transfer_report_get_received_transport_byte_count(
        report: *mut c_void,
        path_index: u32,
    ) -> u64;
    #[link_name = "nw_shim_data_transfer_report_get_received_transport_duplicate_byte_count"]
    pub fn nw_shim_data_transfer_report_get_received_transport_duplicate_byte_count(
        report: *mut c_void,
        path_index: u32,
    ) -> u64;
    #[link_name = "nw_shim_data_transfer_report_get_received_transport_out_of_order_byte_count"]
    pub fn nw_shim_data_transfer_report_get_received_transport_out_of_order_byte_count(
        report: *mut c_void,
        path_index: u32,
    ) -> u64;
    #[link_name = "nw_shim_data_transfer_report_get_sent_transport_byte_count"]
    pub fn nw_shim_data_transfer_report_get_sent_transport_byte_count(
        report: *mut c_void,
        path_index: u32,
    ) -> u64;
    #[link_name = "nw_shim_data_transfer_report_get_sent_transport_retransmitted_byte_count"]
    pub fn nw_shim_data_transfer_report_get_sent_transport_retransmitted_byte_count(
        report: *mut c_void,
        path_index: u32,
    ) -> u64;
    #[link_name = "nw_shim_data_transfer_report_get_transport_smoothed_rtt_milliseconds"]
    pub fn nw_shim_data_transfer_report_get_transport_smoothed_rtt_milliseconds(
        report: *mut c_void,
        path_index: u32,
    ) -> u64;
    #[link_name = "nw_shim_data_transfer_report_get_transport_minimum_rtt_milliseconds"]
    pub fn nw_shim_data_transfer_report_get_transport_minimum_rtt_milliseconds(
        report: *mut c_void,
        path_index: u32,
    ) -> u64;
    #[link_name = "nw_shim_data_transfer_report_get_transport_rtt_variance"]
    pub fn nw_shim_data_transfer_report_get_transport_rtt_variance(
        report: *mut c_void,
        path_index: u32,
    ) -> u64;
    #[link_name = "nw_shim_data_transfer_report_get_received_application_byte_count"]
    pub fn nw_shim_data_transfer_report_get_received_application_byte_count(
        report: *mut c_void,
        path_index: u32,
    ) -> u64;
    #[link_name = "nw_shim_data_transfer_report_get_sent_application_byte_count"]
    pub fn nw_shim_data_transfer_report_get_sent_application_byte_count(
        report: *mut c_void,
        path_index: u32,
    ) -> u64;
    #[link_name = "nw_shim_data_transfer_report_copy_path_interface"]
    pub fn nw_shim_data_transfer_report_copy_path_interface(
        report: *mut c_void,
        path_index: u32,
    ) -> *mut c_void;
    #[link_name = "nw_shim_data_transfer_report_get_path_radio_type"]
    pub fn nw_shim_data_transfer_report_get_path_radio_type(
        report: *mut c_void,
        path_index: u32,
    ) -> c_int;

    #[link_name = "nw_shim_endpoint_copy_address"]
    pub fn nw_shim_endpoint_copy_address(
        endpoint: *mut c_void,
        out_buffer: *mut c_void,
        out_buffer_length: usize,
    ) -> usize;
    #[link_name = "nw_shim_endpoint_copy_txt_record"]
    pub fn nw_shim_endpoint_copy_txt_record(endpoint: *mut c_void) -> *mut c_void;

    #[link_name = "nw_shim_txt_record_create_with_bytes"]
    pub fn nw_shim_txt_record_create_with_bytes(
        txt_bytes: *const u8,
        txt_length: usize,
    ) -> *mut c_void;
    #[link_name = "nw_shim_txt_record_create_dictionary"]
    pub fn nw_shim_txt_record_create_dictionary() -> *mut c_void;
    #[link_name = "nw_shim_txt_record_copy"]
    pub fn nw_shim_txt_record_copy(txt_record: *mut c_void) -> *mut c_void;
    #[link_name = "nw_shim_txt_record_find_key"]
    pub fn nw_shim_txt_record_find_key(txt_record: *mut c_void, key: *const c_char) -> c_int;
    #[link_name = "nw_shim_txt_record_copy_value"]
    pub fn nw_shim_txt_record_copy_value(
        txt_record: *mut c_void,
        key: *const c_char,
        out_value_length: *mut usize,
        out_found: *mut c_int,
    ) -> *mut u8;
    #[link_name = "nw_shim_txt_record_set_key"]
    pub fn nw_shim_txt_record_set_key(
        txt_record: *mut c_void,
        key: *const c_char,
        value: *const u8,
        value_length: usize,
    ) -> c_int;
    #[link_name = "nw_shim_txt_record_remove_key"]
    pub fn nw_shim_txt_record_remove_key(txt_record: *mut c_void, key: *const c_char) -> c_int;
    #[link_name = "nw_shim_txt_record_get_key_count"]
    pub fn nw_shim_txt_record_get_key_count(txt_record: *mut c_void) -> usize;
    #[link_name = "nw_shim_txt_record_copy_bytes"]
    pub fn nw_shim_txt_record_copy_bytes(
        txt_record: *mut c_void,
        out_length: *mut usize,
    ) -> *mut u8;
    #[link_name = "nw_shim_txt_record_apply"]
    pub fn nw_shim_txt_record_apply(
        txt_record: *mut c_void,
        callback: Option<TxtRecordEntryCallback>,
        user_info: *mut c_void,
    ) -> c_int;
    #[link_name = "nw_shim_txt_record_is_dictionary"]
    pub fn nw_shim_txt_record_is_dictionary(txt_record: *mut c_void) -> c_int;
    #[link_name = "nw_shim_txt_record_is_equal"]
    pub fn nw_shim_txt_record_is_equal(
        txt_record: *mut c_void,
        other_txt_record: *mut c_void,
    ) -> c_int;

    #[link_name = "nw_shim_connection_copy_quic_metadata"]
    pub fn nw_shim_connection_copy_quic_metadata(handle: *mut c_void) -> *mut c_void;
    #[link_name = "nw_shim_content_context_copy_quic_metadata"]
    pub fn nw_shim_content_context_copy_quic_metadata(context: *mut c_void) -> *mut c_void;
    #[link_name = "nw_shim_protocol_metadata_is_quic"]
    pub fn nw_shim_protocol_metadata_is_quic(metadata: *mut c_void) -> c_int;
    #[link_name = "nw_shim_quic_copy_sec_protocol_options"]
    pub fn nw_shim_quic_copy_sec_protocol_options(options: *mut c_void) -> *mut c_void;
    #[link_name = "nw_shim_quic_copy_sec_protocol_metadata"]
    pub fn nw_shim_quic_copy_sec_protocol_metadata(metadata: *mut c_void) -> *mut c_void;
    #[link_name = "nw_shim_quic_get_initial_max_streams_bidirectional"]
    pub fn nw_shim_quic_get_initial_max_streams_bidirectional(options: *mut c_void) -> u64;
    #[link_name = "nw_shim_quic_set_initial_max_streams_bidirectional"]
    pub fn nw_shim_quic_set_initial_max_streams_bidirectional(
        options: *mut c_void,
        initial_max_streams_bidirectional: u64,
    );
    #[link_name = "nw_shim_quic_get_initial_max_streams_unidirectional"]
    pub fn nw_shim_quic_get_initial_max_streams_unidirectional(options: *mut c_void) -> u64;
    #[link_name = "nw_shim_quic_set_initial_max_streams_unidirectional"]
    pub fn nw_shim_quic_set_initial_max_streams_unidirectional(
        options: *mut c_void,
        initial_max_streams_unidirectional: u64,
    );
    #[link_name = "nw_shim_quic_get_initial_max_stream_data_bidirectional_local"]
    pub fn nw_shim_quic_get_initial_max_stream_data_bidirectional_local(
        options: *mut c_void,
    ) -> u64;
    #[link_name = "nw_shim_quic_set_initial_max_stream_data_bidirectional_local"]
    pub fn nw_shim_quic_set_initial_max_stream_data_bidirectional_local(
        options: *mut c_void,
        initial_max_stream_data_bidirectional_local: u64,
    );
    #[link_name = "nw_shim_quic_get_initial_max_stream_data_bidirectional_remote"]
    pub fn nw_shim_quic_get_initial_max_stream_data_bidirectional_remote(
        options: *mut c_void,
    ) -> u64;
    #[link_name = "nw_shim_quic_set_initial_max_stream_data_bidirectional_remote"]
    pub fn nw_shim_quic_set_initial_max_stream_data_bidirectional_remote(
        options: *mut c_void,
        initial_max_stream_data_bidirectional_remote: u64,
    );
    #[link_name = "nw_shim_quic_get_initial_max_stream_data_unidirectional"]
    pub fn nw_shim_quic_get_initial_max_stream_data_unidirectional(options: *mut c_void) -> u64;
    #[link_name = "nw_shim_quic_set_initial_max_stream_data_unidirectional"]
    pub fn nw_shim_quic_set_initial_max_stream_data_unidirectional(
        options: *mut c_void,
        initial_max_stream_data_unidirectional: u64,
    );
    #[link_name = "nw_shim_quic_get_max_datagram_frame_size"]
    pub fn nw_shim_quic_get_max_datagram_frame_size(options: *mut c_void) -> u16;
    #[link_name = "nw_shim_quic_set_max_datagram_frame_size"]
    pub fn nw_shim_quic_set_max_datagram_frame_size(
        options: *mut c_void,
        max_datagram_frame_size: u16,
    );
    #[link_name = "nw_shim_quic_get_stream_id"]
    pub fn nw_shim_quic_get_stream_id(metadata: *mut c_void) -> u64;
    #[link_name = "nw_shim_quic_get_stream_type"]
    pub fn nw_shim_quic_get_stream_type(metadata: *mut c_void) -> c_int;
    #[link_name = "nw_shim_quic_get_stream_application_error"]
    pub fn nw_shim_quic_get_stream_application_error(metadata: *mut c_void) -> u64;
    #[link_name = "nw_shim_quic_set_stream_application_error"]
    pub fn nw_shim_quic_set_stream_application_error(metadata: *mut c_void, application_error: u64);
    #[link_name = "nw_shim_quic_get_local_max_streams_bidirectional"]
    pub fn nw_shim_quic_get_local_max_streams_bidirectional(metadata: *mut c_void) -> u64;
    #[link_name = "nw_shim_quic_set_local_max_streams_bidirectional"]
    pub fn nw_shim_quic_set_local_max_streams_bidirectional(
        metadata: *mut c_void,
        max_streams_bidirectional: u64,
    );
    #[link_name = "nw_shim_quic_get_local_max_streams_unidirectional"]
    pub fn nw_shim_quic_get_local_max_streams_unidirectional(metadata: *mut c_void) -> u64;
    #[link_name = "nw_shim_quic_set_local_max_streams_unidirectional"]
    pub fn nw_shim_quic_set_local_max_streams_unidirectional(
        metadata: *mut c_void,
        max_streams_unidirectional: u64,
    );
    #[link_name = "nw_shim_quic_get_remote_max_streams_bidirectional"]
    pub fn nw_shim_quic_get_remote_max_streams_bidirectional(metadata: *mut c_void) -> u64;
    #[link_name = "nw_shim_quic_get_remote_max_streams_unidirectional"]
    pub fn nw_shim_quic_get_remote_max_streams_unidirectional(metadata: *mut c_void) -> u64;
    #[link_name = "nw_shim_quic_get_stream_usable_datagram_frame_size"]
    pub fn nw_shim_quic_get_stream_usable_datagram_frame_size(metadata: *mut c_void) -> u16;
    #[link_name = "nw_shim_quic_get_application_error"]
    pub fn nw_shim_quic_get_application_error(metadata: *mut c_void) -> u64;
    #[link_name = "nw_shim_quic_copy_application_error_reason"]
    pub fn nw_shim_quic_copy_application_error_reason(metadata: *mut c_void) -> *mut c_char;
    #[link_name = "nw_shim_quic_set_application_error"]
    pub fn nw_shim_quic_set_application_error(
        metadata: *mut c_void,
        application_error: u64,
        reason: *const c_char,
    );
    #[link_name = "nw_shim_quic_get_keepalive_interval"]
    pub fn nw_shim_quic_get_keepalive_interval(metadata: *mut c_void) -> u16;
    #[link_name = "nw_shim_quic_set_keepalive_interval"]
    pub fn nw_shim_quic_set_keepalive_interval(metadata: *mut c_void, keepalive_interval: u16);
    #[link_name = "nw_shim_quic_get_remote_idle_timeout"]
    pub fn nw_shim_quic_get_remote_idle_timeout(metadata: *mut c_void) -> u64;

    #[link_name = "nw_shim_ethernet_channel_create"]
    pub fn nw_shim_ethernet_channel_create(
        ether_type: u16,
        name: *const c_char,
        interface_type: c_int,
        index: u32,
    ) -> *mut c_void;
    #[link_name = "nw_shim_ethernet_channel_create_with_parameters"]
    pub fn nw_shim_ethernet_channel_create_with_parameters(
        ether_type: u16,
        name: *const c_char,
        interface_type: c_int,
        index: u32,
        parameters: *mut c_void,
    ) -> *mut c_void;
    #[link_name = "nw_shim_ethernet_channel_set_state_changed_handler"]
    pub fn nw_shim_ethernet_channel_set_state_changed_handler(
        handle: *mut c_void,
        callback: Option<EthernetChannelStateCallback>,
        user_info: *mut c_void,
    );
    #[link_name = "nw_shim_ethernet_channel_set_receive_handler"]
    pub fn nw_shim_ethernet_channel_set_receive_handler(
        handle: *mut c_void,
        callback: Option<EthernetChannelReceiveCallback>,
        user_info: *mut c_void,
    );
    #[link_name = "nw_shim_ethernet_channel_get_maximum_payload_size"]
    pub fn nw_shim_ethernet_channel_get_maximum_payload_size(handle: *mut c_void) -> u32;
    #[link_name = "nw_shim_ethernet_channel_start"]
    pub fn nw_shim_ethernet_channel_start(handle: *mut c_void);
    #[link_name = "nw_shim_ethernet_channel_cancel"]
    pub fn nw_shim_ethernet_channel_cancel(handle: *mut c_void);
    #[link_name = "nw_shim_ethernet_channel_send"]
    pub fn nw_shim_ethernet_channel_send(
        handle: *mut c_void,
        data: *const u8,
        len: usize,
        vlan_tag: u16,
        remote_address: *const u8,
    ) -> c_int;
    #[link_name = "nw_shim_ethernet_channel_release"]
    pub fn nw_shim_ethernet_channel_release(handle: *mut c_void);

    #[link_name = "nw_shim_advertise_descriptor_set_txt_record_object"]
    pub fn nw_shim_advertise_descriptor_set_txt_record_object(
        descriptor: *mut c_void,
        txt_record: *mut c_void,
    );
    #[link_name = "nw_shim_advertise_descriptor_copy_txt_record_object"]
    pub fn nw_shim_advertise_descriptor_copy_txt_record_object(
        descriptor: *mut c_void,
    ) -> *mut c_void;

    #[link_name = "nw_shim_browser_start_results_with_descriptor"]
    pub fn nw_shim_browser_start_results_with_descriptor(
        descriptor: *mut c_void,
        parameters: *mut c_void,
        callback: Option<BrowseResultChangedCallback>,
        user_info: *mut c_void,
    ) -> *mut c_void;
    #[link_name = "nw_shim_browser_copy_browse_descriptor"]
    pub fn nw_shim_browser_copy_browse_descriptor(handle: *mut c_void) -> *mut c_void;
    #[link_name = "nw_shim_browser_copy_parameters"]
    pub fn nw_shim_browser_copy_parameters(handle: *mut c_void) -> *mut c_void;
    #[link_name = "nw_shim_browser_set_state_changed_handler"]
    pub fn nw_shim_browser_set_state_changed_handler(
        handle: *mut c_void,
        callback: Option<BrowserStateChangedCallback>,
        user_info: *mut c_void,
    );
    #[link_name = "nw_shim_browser_set_browse_results_changed_handler"]
    pub fn nw_shim_browser_set_browse_results_changed_handler(
        handle: *mut c_void,
        callback: Option<BrowseResultChangedCallback>,
        user_info: *mut c_void,
    );
    #[link_name = "nw_shim_browser_drain_queue"]
    pub fn nw_shim_browser_drain_queue(handle: *mut c_void);
    #[link_name = "nw_shim_browse_result_get_changes"]
    pub fn nw_shim_browse_result_get_changes(
        old_result: *mut c_void,
        new_result: *mut c_void,
    ) -> u64;
    #[link_name = "nw_shim_browse_result_copy_endpoint"]
    pub fn nw_shim_browse_result_copy_endpoint(result: *mut c_void) -> *mut c_void;
    #[link_name = "nw_shim_browse_result_get_interfaces_count"]
    pub fn nw_shim_browse_result_get_interfaces_count(result: *mut c_void) -> usize;
    #[link_name = "nw_shim_browse_result_copy_txt_record_object"]
    pub fn nw_shim_browse_result_copy_txt_record_object(result: *mut c_void) -> *mut c_void;
    #[link_name = "nw_shim_browse_result_enumerate_interfaces"]
    pub fn nw_shim_browse_result_enumerate_interfaces(
        result: *mut c_void,
        callback: Option<InterfaceEnumerationCallback>,
        user_info: *mut c_void,
    ) -> c_int;

    #[link_name = "nw_shim_connection_set_state_changed_handler"]
    pub fn nw_shim_connection_set_state_changed_handler(
        handle: *mut c_void,
        callback: Option<ConnectionStateCallback>,
        user_info: *mut c_void,
    );
    #[link_name = "nw_shim_connection_drain_queue"]
    pub fn nw_shim_connection_drain_queue(handle: *mut c_void);
    #[link_name = "nw_shim_connection_set_viability_changed_handler"]
    pub fn nw_shim_connection_set_viability_changed_handler(
        handle: *mut c_void,
        callback: Option<ConnectionBooleanCallback>,
        user_info: *mut c_void,
    );
    #[link_name = "nw_shim_connection_set_better_path_available_handler"]
    pub fn nw_shim_connection_set_better_path_available_handler(
        handle: *mut c_void,
        callback: Option<ConnectionBooleanCallback>,
        user_info: *mut c_void,
    );
    #[link_name = "nw_shim_connection_set_path_changed_handler"]
    pub fn nw_shim_connection_set_path_changed_handler(
        handle: *mut c_void,
        callback: Option<ConnectionPathCallback>,
        user_info: *mut c_void,
    );
    #[link_name = "nw_shim_connection_restart"]
    pub fn nw_shim_connection_restart(handle: *mut c_void);
    #[link_name = "nw_shim_connection_force_cancel"]
    pub fn nw_shim_connection_force_cancel(handle: *mut c_void);
    #[link_name = "nw_shim_connection_cancel_current_endpoint"]
    pub fn nw_shim_connection_cancel_current_endpoint(handle: *mut c_void);
    #[link_name = "nw_shim_connection_batch"]
    pub fn nw_shim_connection_batch(
        handle: *mut c_void,
        callback: Option<ConnectionBatchCallback>,
        user_info: *mut c_void,
    );
    #[link_name = "nw_shim_connection_copy_description"]
    pub fn nw_shim_connection_copy_description(handle: *mut c_void) -> *mut c_char;
    #[link_name = "nw_shim_connection_copy_protocol_metadata"]
    pub fn nw_shim_connection_copy_protocol_metadata(
        handle: *mut c_void,
        definition: *mut c_void,
    ) -> *mut c_void;
    #[link_name = "nw_shim_connection_get_maximum_datagram_size"]
    pub fn nw_shim_connection_get_maximum_datagram_size(handle: *mut c_void) -> u32;
    #[link_name = "nw_shim_connection_release_without_cancel"]
    pub fn nw_shim_connection_release_without_cancel(handle: *mut c_void);

    #[link_name = "nw_shim_content_context_foreach_protocol_metadata"]
    pub fn nw_shim_content_context_foreach_protocol_metadata(
        context: *mut c_void,
        callback: Option<ProtocolMetadataEnumerationCallback>,
        user_info: *mut c_void,
    );

    #[link_name = "nw_shim_framer_copy_remote_endpoint"]
    pub fn nw_shim_framer_copy_remote_endpoint(framer: *mut c_void) -> *mut c_void;
    #[link_name = "nw_shim_framer_copy_local_endpoint"]
    pub fn nw_shim_framer_copy_local_endpoint(framer: *mut c_void) -> *mut c_void;
    #[link_name = "nw_shim_framer_copy_parameters"]
    pub fn nw_shim_framer_copy_parameters(framer: *mut c_void) -> *mut c_void;
    #[link_name = "nw_shim_framer_copy_options"]
    pub fn nw_shim_framer_copy_options(framer: *mut c_void) -> *mut c_void;

    #[link_name = "nw_shim_group_descriptor_enumerate_endpoints"]
    pub fn nw_shim_group_descriptor_enumerate_endpoints(
        descriptor: *mut c_void,
        callback: Option<EndpointEnumerationCallback>,
        user_info: *mut c_void,
    ) -> c_int;
    #[link_name = "nw_shim_multicast_group_descriptor_set_specific_source"]
    pub fn nw_shim_multicast_group_descriptor_set_specific_source(
        descriptor: *mut c_void,
        endpoint: *mut c_void,
    );
    #[link_name = "nw_shim_multicast_group_descriptor_get_disable_unicast_traffic"]
    pub fn nw_shim_multicast_group_descriptor_get_disable_unicast_traffic(
        descriptor: *mut c_void,
    ) -> c_int;
    #[link_name = "nw_shim_multicast_group_descriptor_set_disable_unicast_traffic"]
    pub fn nw_shim_multicast_group_descriptor_set_disable_unicast_traffic(
        descriptor: *mut c_void,
        disable_unicast_traffic: c_int,
    );

    #[link_name = "nw_shim_connection_group_copy_descriptor"]
    pub fn nw_shim_connection_group_copy_descriptor(handle: *mut c_void) -> *mut c_void;
    #[link_name = "nw_shim_connection_group_copy_parameters"]
    pub fn nw_shim_connection_group_copy_parameters(handle: *mut c_void) -> *mut c_void;
    #[link_name = "nw_shim_connection_group_copy_remote_endpoint_for_message"]
    pub fn nw_shim_connection_group_copy_remote_endpoint_for_message(
        handle: *mut c_void,
        context: *mut c_void,
    ) -> *mut c_void;
    #[link_name = "nw_shim_connection_group_copy_local_endpoint_for_message"]
    pub fn nw_shim_connection_group_copy_local_endpoint_for_message(
        handle: *mut c_void,
        context: *mut c_void,
    ) -> *mut c_void;
    #[link_name = "nw_shim_connection_group_copy_path_for_message"]
    pub fn nw_shim_connection_group_copy_path_for_message(
        handle: *mut c_void,
        context: *mut c_void,
    ) -> *mut c_void;
    #[link_name = "nw_shim_connection_group_copy_protocol_metadata_for_message"]
    pub fn nw_shim_connection_group_copy_protocol_metadata_for_message(
        handle: *mut c_void,
        context: *mut c_void,
        definition: *mut c_void,
    ) -> *mut c_void;
    #[link_name = "nw_shim_connection_group_copy_protocol_metadata"]
    pub fn nw_shim_connection_group_copy_protocol_metadata(
        handle: *mut c_void,
        definition: *mut c_void,
    ) -> *mut c_void;
    #[link_name = "nw_shim_connection_group_extract_connection_for_message"]
    pub fn nw_shim_connection_group_extract_connection_for_message(
        handle: *mut c_void,
        context: *mut c_void,
        out_status: *mut c_int,
    ) -> *mut c_void;
    #[link_name = "nw_shim_connection_group_extract_connection"]
    pub fn nw_shim_connection_group_extract_connection(
        handle: *mut c_void,
        endpoint: *mut c_void,
        protocol_options: *mut c_void,
        out_status: *mut c_int,
    ) -> *mut c_void;
    #[link_name = "nw_shim_connection_group_reinsert_extracted_connection"]
    pub fn nw_shim_connection_group_reinsert_extracted_connection(
        handle: *mut c_void,
        connection_handle: *mut c_void,
    ) -> c_int;
    #[link_name = "nw_shim_connection_group_reply"]
    pub fn nw_shim_connection_group_reply(
        handle: *mut c_void,
        inbound_message: *mut c_void,
        outbound_message: *mut c_void,
        data: *const u8,
        len: usize,
    ) -> c_int;
    #[link_name = "nw_shim_connection_group_set_new_connection_handler"]
    pub fn nw_shim_connection_group_set_new_connection_handler(
        handle: *mut c_void,
        callback: Option<ConnectionGroupNewConnectionCallback>,
        user_info: *mut c_void,
    );

    #[link_name = "nw_shim_error_get_domain"]
    pub fn nw_shim_error_get_domain(error: *mut c_void) -> c_int;
    #[link_name = "nw_shim_error_get_code"]
    pub fn nw_shim_error_get_code(error: *mut c_void) -> c_int;
    #[link_name = "nw_shim_error_copy_cf_error"]
    pub fn nw_shim_error_copy_cf_error(error: *mut c_void) -> *mut c_void;
    #[link_name = "nw_shim_error_copy_cf_error_domain"]
    pub fn nw_shim_error_copy_cf_error_domain(error: *mut c_void) -> *mut c_char;
    #[link_name = "nw_shim_error_copy_cf_error_description"]
    pub fn nw_shim_error_copy_cf_error_description(error: *mut c_void) -> *mut c_char;
    #[link_name = "nw_shim_error_copy_posix_domain"]
    pub fn nw_shim_error_copy_posix_domain() -> *mut c_char;
    #[link_name = "nw_shim_error_copy_dns_domain"]
    pub fn nw_shim_error_copy_dns_domain() -> *mut c_char;
    #[link_name = "nw_shim_error_copy_tls_domain"]
    pub fn nw_shim_error_copy_tls_domain() -> *mut c_char;
    #[link_name = "nw_shim_error_copy_wifi_aware_domain"]
    pub fn nw_shim_error_copy_wifi_aware_domain() -> *mut c_char;

    #[link_name = "nw_shim_parameters_create_custom_ip"]
    pub fn nw_shim_parameters_create_custom_ip(protocol_number: u8) -> *mut c_void;

    #[link_name = "nw_shim_listener_create_direct"]
    pub fn nw_shim_listener_create_direct(
        parameters: *mut c_void,
        out_status: *mut c_int,
    ) -> *mut c_void;
    #[link_name = "nw_shim_listener_create_with_connection"]
    pub fn nw_shim_listener_create_with_connection(
        connection_handle: *mut c_void,
        parameters: *mut c_void,
        out_status: *mut c_int,
    ) -> *mut c_void;
    #[link_name = "nw_shim_listener_create_with_launchd_key"]
    pub fn nw_shim_listener_create_with_launchd_key(
        parameters: *mut c_void,
        launchd_key: *const c_char,
        out_status: *mut c_int,
    ) -> *mut c_void;
    #[link_name = "nw_shim_listener_get_new_connection_limit"]
    pub fn nw_shim_listener_get_new_connection_limit(handle: *mut c_void) -> u32;
    #[link_name = "nw_shim_listener_set_new_connection_limit"]
    pub fn nw_shim_listener_set_new_connection_limit(
        handle: *mut c_void,
        new_connection_limit: u32,
    );
    #[link_name = "nw_shim_listener_set_advertised_endpoint_changed_handler"]
    pub fn nw_shim_listener_set_advertised_endpoint_changed_handler(
        handle: *mut c_void,
        callback: Option<ListenerAdvertisedEndpointChangedCallback>,
        user_info: *mut c_void,
    );
    #[link_name = "nw_shim_listener_set_new_connection_group_handler"]
    pub fn nw_shim_listener_set_new_connection_group_handler(
        handle: *mut c_void,
        callback: Option<ListenerNewConnectionGroupCallback>,
        user_info: *mut c_void,
    );
    #[link_name = "nw_shim_listener_set_state_changed_handler"]
    pub fn nw_shim_listener_set_state_changed_handler(
        handle: *mut c_void,
        callback: Option<ListenerStateCallback>,
        user_info: *mut c_void,
    );
    #[link_name = "nw_shim_listener_set_new_connection_handler"]
    pub fn nw_shim_listener_set_new_connection_handler(
        handle: *mut c_void,
        callback: Option<ListenerNewConnectionCallback>,
        user_info: *mut c_void,
    );
    #[link_name = "nw_shim_listener_drain_queue"]
    pub fn nw_shim_listener_drain_queue(handle: *mut c_void);

    #[link_name = "nw_shim_path_enumerate_gateways"]
    pub fn nw_shim_path_enumerate_gateways(
        path: *mut c_void,
        callback: Option<EndpointEnumerationCallback>,
        user_info: *mut c_void,
    ) -> c_int;
    #[link_name = "nw_shim_path_monitor_start_with_type"]
    pub fn nw_shim_path_monitor_start_with_type(
        interface_type: c_int,
        callback: PathMonitorCallback,
        user_info: *mut c_void,
    ) -> *mut c_void;
    #[link_name = "nw_shim_path_monitor_start_for_ethernet_channel"]
    pub fn nw_shim_path_monitor_start_for_ethernet_channel(
        callback: PathMonitorCallback,
        user_info: *mut c_void,
    ) -> *mut c_void;
    #[link_name = "nw_shim_path_monitor_prohibit_interface_type"]
    pub fn nw_shim_path_monitor_prohibit_interface_type(handle: *mut c_void, interface_type: c_int);
    #[link_name = "nw_shim_path_monitor_set_cancel_handler"]
    pub fn nw_shim_path_monitor_set_cancel_handler(
        handle: *mut c_void,
        callback: Option<PathMonitorCancelCallback>,
        user_info: *mut c_void,
    );
    #[link_name = "nw_shim_path_monitor_set_update_handler"]
    pub fn nw_shim_path_monitor_set_update_handler(
        handle: *mut c_void,
        callback: Option<ConnectionPathCallback>,
        user_info: *mut c_void,
    );
    #[link_name = "nw_shim_path_monitor_drain_queue"]
    pub fn nw_shim_path_monitor_drain_queue(handle: *mut c_void);

    #[link_name = "nw_shim_protocol_create_ip_metadata"]
    pub fn nw_shim_protocol_create_ip_metadata() -> *mut c_void;
    #[link_name = "nw_shim_protocol_create_udp_metadata"]
    pub fn nw_shim_protocol_create_udp_metadata() -> *mut c_void;
    #[link_name = "nw_shim_protocol_create_ws_options_with_version"]
    pub fn nw_shim_protocol_create_ws_options_with_version(version: c_int) -> *mut c_void;
    #[link_name = "nw_shim_protocol_create_ws_metadata"]
    pub fn nw_shim_protocol_create_ws_metadata(opcode: c_int) -> *mut c_void;
    #[link_name = "nw_shim_protocol_metadata_is_ip"]
    pub fn nw_shim_protocol_metadata_is_ip(metadata: *mut c_void) -> c_int;
    #[link_name = "nw_shim_protocol_metadata_is_tcp"]
    pub fn nw_shim_protocol_metadata_is_tcp(metadata: *mut c_void) -> c_int;
    #[link_name = "nw_shim_protocol_metadata_is_tls"]
    pub fn nw_shim_protocol_metadata_is_tls(metadata: *mut c_void) -> c_int;
    #[link_name = "nw_shim_protocol_metadata_is_udp"]
    pub fn nw_shim_protocol_metadata_is_udp(metadata: *mut c_void) -> c_int;
    #[link_name = "nw_shim_protocol_metadata_is_ws"]
    pub fn nw_shim_protocol_metadata_is_ws(metadata: *mut c_void) -> c_int;

    #[link_name = "nw_shim_ip_options_set_version"]
    pub fn nw_shim_ip_options_set_version(options: *mut c_void, version: c_int);
    #[link_name = "nw_shim_ip_options_set_hop_limit"]
    pub fn nw_shim_ip_options_set_hop_limit(options: *mut c_void, hop_limit: u8);
    #[link_name = "nw_shim_ip_options_set_use_minimum_mtu"]
    pub fn nw_shim_ip_options_set_use_minimum_mtu(options: *mut c_void, use_minimum_mtu: c_int);
    #[link_name = "nw_shim_ip_options_set_disable_fragmentation"]
    pub fn nw_shim_ip_options_set_disable_fragmentation(
        options: *mut c_void,
        disable_fragmentation: c_int,
    );
    #[link_name = "nw_shim_ip_options_set_calculate_receive_time"]
    pub fn nw_shim_ip_options_set_calculate_receive_time(
        options: *mut c_void,
        calculate_receive_time: c_int,
    );
    #[link_name = "nw_shim_ip_options_set_local_address_preference"]
    pub fn nw_shim_ip_options_set_local_address_preference(options: *mut c_void, preference: c_int);
    #[link_name = "nw_shim_ip_options_set_disable_multicast_loopback"]
    pub fn nw_shim_ip_options_set_disable_multicast_loopback(
        options: *mut c_void,
        disable_multicast_loopback: c_int,
    );
    #[link_name = "nw_shim_ip_metadata_set_ecn_flag"]
    pub fn nw_shim_ip_metadata_set_ecn_flag(metadata: *mut c_void, ecn_flag: c_int);
    #[link_name = "nw_shim_ip_metadata_get_ecn_flag"]
    pub fn nw_shim_ip_metadata_get_ecn_flag(metadata: *mut c_void) -> c_int;
    #[link_name = "nw_shim_ip_metadata_set_service_class"]
    pub fn nw_shim_ip_metadata_set_service_class(metadata: *mut c_void, service_class: c_int);
    #[link_name = "nw_shim_ip_metadata_get_service_class"]
    pub fn nw_shim_ip_metadata_get_service_class(metadata: *mut c_void) -> c_int;
    #[link_name = "nw_shim_ip_metadata_get_receive_time"]
    pub fn nw_shim_ip_metadata_get_receive_time(metadata: *mut c_void) -> u64;

    #[link_name = "nw_shim_tcp_options_set_no_delay"]
    pub fn nw_shim_tcp_options_set_no_delay(options: *mut c_void, no_delay: c_int);
    #[link_name = "nw_shim_tcp_options_set_no_push"]
    pub fn nw_shim_tcp_options_set_no_push(options: *mut c_void, no_push: c_int);
    #[link_name = "nw_shim_tcp_options_set_no_options"]
    pub fn nw_shim_tcp_options_set_no_options(options: *mut c_void, no_options: c_int);
    #[link_name = "nw_shim_tcp_options_set_enable_keepalive"]
    pub fn nw_shim_tcp_options_set_enable_keepalive(options: *mut c_void, enable_keepalive: c_int);
    #[link_name = "nw_shim_tcp_options_set_keepalive_count"]
    pub fn nw_shim_tcp_options_set_keepalive_count(options: *mut c_void, keepalive_count: u32);
    #[link_name = "nw_shim_tcp_options_set_keepalive_idle_time"]
    pub fn nw_shim_tcp_options_set_keepalive_idle_time(
        options: *mut c_void,
        keepalive_idle_time: u32,
    );
    #[link_name = "nw_shim_tcp_options_set_keepalive_interval"]
    pub fn nw_shim_tcp_options_set_keepalive_interval(
        options: *mut c_void,
        keepalive_interval: u32,
    );
    #[link_name = "nw_shim_tcp_options_set_maximum_segment_size"]
    pub fn nw_shim_tcp_options_set_maximum_segment_size(
        options: *mut c_void,
        maximum_segment_size: u32,
    );
    #[link_name = "nw_shim_tcp_options_set_connection_timeout"]
    pub fn nw_shim_tcp_options_set_connection_timeout(
        options: *mut c_void,
        connection_timeout: u32,
    );
    #[link_name = "nw_shim_tcp_options_set_persist_timeout"]
    pub fn nw_shim_tcp_options_set_persist_timeout(options: *mut c_void, persist_timeout: u32);
    #[link_name = "nw_shim_tcp_options_set_retransmit_connection_drop_time"]
    pub fn nw_shim_tcp_options_set_retransmit_connection_drop_time(
        options: *mut c_void,
        value: u32,
    );
    #[link_name = "nw_shim_tcp_options_set_retransmit_fin_drop"]
    pub fn nw_shim_tcp_options_set_retransmit_fin_drop(
        options: *mut c_void,
        retransmit_fin_drop: c_int,
    );
    #[link_name = "nw_shim_tcp_options_set_disable_ack_stretching"]
    pub fn nw_shim_tcp_options_set_disable_ack_stretching(
        options: *mut c_void,
        disable_ack_stretching: c_int,
    );
    #[link_name = "nw_shim_tcp_options_set_enable_fast_open"]
    pub fn nw_shim_tcp_options_set_enable_fast_open(options: *mut c_void, enable_fast_open: c_int);
    #[link_name = "nw_shim_tcp_options_set_disable_ecn"]
    pub fn nw_shim_tcp_options_set_disable_ecn(options: *mut c_void, disable_ecn: c_int);
    #[link_name = "nw_shim_tcp_options_set_multipath_force_version"]
    pub fn nw_shim_tcp_options_set_multipath_force_version(
        options: *mut c_void,
        multipath_force_version: c_int,
    );
    #[link_name = "nw_shim_tcp_get_available_receive_buffer"]
    pub fn nw_shim_tcp_get_available_receive_buffer(metadata: *mut c_void) -> u32;
    #[link_name = "nw_shim_tcp_get_available_send_buffer"]
    pub fn nw_shim_tcp_get_available_send_buffer(metadata: *mut c_void) -> u32;

    #[link_name = "nw_shim_tls_copy_sec_protocol_options"]
    pub fn nw_shim_tls_copy_sec_protocol_options(options: *mut c_void) -> *mut c_void;
    #[link_name = "nw_shim_tls_copy_sec_protocol_metadata"]
    pub fn nw_shim_tls_copy_sec_protocol_metadata(metadata: *mut c_void) -> *mut c_void;

    #[link_name = "nw_shim_udp_options_set_prefer_no_checksum"]
    pub fn nw_shim_udp_options_set_prefer_no_checksum(
        options: *mut c_void,
        prefer_no_checksum: c_int,
    );

    #[link_name = "nw_shim_ws_options_add_additional_header"]
    pub fn nw_shim_ws_options_add_additional_header(
        options: *mut c_void,
        name: *const c_char,
        value: *const c_char,
    );
    #[link_name = "nw_shim_ws_options_add_subprotocol"]
    pub fn nw_shim_ws_options_add_subprotocol(options: *mut c_void, subprotocol: *const c_char);
    #[link_name = "nw_shim_ws_options_set_auto_reply_ping"]
    pub fn nw_shim_ws_options_set_auto_reply_ping(options: *mut c_void, auto_reply_ping: c_int);
    #[link_name = "nw_shim_ws_options_set_skip_handshake"]
    pub fn nw_shim_ws_options_set_skip_handshake(options: *mut c_void, skip_handshake: c_int);
    #[link_name = "nw_shim_ws_options_set_maximum_message_size"]
    pub fn nw_shim_ws_options_set_maximum_message_size(
        options: *mut c_void,
        maximum_message_size: usize,
    );
    #[link_name = "nw_shim_ws_metadata_set_close_code"]
    pub fn nw_shim_ws_metadata_set_close_code(metadata: *mut c_void, close_code: c_int);
    #[link_name = "nw_shim_ws_metadata_get_close_code"]
    pub fn nw_shim_ws_metadata_get_close_code(metadata: *mut c_void) -> c_int;
    #[link_name = "nw_shim_ws_metadata_copy_server_response"]
    pub fn nw_shim_ws_metadata_copy_server_response(metadata: *mut c_void) -> *mut c_void;
    #[link_name = "nw_shim_ws_metadata_set_pong_handler"]
    pub fn nw_shim_ws_metadata_set_pong_handler(
        metadata: *mut c_void,
        callback: Option<WsPongCallback>,
        user_info: *mut c_void,
    );
    #[link_name = "nw_shim_ws_request_enumerate_subprotocols"]
    pub fn nw_shim_ws_request_enumerate_subprotocols(
        request: *mut c_void,
        callback: Option<StringEnumerationCallback>,
        user_info: *mut c_void,
    ) -> c_int;
    #[link_name = "nw_shim_ws_request_enumerate_additional_headers"]
    pub fn nw_shim_ws_request_enumerate_additional_headers(
        request: *mut c_void,
        callback: Option<HeaderEnumerationCallback>,
        user_info: *mut c_void,
    ) -> c_int;
    #[link_name = "nw_shim_ws_response_create"]
    pub fn nw_shim_ws_response_create(
        status: c_int,
        selected_subprotocol: *const c_char,
    ) -> *mut c_void;
    #[link_name = "nw_shim_ws_response_get_status"]
    pub fn nw_shim_ws_response_get_status(response: *mut c_void) -> c_int;
    #[link_name = "nw_shim_ws_response_get_selected_subprotocol"]
    pub fn nw_shim_ws_response_get_selected_subprotocol(response: *mut c_void) -> *mut c_char;
    #[link_name = "nw_shim_ws_response_add_additional_header"]
    pub fn nw_shim_ws_response_add_additional_header(
        response: *mut c_void,
        name: *const c_char,
        value: *const c_char,
    );
    #[link_name = "nw_shim_ws_response_enumerate_additional_headers"]
    pub fn nw_shim_ws_response_enumerate_additional_headers(
        response: *mut c_void,
        callback: Option<HeaderEnumerationCallback>,
        user_info: *mut c_void,
    ) -> c_int;

    #[link_name = "nw_shim_framer_message_set_object_value"]
    pub fn nw_shim_framer_message_set_object_value(
        message: *mut c_void,
        key: *const c_char,
        value: *mut c_void,
    );
    #[link_name = "nw_shim_framer_message_copy_object_value"]
    pub fn nw_shim_framer_message_copy_object_value(
        message: *mut c_void,
        key: *const c_char,
    ) -> *mut c_void;
    #[link_name = "nw_shim_framer_options_set_object_value"]
    pub fn nw_shim_framer_options_set_object_value(
        options: *mut c_void,
        key: *const c_char,
        value: *mut c_void,
    );
    #[link_name = "nw_shim_framer_options_copy_object_value"]
    pub fn nw_shim_framer_options_copy_object_value(
        options: *mut c_void,
        key: *const c_char,
    ) -> *mut c_void;
    #[link_name = "nw_shim_ws_options_set_client_request_handler"]
    pub fn nw_shim_ws_options_set_client_request_handler(
        options: *mut c_void,
        callback: Option<WsClientRequestCallback>,
        user_info: *mut c_void,
    );
    #[link_name = "nw_shim_url_session_configuration_default"]
    pub fn nw_shim_url_session_configuration_default() -> *mut c_void;
    #[link_name = "nw_shim_url_session_configuration_ephemeral"]
    pub fn nw_shim_url_session_configuration_ephemeral() -> *mut c_void;
    #[link_name = "nw_shim_url_session_configuration_release"]
    pub fn nw_shim_url_session_configuration_release(configuration: *mut c_void);
    #[link_name = "nw_shim_url_session_configuration_set_proxy_configurations"]
    pub fn nw_shim_url_session_configuration_set_proxy_configurations(
        configuration: *mut c_void,
        items: *const *mut c_void,
        count: usize,
    );
    #[link_name = "nw_shim_url_session_configuration_copy_proxy_configurations"]
    pub fn nw_shim_url_session_configuration_copy_proxy_configurations(
        configuration: *mut c_void,
        out_count: *mut usize,
    ) -> *mut *mut c_void;

}
