import Foundation
import NetworkFrameworkCShim

@_cdecl("nfw_connection_copy_establishment_report")
public func nfwConnectionCopyEstablishmentReport(_ handle: UnsafeMutableRawPointer?) -> UnsafeMutableRawPointer? {
    nw_shim_connection_copy_establishment_report(handle)
}

@_cdecl("nfw_establishment_report_get_duration_milliseconds")
public func nfwEstablishmentReportGetDurationMilliseconds(_ report: UnsafeMutableRawPointer?) -> UInt64 {
    nw_shim_establishment_report_get_duration_milliseconds(report)
}

@_cdecl("nfw_establishment_report_get_attempt_started_after_milliseconds")
public func nfwEstablishmentReportGetAttemptStartedAfterMilliseconds(_ report: UnsafeMutableRawPointer?) -> UInt64 {
    nw_shim_establishment_report_get_attempt_started_after_milliseconds(report)
}

@_cdecl("nfw_establishment_report_get_previous_attempt_count")
public func nfwEstablishmentReportGetPreviousAttemptCount(_ report: UnsafeMutableRawPointer?) -> UInt32 {
    nw_shim_establishment_report_get_previous_attempt_count(report)
}

@_cdecl("nfw_establishment_report_get_used_proxy")
public func nfwEstablishmentReportGetUsedProxy(_ report: UnsafeMutableRawPointer?) -> Int32 {
    nw_shim_establishment_report_get_used_proxy(report)
}

@_cdecl("nfw_establishment_report_get_proxy_configured")
public func nfwEstablishmentReportGetProxyConfigured(_ report: UnsafeMutableRawPointer?) -> Int32 {
    nw_shim_establishment_report_get_proxy_configured(report)
}

@_cdecl("nfw_establishment_report_copy_proxy_endpoint")
public func nfwEstablishmentReportCopyProxyEndpoint(_ report: UnsafeMutableRawPointer?) -> UnsafeMutableRawPointer? {
    nw_shim_establishment_report_copy_proxy_endpoint(report)
}

@_cdecl("nfw_establishment_report_copy_protocols")
public func nfwEstablishmentReportCopyProtocols(
    _ report: UnsafeMutableRawPointer?,
    _ outCount: UnsafeMutablePointer<Int>?
) -> UnsafeMutablePointer<nw_shim_establishment_protocol_info>? {
    nw_shim_establishment_report_copy_protocols(report, outCount)
}

@_cdecl("nfw_establishment_report_copy_resolutions")
public func nfwEstablishmentReportCopyResolutions(
    _ report: UnsafeMutableRawPointer?,
    _ outCount: UnsafeMutablePointer<Int>?
) -> UnsafeMutablePointer<nw_shim_resolution_step_info>? {
    nw_shim_establishment_report_copy_resolutions(report, outCount)
}

@_cdecl("nfw_establishment_report_copy_resolution_reports")
public func nfwEstablishmentReportCopyResolutionReports(
    _ report: UnsafeMutableRawPointer?,
    _ outCount: UnsafeMutablePointer<Int>?
) -> UnsafeMutablePointer<UnsafeMutableRawPointer?>? {
    nw_shim_establishment_report_copy_resolution_reports(report, outCount)
}

@_cdecl("nfw_resolution_report_get_source")
public func nfwResolutionReportGetSource(_ report: UnsafeMutableRawPointer?) -> Int32 {
    nw_shim_resolution_report_get_source(report)
}

@_cdecl("nfw_resolution_report_get_milliseconds")
public func nfwResolutionReportGetMilliseconds(_ report: UnsafeMutableRawPointer?) -> UInt64 {
    nw_shim_resolution_report_get_milliseconds(report)
}

@_cdecl("nfw_resolution_report_get_endpoint_count")
public func nfwResolutionReportGetEndpointCount(_ report: UnsafeMutableRawPointer?) -> UInt32 {
    nw_shim_resolution_report_get_endpoint_count(report)
}

@_cdecl("nfw_resolution_report_copy_successful_endpoint")
public func nfwResolutionReportCopySuccessfulEndpoint(_ report: UnsafeMutableRawPointer?) -> UnsafeMutableRawPointer? {
    nw_shim_resolution_report_copy_successful_endpoint(report)
}

@_cdecl("nfw_resolution_report_copy_preferred_endpoint")
public func nfwResolutionReportCopyPreferredEndpoint(_ report: UnsafeMutableRawPointer?) -> UnsafeMutableRawPointer? {
    nw_shim_resolution_report_copy_preferred_endpoint(report)
}

@_cdecl("nfw_resolution_report_get_protocol")
public func nfwResolutionReportGetProtocol(_ report: UnsafeMutableRawPointer?) -> Int32 {
    nw_shim_resolution_report_get_protocol(report)
}

@_cdecl("nfw_connection_create_data_transfer_report")
public func nfwConnectionCreateDataTransferReport(_ handle: UnsafeMutableRawPointer?) -> UnsafeMutableRawPointer? {
    nw_shim_connection_create_data_transfer_report(handle)
}

@_cdecl("nfw_data_transfer_report_collect")
public func nfwDataTransferReportCollect(_ report: UnsafeMutableRawPointer?) -> Int32 {
    nw_shim_data_transfer_report_collect(report)
}

@_cdecl("nfw_data_transfer_report_get_state")
public func nfwDataTransferReportGetState(_ report: UnsafeMutableRawPointer?) -> Int32 {
    nw_shim_data_transfer_report_get_state(report)
}

@_cdecl("nfw_data_transfer_report_all_paths")
public func nfwDataTransferReportAllPaths() -> UInt32 {
    nw_shim_data_transfer_report_all_paths()
}

