import Foundation
import NetworkFrameworkCShim

@_cdecl("nfw_sec_retain")
public func nfwSecRetain(_ object: UnsafeMutableRawPointer?) -> UnsafeMutableRawPointer? {
    nw_shim_sec_retain(object)
}

@_cdecl("nfw_sec_release")
public func nfwSecRelease(_ object: UnsafeMutableRawPointer?) {
    nw_shim_sec_release(object)
}

@_cdecl("nfw_connection_copy_quic_metadata")
public func nfwConnectionCopyQUICMetadata(_ handle: UnsafeMutableRawPointer?) -> UnsafeMutableRawPointer? {
    nw_shim_connection_copy_quic_metadata(handle)
}

@_cdecl("nfw_content_context_copy_quic_metadata")
public func nfwContentContextCopyQUICMetadata(_ context: UnsafeMutableRawPointer?) -> UnsafeMutableRawPointer? {
    nw_shim_content_context_copy_quic_metadata(context)
}

@_cdecl("nfw_protocol_metadata_is_quic")
public func nfwProtocolMetadataIsQUIC(_ metadata: UnsafeMutableRawPointer?) -> Int32 {
    nw_shim_protocol_metadata_is_quic(metadata)
}

@_cdecl("nfw_quic_copy_sec_protocol_options")
public func nfwQUICCopySecProtocolOptions(_ options: UnsafeMutableRawPointer?) -> UnsafeMutableRawPointer? {
    nw_shim_quic_copy_sec_protocol_options(options)
}

@_cdecl("nfw_quic_copy_sec_protocol_metadata")
public func nfwQUICCopySecProtocolMetadata(_ metadata: UnsafeMutableRawPointer?) -> UnsafeMutableRawPointer? {
    nw_shim_quic_copy_sec_protocol_metadata(metadata)
}

@_cdecl("nfw_quic_get_initial_max_streams_bidirectional")
public func nfwQUICGetInitialMaxStreamsBidirectional(_ options: UnsafeMutableRawPointer?) -> UInt64 {
    nw_shim_quic_get_initial_max_streams_bidirectional(options)
}

@_cdecl("nfw_quic_set_initial_max_streams_bidirectional")
public func nfwQUICSetInitialMaxStreamsBidirectional(_ options: UnsafeMutableRawPointer?, _ value: UInt64) {
    nw_shim_quic_set_initial_max_streams_bidirectional(options, value)
}

@_cdecl("nfw_quic_get_initial_max_streams_unidirectional")
public func nfwQUICGetInitialMaxStreamsUnidirectional(_ options: UnsafeMutableRawPointer?) -> UInt64 {
    nw_shim_quic_get_initial_max_streams_unidirectional(options)
}

@_cdecl("nfw_quic_set_initial_max_streams_unidirectional")
public func nfwQUICSetInitialMaxStreamsUnidirectional(_ options: UnsafeMutableRawPointer?, _ value: UInt64) {
    nw_shim_quic_set_initial_max_streams_unidirectional(options, value)
}

@_cdecl("nfw_quic_get_initial_max_stream_data_bidirectional_local")
public func nfwQUICGetInitialMaxStreamDataBidirectionalLocal(_ options: UnsafeMutableRawPointer?) -> UInt64 {
    nw_shim_quic_get_initial_max_stream_data_bidirectional_local(options)
}

@_cdecl("nfw_quic_set_initial_max_stream_data_bidirectional_local")
public func nfwQUICSetInitialMaxStreamDataBidirectionalLocal(_ options: UnsafeMutableRawPointer?, _ value: UInt64) {
    nw_shim_quic_set_initial_max_stream_data_bidirectional_local(options, value)
}

@_cdecl("nfw_quic_get_initial_max_stream_data_bidirectional_remote")
public func nfwQUICGetInitialMaxStreamDataBidirectionalRemote(_ options: UnsafeMutableRawPointer?) -> UInt64 {
    nw_shim_quic_get_initial_max_stream_data_bidirectional_remote(options)
}

@_cdecl("nfw_quic_set_initial_max_stream_data_bidirectional_remote")
public func nfwQUICSetInitialMaxStreamDataBidirectionalRemote(_ options: UnsafeMutableRawPointer?, _ value: UInt64) {
    nw_shim_quic_set_initial_max_stream_data_bidirectional_remote(options, value)
}

@_cdecl("nfw_quic_get_initial_max_stream_data_unidirectional")
public func nfwQUICGetInitialMaxStreamDataUnidirectional(_ options: UnsafeMutableRawPointer?) -> UInt64 {
    nw_shim_quic_get_initial_max_stream_data_unidirectional(options)
}

@_cdecl("nfw_quic_set_initial_max_stream_data_unidirectional")
public func nfwQUICSetInitialMaxStreamDataUnidirectional(_ options: UnsafeMutableRawPointer?, _ value: UInt64) {
    nw_shim_quic_set_initial_max_stream_data_unidirectional(options, value)
}

@_cdecl("nfw_quic_get_max_datagram_frame_size")
public func nfwQUICGetMaxDatagramFrameSize(_ options: UnsafeMutableRawPointer?) -> UInt16 {
    nw_shim_quic_get_max_datagram_frame_size(options)
}

