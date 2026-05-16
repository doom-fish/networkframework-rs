import Foundation
import NetworkFrameworkCShim

@_cdecl("nfw_listener_create")
public func nfwListenerCreate(_ port: UInt16, _ useTLS: Int32, _ outStatus: UnsafeMutablePointer<Int32>?) -> UnsafeMutableRawPointer? {
    nw_shim_listener_create(port, useTLS, outStatus)
}

@_cdecl("nfw_listener_port")
public func nfwListenerPort(_ handle: UnsafeMutableRawPointer?) -> UInt16 {
    nw_shim_listener_port(handle)
}

@_cdecl("nfw_listener_accept")
public func nfwListenerAccept(_ handle: UnsafeMutableRawPointer?, _ outStatus: UnsafeMutablePointer<Int32>?) -> UnsafeMutableRawPointer? {
    nw_shim_listener_accept(handle, outStatus)
}

@_cdecl("nfw_listener_close")
public func nfwListenerClose(_ handle: UnsafeMutableRawPointer?) {
    nw_shim_listener_close(handle)
}

@_cdecl("nfw_listener_create_with_parameters")
public func nfwListenerCreateWithParameters(
    _ parameters: UnsafeMutableRawPointer?,
    _ port: UInt16,
    _ outStatus: UnsafeMutablePointer<Int32>?
) -> UnsafeMutableRawPointer? {
    nw_shim_listener_create_with_parameters(parameters, port, outStatus)
}
