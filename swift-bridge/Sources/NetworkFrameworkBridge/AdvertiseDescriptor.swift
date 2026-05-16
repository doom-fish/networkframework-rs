import Foundation
import NetworkFrameworkCShim

@_cdecl("nfw_advertise_descriptor_create_bonjour_service")
public func nfwAdvertiseDescriptorCreateBonjourService(
    _ name: UnsafePointer<CChar>?,
    _ type: UnsafePointer<CChar>?,
    _ domain: UnsafePointer<CChar>?
) -> UnsafeMutableRawPointer? {
    nw_shim_advertise_descriptor_create_bonjour_service(name, type, domain)
}

@_cdecl("nfw_advertise_descriptor_create_application_service")
public func nfwAdvertiseDescriptorCreateApplicationService(_ applicationServiceName: UnsafePointer<CChar>?) -> UnsafeMutableRawPointer? {
    nw_shim_advertise_descriptor_create_application_service(applicationServiceName)
}

@_cdecl("nfw_advertise_descriptor_set_txt_record")
public func nfwAdvertiseDescriptorSetTXTRecord(_ descriptor: UnsafeMutableRawPointer?, _ txtRecord: UnsafePointer<UInt8>?, _ txtLength: Int) {
    nw_shim_advertise_descriptor_set_txt_record(descriptor, txtRecord, txtLength)
}

@_cdecl("nfw_advertise_descriptor_set_no_auto_rename")
public func nfwAdvertiseDescriptorSetNoAutoRename(_ descriptor: UnsafeMutableRawPointer?, _ noAutoRename: Int32) {
    nw_shim_advertise_descriptor_set_no_auto_rename(descriptor, noAutoRename)
}

@_cdecl("nfw_advertise_descriptor_get_no_auto_rename")
public func nfwAdvertiseDescriptorGetNoAutoRename(_ descriptor: UnsafeMutableRawPointer?) -> Int32 {
    nw_shim_advertise_descriptor_get_no_auto_rename(descriptor)
}

@_cdecl("nfw_advertise_descriptor_copy_application_service_name")
public func nfwAdvertiseDescriptorCopyApplicationServiceName(_ descriptor: UnsafeMutableRawPointer?) -> UnsafeMutablePointer<CChar>? {
    nw_shim_advertise_descriptor_copy_application_service_name(descriptor)
}

@_cdecl("nfw_bonjour_advertise_start")
public func nfwBonjourAdvertiseStart(
    _ serviceType: UnsafePointer<CChar>?,
    _ serviceName: UnsafePointer<CChar>?,
    _ domain: UnsafePointer<CChar>?,
    _ port: UInt16,
    _ outStatus: UnsafeMutablePointer<Int32>?
) -> UnsafeMutableRawPointer? {
    nw_shim_bonjour_advertise_start(serviceType, serviceName, domain, port, outStatus)
}

@_cdecl("nfw_bonjour_advertise_start_with_descriptor")
public func nfwBonjourAdvertiseStartWithDescriptor(
    _ descriptor: UnsafeMutableRawPointer?,
    _ port: UInt16,
    _ outStatus: UnsafeMutablePointer<Int32>?
) -> UnsafeMutableRawPointer? {
    nw_shim_bonjour_advertise_start_with_descriptor(descriptor, port, outStatus)
}

@_cdecl("nfw_bonjour_advertise_stop")
public func nfwBonjourAdvertiseStop(_ handle: UnsafeMutableRawPointer?) {
    nw_shim_bonjour_advertise_stop(handle)
}