@_cdecl("nfw_quic_set_max_datagram_frame_size")
public func nfwQUICSetMaxDatagramFrameSize(_ options: UnsafeMutableRawPointer?, _ value: UInt16) {
    nw_shim_quic_set_max_datagram_frame_size(options, value)
}

@_cdecl("nfw_quic_get_stream_id")
public func nfwQUICGetStreamID(_ metadata: UnsafeMutableRawPointer?) -> UInt64 {
    nw_shim_quic_get_stream_id(metadata)
}

@_cdecl("nfw_quic_get_stream_type")
public func nfwQUICGetStreamType(_ metadata: UnsafeMutableRawPointer?) -> Int32 {
    nw_shim_quic_get_stream_type(metadata)
}

@_cdecl("nfw_quic_get_stream_application_error")
public func nfwQUICGetStreamApplicationError(_ metadata: UnsafeMutableRawPointer?) -> UInt64 {
    nw_shim_quic_get_stream_application_error(metadata)
}

@_cdecl("nfw_quic_set_stream_application_error")
public func nfwQUICSetStreamApplicationError(_ metadata: UnsafeMutableRawPointer?, _ value: UInt64) {
    nw_shim_quic_set_stream_application_error(metadata, value)
}

@_cdecl("nfw_quic_get_local_max_streams_bidirectional")
public func nfwQUICGetLocalMaxStreamsBidirectional(_ metadata: UnsafeMutableRawPointer?) -> UInt64 {
    nw_shim_quic_get_local_max_streams_bidirectional(metadata)
}

@_cdecl("nfw_quic_set_local_max_streams_bidirectional")
public func nfwQUICSetLocalMaxStreamsBidirectional(_ metadata: UnsafeMutableRawPointer?, _ value: UInt64) {
    nw_shim_quic_set_local_max_streams_bidirectional(metadata, value)
}

@_cdecl("nfw_quic_get_local_max_streams_unidirectional")
public func nfwQUICGetLocalMaxStreamsUnidirectional(_ metadata: UnsafeMutableRawPointer?) -> UInt64 {
    nw_shim_quic_get_local_max_streams_unidirectional(metadata)
}

@_cdecl("nfw_quic_set_local_max_streams_unidirectional")
public func nfwQUICSetLocalMaxStreamsUnidirectional(_ metadata: UnsafeMutableRawPointer?, _ value: UInt64) {
    nw_shim_quic_set_local_max_streams_unidirectional(metadata, value)
}

@_cdecl("nfw_quic_get_remote_max_streams_bidirectional")
public func nfwQUICGetRemoteMaxStreamsBidirectional(_ metadata: UnsafeMutableRawPointer?) -> UInt64 {
    nw_shim_quic_get_remote_max_streams_bidirectional(metadata)
}

@_cdecl("nfw_quic_get_remote_max_streams_unidirectional")
public func nfwQUICGetRemoteMaxStreamsUnidirectional(_ metadata: UnsafeMutableRawPointer?) -> UInt64 {
    nw_shim_quic_get_remote_max_streams_unidirectional(metadata)
}

@_cdecl("nfw_quic_get_stream_usable_datagram_frame_size")
public func nfwQUICGetStreamUsableDatagramFrameSize(_ metadata: UnsafeMutableRawPointer?) -> UInt16 {
    nw_shim_quic_get_stream_usable_datagram_frame_size(metadata)
}

@_cdecl("nfw_quic_get_application_error")
public func nfwQUICGetApplicationError(_ metadata: UnsafeMutableRawPointer?) -> UInt64 {
    nw_shim_quic_get_application_error(metadata)
}

@_cdecl("nfw_quic_copy_application_error_reason")
public func nfwQUICCopyApplicationErrorReason(_ metadata: UnsafeMutableRawPointer?) -> UnsafeMutablePointer<CChar>? {
    nw_shim_quic_copy_application_error_reason(metadata)
}

@_cdecl("nfw_quic_set_application_error")
public func nfwQUICSetApplicationError(_ metadata: UnsafeMutableRawPointer?, _ applicationError: UInt64, _ reason: UnsafePointer<CChar>?) {
    nw_shim_quic_set_application_error(metadata, applicationError, reason)
}

@_cdecl("nfw_quic_get_keepalive_interval")
public func nfwQUICGetKeepaliveInterval(_ metadata: UnsafeMutableRawPointer?) -> UInt16 {
    nw_shim_quic_get_keepalive_interval(metadata)
}

@_cdecl("nfw_quic_set_keepalive_interval")
public func nfwQUICSetKeepaliveInterval(_ metadata: UnsafeMutableRawPointer?, _ keepaliveInterval: UInt16) {
    nw_shim_quic_set_keepalive_interval(metadata, keepaliveInterval)
}

@_cdecl("nfw_quic_get_remote_idle_timeout")
public func nfwQUICGetRemoteIdleTimeout(_ metadata: UnsafeMutableRawPointer?) -> UInt64 {
    nw_shim_quic_get_remote_idle_timeout(metadata)
}
