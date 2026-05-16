import Foundation
import NetworkFrameworkCShim

@_cdecl("nfw_quic_connect")
public func nfwQuicConnect(
    _ host: UnsafePointer<CChar>?,
    _ port: UInt16,
    _ alpn: UnsafePointer<CChar>?,
    _ outStatus: UnsafeMutablePointer<Int32>?
) -> UnsafeMutableRawPointer? {
    nw_shim_quic_connect(host, port, alpn, outStatus)
}

@_cdecl("nfw_quic_add_tls_application_protocol")
public func nfwQuicAddTLSApplicationProtocol(_ options: UnsafeMutableRawPointer?, _ applicationProtocol: UnsafePointer<CChar>?) {
    nw_shim_quic_add_tls_application_protocol(options, applicationProtocol)
}

@_cdecl("nfw_quic_get_stream_is_unidirectional")
public func nfwQuicGetStreamIsUnidirectional(_ options: UnsafeMutableRawPointer?) -> Int32 {
    nw_shim_quic_get_stream_is_unidirectional(options)
}

@_cdecl("nfw_quic_set_stream_is_unidirectional")
public func nfwQuicSetStreamIsUnidirectional(_ options: UnsafeMutableRawPointer?, _ isUnidirectional: Int32) {
    nw_shim_quic_set_stream_is_unidirectional(options, isUnidirectional)
}

@_cdecl("nfw_quic_get_stream_is_datagram")
public func nfwQuicGetStreamIsDatagram(_ options: UnsafeMutableRawPointer?) -> Int32 {
    nw_shim_quic_get_stream_is_datagram(options)
}

@_cdecl("nfw_quic_set_stream_is_datagram")
public func nfwQuicSetStreamIsDatagram(_ options: UnsafeMutableRawPointer?, _ isDatagram: Int32) {
    nw_shim_quic_set_stream_is_datagram(options, isDatagram)
}

@_cdecl("nfw_quic_get_initial_max_data")
public func nfwQuicGetInitialMaxData(_ options: UnsafeMutableRawPointer?) -> UInt64 { nw_shim_quic_get_initial_max_data(options) }

@_cdecl("nfw_quic_set_initial_max_data")
public func nfwQuicSetInitialMaxData(_ options: UnsafeMutableRawPointer?, _ initialMaxData: UInt64) {
    nw_shim_quic_set_initial_max_data(options, initialMaxData)
}

@_cdecl("nfw_quic_get_max_udp_payload_size")
public func nfwQuicGetMaxUDPPayloadSize(_ options: UnsafeMutableRawPointer?) -> UInt16 {
    nw_shim_quic_get_max_udp_payload_size(options)
}

@_cdecl("nfw_quic_set_max_udp_payload_size")
public func nfwQuicSetMaxUDPPayloadSize(_ options: UnsafeMutableRawPointer?, _ maxUDPPayloadSize: UInt16) {
    nw_shim_quic_set_max_udp_payload_size(options, maxUDPPayloadSize)
}

@_cdecl("nfw_quic_get_idle_timeout")
public func nfwQuicGetIdleTimeout(_ options: UnsafeMutableRawPointer?) -> UInt32 { nw_shim_quic_get_idle_timeout(options) }

@_cdecl("nfw_quic_set_idle_timeout")
public func nfwQuicSetIdleTimeout(_ options: UnsafeMutableRawPointer?, _ idleTimeout: UInt32) {
    nw_shim_quic_set_idle_timeout(options, idleTimeout)
}
