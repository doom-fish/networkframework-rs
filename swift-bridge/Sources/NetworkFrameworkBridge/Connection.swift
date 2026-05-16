import Foundation
import NetworkFrameworkCShim

@_cdecl("nfw_tcp_connect")
public func nfwTcpConnect(
    _ host: UnsafePointer<CChar>?,
    _ port: UInt16,
    _ useTLS: Int32,
    _ outStatus: UnsafeMutablePointer<Int32>?
) -> UnsafeMutableRawPointer? {
    nw_shim_tcp_connect(host, port, useTLS, outStatus)
}

@_cdecl("nfw_tcp_send")
public func nfwTcpSend(_ handle: UnsafeMutableRawPointer?, _ data: UnsafePointer<UInt8>?, _ len: Int) -> Int32 {
    nw_shim_tcp_send(handle, data, len)
}

@_cdecl("nfw_tcp_receive")
public func nfwTcpReceive(_ handle: UnsafeMutableRawPointer?, _ outBuf: UnsafeMutablePointer<UInt8>?, _ maxLen: Int) -> Int {
    nw_shim_tcp_receive(handle, outBuf, maxLen)
}

@_cdecl("nfw_tcp_close")
public func nfwTcpClose(_ handle: UnsafeMutableRawPointer?) {
    nw_shim_tcp_close(handle)
}

@_cdecl("nfw_connection_create_with_parameters")
public func nfwConnectionCreateWithParameters(
    _ host: UnsafePointer<CChar>?,
    _ port: UInt16,
    _ parameters: UnsafeMutableRawPointer?,
    _ outStatus: UnsafeMutablePointer<Int32>?
) -> UnsafeMutableRawPointer? {
    nw_shim_connection_create_with_parameters(host, port, parameters, outStatus)
}

@_cdecl("nfw_connection_copy_endpoint")
public func nfwConnectionCopyEndpoint(_ handle: UnsafeMutableRawPointer?) -> UnsafeMutableRawPointer? {
    nw_shim_connection_copy_endpoint(handle)
}

@_cdecl("nfw_connection_copy_parameters")
public func nfwConnectionCopyParameters(_ handle: UnsafeMutableRawPointer?) -> UnsafeMutableRawPointer? {
    nw_shim_connection_copy_parameters(handle)
}

@_cdecl("nfw_connection_copy_current_path")
public func nfwConnectionCopyCurrentPath(_ handle: UnsafeMutableRawPointer?) -> UnsafeMutableRawPointer? {
    nw_shim_connection_copy_current_path(handle)
}

@_cdecl("nfw_connection_send_with_context")
public func nfwConnectionSendWithContext(
    _ handle: UnsafeMutableRawPointer?,
    _ data: UnsafePointer<UInt8>?,
    _ len: Int,
    _ context: UnsafeMutableRawPointer?
) -> Int32 {
    nw_shim_connection_send_with_context(handle, data, len, context)
}

@_cdecl("nfw_connection_receive_with_context")
public func nfwConnectionReceiveWithContext(
    _ handle: UnsafeMutableRawPointer?,
    _ outBuf: UnsafeMutablePointer<UInt8>?,
    _ maxLen: Int,
    _ outContext: UnsafeMutablePointer<UnsafeMutableRawPointer?>?,
    _ outIsComplete: UnsafeMutablePointer<Int32>?
) -> Int {
    nw_shim_connection_receive_with_context(handle, outBuf, maxLen, outContext, outIsComplete)
}

@_cdecl("nfw_udp_connect")
public func nfwUDPConnect(_ host: UnsafePointer<CChar>?, _ port: UInt16, _ outStatus: UnsafeMutablePointer<Int32>?) -> UnsafeMutableRawPointer? {
    nw_shim_udp_connect(host, port, outStatus)
}

@_cdecl("nfw_ws_connect")
public func nfwWSConnect(
    _ host: UnsafePointer<CChar>?,
    _ port: UInt16,
    _ path: UnsafePointer<CChar>?,
    _ useTLS: Int32,
    _ outStatus: UnsafeMutablePointer<Int32>?
) -> UnsafeMutableRawPointer? {
    nw_shim_ws_connect(host, port, path, useTLS, outStatus)
}

@_cdecl("nfw_ws_send")
public func nfwWSSend(_ handle: UnsafeMutableRawPointer?, _ data: UnsafePointer<UInt8>?, _ len: Int, _ opcode: Int32) -> Int32 {
    nw_shim_ws_send(handle, data, len, opcode)
}

@_cdecl("nfw_ws_receive")
public func nfwWSReceive(
    _ handle: UnsafeMutableRawPointer?,
    _ outBuf: UnsafeMutablePointer<UInt8>?,
    _ maxLen: Int,
    _ outOpcode: UnsafeMutablePointer<Int32>?
) -> Int {
    nw_shim_ws_receive(handle, outBuf, maxLen, outOpcode)
}
