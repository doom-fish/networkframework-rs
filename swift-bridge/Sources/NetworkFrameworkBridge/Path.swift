import Foundation
import NetworkFrameworkCShim

@_cdecl("nfw_path_monitor_start")
public func nfwPathMonitorStart(_ callback: PathMonitorCallback?, _ userInfo: UnsafeMutableRawPointer?) -> UnsafeMutableRawPointer? {
    nw_shim_path_monitor_start(callback, userInfo)
}

@_cdecl("nfw_path_monitor_stop")
public func nfwPathMonitorStop(_ handle: UnsafeMutableRawPointer?) {
    nw_shim_path_monitor_stop(handle)
}

@_cdecl("nfw_path_monitor_copy_latest_path")
public func nfwPathMonitorCopyLatestPath(_ handle: UnsafeMutableRawPointer?) -> UnsafeMutableRawPointer? {
    nw_shim_path_monitor_copy_latest_path(handle)
}

@_cdecl("nfw_path_monitor_enumerate_interfaces")
public func nfwPathMonitorEnumerateInterfaces(
    _ handle: UnsafeMutableRawPointer?,
    _ callback: InterfaceEnumerationCallback?,
    _ userInfo: UnsafeMutableRawPointer?
) -> Int32 {
    nw_shim_path_monitor_enumerate_interfaces(handle, callback, userInfo)
}

@_cdecl("nfw_list_interfaces")
public func nfwListInterfaces(_ callback: InterfaceEnumerationCallback?, _ userInfo: UnsafeMutableRawPointer?) -> Int32 {
    nw_shim_list_interfaces(callback, userInfo)
}

@_cdecl("nfw_path_get_status")
public func nfwPathGetStatus(_ path: UnsafeMutableRawPointer?) -> Int32 { nw_shim_path_get_status(path) }

@_cdecl("nfw_path_get_unsatisfied_reason")
public func nfwPathGetUnsatisfiedReason(_ path: UnsafeMutableRawPointer?) -> Int32 { nw_shim_path_get_unsatisfied_reason(path) }

@_cdecl("nfw_path_is_equal")
public func nfwPathIsEqual(_ path: UnsafeMutableRawPointer?, _ otherPath: UnsafeMutableRawPointer?) -> Int32 {
    nw_shim_path_is_equal(path, otherPath)
}

@_cdecl("nfw_path_is_expensive")
public func nfwPathIsExpensive(_ path: UnsafeMutableRawPointer?) -> Int32 { nw_shim_path_is_expensive(path) }

@_cdecl("nfw_path_is_constrained")
public func nfwPathIsConstrained(_ path: UnsafeMutableRawPointer?) -> Int32 { nw_shim_path_is_constrained(path) }

@_cdecl("nfw_path_is_ultra_constrained")
public func nfwPathIsUltraConstrained(_ path: UnsafeMutableRawPointer?) -> Int32 { nw_shim_path_is_ultra_constrained(path) }

@_cdecl("nfw_path_has_ipv4")
public func nfwPathHasIPv4(_ path: UnsafeMutableRawPointer?) -> Int32 { nw_shim_path_has_ipv4(path) }

@_cdecl("nfw_path_has_ipv6")
public func nfwPathHasIPv6(_ path: UnsafeMutableRawPointer?) -> Int32 { nw_shim_path_has_ipv6(path) }

@_cdecl("nfw_path_has_dns")
public func nfwPathHasDNS(_ path: UnsafeMutableRawPointer?) -> Int32 { nw_shim_path_has_dns(path) }

@_cdecl("nfw_path_uses_interface_type")
public func nfwPathUsesInterfaceType(_ path: UnsafeMutableRawPointer?, _ interfaceType: Int32) -> Int32 {
    nw_shim_path_uses_interface_type(path, interfaceType)
}

@_cdecl("nfw_path_copy_effective_local_endpoint")
public func nfwPathCopyEffectiveLocalEndpoint(_ path: UnsafeMutableRawPointer?) -> UnsafeMutableRawPointer? {
    nw_shim_path_copy_effective_local_endpoint(path)
}

@_cdecl("nfw_path_copy_effective_remote_endpoint")
public func nfwPathCopyEffectiveRemoteEndpoint(_ path: UnsafeMutableRawPointer?) -> UnsafeMutableRawPointer? {
    nw_shim_path_copy_effective_remote_endpoint(path)
}

@_cdecl("nfw_path_get_link_quality")
public func nfwPathGetLinkQuality(_ path: UnsafeMutableRawPointer?) -> Int32 { nw_shim_path_get_link_quality(path) }

@_cdecl("nfw_path_enumerate_interfaces")
public func nfwPathEnumerateInterfaces(
    _ path: UnsafeMutableRawPointer?,
    _ callback: InterfaceEnumerationCallback?,
    _ userInfo: UnsafeMutableRawPointer?
) -> Int32 {
    nw_shim_path_enumerate_interfaces(path, callback, userInfo)
}
