import Foundation
import NetworkFrameworkCShim

@_cdecl("nfw_ethernet_channel_create")
public func nfwEthernetChannelCreate(
    _ etherType: UInt16,
    _ name: UnsafePointer<CChar>?,
    _ interfaceType: Int32,
    _ index: UInt32
) -> UnsafeMutableRawPointer? {
    nw_shim_ethernet_channel_create(etherType, name, interfaceType, index)
}

@_cdecl("nfw_ethernet_channel_create_with_parameters")
public func nfwEthernetChannelCreateWithParameters(
    _ etherType: UInt16,
    _ name: UnsafePointer<CChar>?,
    _ interfaceType: Int32,
    _ index: UInt32,
    _ parameters: UnsafeMutableRawPointer?
) -> UnsafeMutableRawPointer? {
    nw_shim_ethernet_channel_create_with_parameters(etherType, name, interfaceType, index, parameters)
}

@_cdecl("nfw_ethernet_channel_set_state_changed_handler")
public func nfwEthernetChannelSetStateChangedHandler(
    _ handle: UnsafeMutableRawPointer?,
    _ callback: EthernetChannelStateCallback?,
    _ userInfo: UnsafeMutableRawPointer?
) {
    nw_shim_ethernet_channel_set_state_changed_handler(handle, callback, userInfo)
}

@_cdecl("nfw_ethernet_channel_set_receive_handler")
public func nfwEthernetChannelSetReceiveHandler(
    _ handle: UnsafeMutableRawPointer?,
    _ callback: EthernetChannelReceiveCallback?,
    _ userInfo: UnsafeMutableRawPointer?
) {
    nw_shim_ethernet_channel_set_receive_handler(handle, callback, userInfo)
}

@_cdecl("nfw_ethernet_channel_get_maximum_payload_size")
public func nfwEthernetChannelGetMaximumPayloadSize(_ handle: UnsafeMutableRawPointer?) -> UInt32 {
    nw_shim_ethernet_channel_get_maximum_payload_size(handle)
}

@_cdecl("nfw_ethernet_channel_start")
public func nfwEthernetChannelStart(_ handle: UnsafeMutableRawPointer?) {
    nw_shim_ethernet_channel_start(handle)
}

@_cdecl("nfw_ethernet_channel_cancel")
public func nfwEthernetChannelCancel(_ handle: UnsafeMutableRawPointer?) {
    nw_shim_ethernet_channel_cancel(handle)
}

@_cdecl("nfw_ethernet_channel_send")
public func nfwEthernetChannelSend(
    _ handle: UnsafeMutableRawPointer?,
    _ data: UnsafePointer<UInt8>?,
    _ len: Int,
    _ vlanTag: UInt16,
    _ remoteAddress: UnsafePointer<UInt8>?
) -> Int32 {
    nw_shim_ethernet_channel_send(handle, data, len, vlanTag, remoteAddress)
}

@_cdecl("nfw_ethernet_channel_release")
public func nfwEthernetChannelRelease(_ handle: UnsafeMutableRawPointer?) {
    nw_shim_ethernet_channel_release(handle)
}
