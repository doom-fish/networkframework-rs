import Foundation
import NetworkFrameworkCShim

@_cdecl("nfw_group_descriptor_create_multiplex")
public func nfwGroupDescriptorCreateMultiplex(_ host: UnsafePointer<CChar>?, _ port: UInt16) -> UnsafeMutableRawPointer? {
    nw_shim_group_descriptor_create_multiplex(host, port)
}

@_cdecl("nfw_group_descriptor_create_multicast")
public func nfwGroupDescriptorCreateMulticast(_ groupAddress: UnsafePointer<CChar>?, _ port: UInt16) -> UnsafeMutableRawPointer? {
    nw_shim_group_descriptor_create_multicast(groupAddress, port)
}

@_cdecl("nfw_group_descriptor_add_endpoint")
public func nfwGroupDescriptorAddEndpoint(_ descriptor: UnsafeMutableRawPointer?, _ host: UnsafePointer<CChar>?, _ port: UInt16) -> Int32 {
    nw_shim_group_descriptor_add_endpoint(descriptor, host, port)
}

@_cdecl("nfw_connection_group_create")
public func nfwConnectionGroupCreate(_ descriptor: UnsafeMutableRawPointer?, _ parameters: UnsafeMutableRawPointer?) -> UnsafeMutableRawPointer? {
    nw_shim_connection_group_create(descriptor, parameters)
}

@_cdecl("nfw_connection_group_start")
public func nfwConnectionGroupStart(_ handle: UnsafeMutableRawPointer?) -> Int32 { nw_shim_connection_group_start(handle) }

@_cdecl("nfw_connection_group_cancel")
public func nfwConnectionGroupCancel(_ handle: UnsafeMutableRawPointer?) { nw_shim_connection_group_cancel(handle) }

@_cdecl("nfw_connection_group_send")
public func nfwConnectionGroupSend(
    _ handle: UnsafeMutableRawPointer?,
    _ data: UnsafePointer<UInt8>?,
    _ len: Int,
    _ host: UnsafePointer<CChar>?,
    _ port: UInt16,
    _ context: UnsafeMutableRawPointer?
) -> Int32 {
    nw_shim_connection_group_send(handle, data, len, host, port, context)
}

@_cdecl("nfw_connection_group_release")
public func nfwConnectionGroupRelease(_ handle: UnsafeMutableRawPointer?) { nw_shim_connection_group_release(handle) }