@_cdecl("nfw_data_transfer_report_get_duration_milliseconds")
public func nfwDataTransferReportGetDurationMilliseconds(_ report: UnsafeMutableRawPointer?) -> UInt64 {
    nw_shim_data_transfer_report_get_duration_milliseconds(report)
}

@_cdecl("nfw_data_transfer_report_get_path_count")
public func nfwDataTransferReportGetPathCount(_ report: UnsafeMutableRawPointer?) -> UInt32 {
    nw_shim_data_transfer_report_get_path_count(report)
}

@_cdecl("nfw_data_transfer_report_get_received_ip_packet_count")
public func nfwDataTransferReportGetReceivedIPPacketCount(_ report: UnsafeMutableRawPointer?, _ pathIndex: UInt32) -> UInt64 {
    nw_shim_data_transfer_report_get_received_ip_packet_count(report, pathIndex)
}

@_cdecl("nfw_data_transfer_report_get_sent_ip_packet_count")
public func nfwDataTransferReportGetSentIPPacketCount(_ report: UnsafeMutableRawPointer?, _ pathIndex: UInt32) -> UInt64 {
    nw_shim_data_transfer_report_get_sent_ip_packet_count(report, pathIndex)
}

@_cdecl("nfw_data_transfer_report_get_received_transport_byte_count")
public func nfwDataTransferReportGetReceivedTransportByteCount(_ report: UnsafeMutableRawPointer?, _ pathIndex: UInt32) -> UInt64 {
    nw_shim_data_transfer_report_get_received_transport_byte_count(report, pathIndex)
}

@_cdecl("nfw_data_transfer_report_get_received_transport_duplicate_byte_count")
public func nfwDataTransferReportGetReceivedTransportDuplicateByteCount(_ report: UnsafeMutableRawPointer?, _ pathIndex: UInt32) -> UInt64 {
    nw_shim_data_transfer_report_get_received_transport_duplicate_byte_count(report, pathIndex)
}

@_cdecl("nfw_data_transfer_report_get_received_transport_out_of_order_byte_count")
public func nfwDataTransferReportGetReceivedTransportOutOfOrderByteCount(_ report: UnsafeMutableRawPointer?, _ pathIndex: UInt32) -> UInt64 {
    nw_shim_data_transfer_report_get_received_transport_out_of_order_byte_count(report, pathIndex)
}

@_cdecl("nfw_data_transfer_report_get_sent_transport_byte_count")
public func nfwDataTransferReportGetSentTransportByteCount(_ report: UnsafeMutableRawPointer?, _ pathIndex: UInt32) -> UInt64 {
    nw_shim_data_transfer_report_get_sent_transport_byte_count(report, pathIndex)
}

@_cdecl("nfw_data_transfer_report_get_sent_transport_retransmitted_byte_count")
public func nfwDataTransferReportGetSentTransportRetransmittedByteCount(_ report: UnsafeMutableRawPointer?, _ pathIndex: UInt32) -> UInt64 {
    nw_shim_data_transfer_report_get_sent_transport_retransmitted_byte_count(report, pathIndex)
}

@_cdecl("nfw_data_transfer_report_get_transport_smoothed_rtt_milliseconds")
public func nfwDataTransferReportGetTransportSmoothedRTTMilliseconds(_ report: UnsafeMutableRawPointer?, _ pathIndex: UInt32) -> UInt64 {
    nw_shim_data_transfer_report_get_transport_smoothed_rtt_milliseconds(report, pathIndex)
}

@_cdecl("nfw_data_transfer_report_get_transport_minimum_rtt_milliseconds")
public func nfwDataTransferReportGetTransportMinimumRTTMilliseconds(_ report: UnsafeMutableRawPointer?, _ pathIndex: UInt32) -> UInt64 {
    nw_shim_data_transfer_report_get_transport_minimum_rtt_milliseconds(report, pathIndex)
}

@_cdecl("nfw_data_transfer_report_get_transport_rtt_variance")
public func nfwDataTransferReportGetTransportRTTVariance(_ report: UnsafeMutableRawPointer?, _ pathIndex: UInt32) -> UInt64 {
    nw_shim_data_transfer_report_get_transport_rtt_variance(report, pathIndex)
}

@_cdecl("nfw_data_transfer_report_get_received_application_byte_count")
public func nfwDataTransferReportGetReceivedApplicationByteCount(_ report: UnsafeMutableRawPointer?, _ pathIndex: UInt32) -> UInt64 {
    nw_shim_data_transfer_report_get_received_application_byte_count(report, pathIndex)
}

@_cdecl("nfw_data_transfer_report_get_sent_application_byte_count")
public func nfwDataTransferReportGetSentApplicationByteCount(_ report: UnsafeMutableRawPointer?, _ pathIndex: UInt32) -> UInt64 {
    nw_shim_data_transfer_report_get_sent_application_byte_count(report, pathIndex)
}

@_cdecl("nfw_data_transfer_report_copy_path_interface")
public func nfwDataTransferReportCopyPathInterface(_ report: UnsafeMutableRawPointer?, _ pathIndex: UInt32) -> UnsafeMutableRawPointer? {
    nw_shim_data_transfer_report_copy_path_interface(report, pathIndex)
}

@_cdecl("nfw_data_transfer_report_get_path_radio_type")
public func nfwDataTransferReportGetPathRadioType(_ report: UnsafeMutableRawPointer?, _ pathIndex: UInt32) -> Int32 {
    nw_shim_data_transfer_report_get_path_radio_type(report, pathIndex)
}
