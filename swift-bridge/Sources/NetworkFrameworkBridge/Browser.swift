import Foundation
import NetworkFrameworkCShim

@_cdecl("nfw_browser_start")
public func nfwBrowserStart(
    _ serviceType: UnsafePointer<CChar>?,
    _ domain: UnsafePointer<CChar>?,
    _ foundCallback: BrowserServiceCallback?,
    _ lostCallback: BrowserServiceCallback?,
    _ userInfo: UnsafeMutableRawPointer?
) -> UnsafeMutableRawPointer? {
    nw_shim_browser_start(serviceType, domain, foundCallback, lostCallback, userInfo)
}

@_cdecl("nfw_browser_start_with_descriptor")
public func nfwBrowserStartWithDescriptor(
    _ descriptor: UnsafeMutableRawPointer?,
    _ parameters: UnsafeMutableRawPointer?,
    _ foundCallback: BrowserServiceCallback?,
    _ lostCallback: BrowserServiceCallback?,
    _ userInfo: UnsafeMutableRawPointer?
) -> UnsafeMutableRawPointer? {
    nw_shim_browser_start_with_descriptor(descriptor, parameters, foundCallback, lostCallback, userInfo)
}

@_cdecl("nfw_browser_stop")
public func nfwBrowserStop(_ handle: UnsafeMutableRawPointer?) {
    nw_shim_browser_stop(handle)
}

@_cdecl("nfw_browse_descriptor_create_bonjour_service")
public func nfwBrowseDescriptorCreateBonjourService(
    _ type: UnsafePointer<CChar>?,
    _ domain: UnsafePointer<CChar>?
) -> UnsafeMutableRawPointer? {
    nw_shim_browse_descriptor_create_bonjour_service(type, domain)
}

@_cdecl("nfw_browse_descriptor_copy_bonjour_service_type")
public func nfwBrowseDescriptorCopyBonjourServiceType(_ descriptor: UnsafeMutableRawPointer?) -> UnsafeMutablePointer<CChar>? {
    nw_shim_browse_descriptor_copy_bonjour_service_type(descriptor)
}

@_cdecl("nfw_browse_descriptor_copy_bonjour_service_domain")
public func nfwBrowseDescriptorCopyBonjourServiceDomain(_ descriptor: UnsafeMutableRawPointer?) -> UnsafeMutablePointer<CChar>? {
    nw_shim_browse_descriptor_copy_bonjour_service_domain(descriptor)
}

@_cdecl("nfw_browse_descriptor_set_include_txt_record")
public func nfwBrowseDescriptorSetIncludeTxtRecord(_ descriptor: UnsafeMutableRawPointer?, _ includeTxtRecord: Int32) {
    nw_shim_browse_descriptor_set_include_txt_record(descriptor, includeTxtRecord)
}

@_cdecl("nfw_browse_descriptor_get_include_txt_record")
public func nfwBrowseDescriptorGetIncludeTxtRecord(_ descriptor: UnsafeMutableRawPointer?) -> Int32 {
    nw_shim_browse_descriptor_get_include_txt_record(descriptor)
}

@_cdecl("nfw_browse_descriptor_create_application_service")
public func nfwBrowseDescriptorCreateApplicationService(_ applicationServiceName: UnsafePointer<CChar>?) -> UnsafeMutableRawPointer? {
    nw_shim_browse_descriptor_create_application_service(applicationServiceName)
}

@_cdecl("nfw_browse_descriptor_copy_application_service_name")
public func nfwBrowseDescriptorCopyApplicationServiceName(_ descriptor: UnsafeMutableRawPointer?) -> UnsafeMutablePointer<CChar>? {
    nw_shim_browse_descriptor_copy_application_service_name(descriptor)
}
