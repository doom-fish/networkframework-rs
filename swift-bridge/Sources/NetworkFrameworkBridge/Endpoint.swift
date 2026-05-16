import Foundation
import NetworkFrameworkCShim

@_cdecl("nfw_endpoint_create_host")
public func nfwEndpointCreateHost(_ host: UnsafePointer<CChar>?, _ port: UInt16) -> UnsafeMutableRawPointer? {
    nw_shim_endpoint_create_host(host, port)
}

@_cdecl("nfw_endpoint_create_address")
public func nfwEndpointCreateAddress(_ address: UnsafePointer<CChar>?, _ port: UInt16) -> UnsafeMutableRawPointer? {
    nw_shim_endpoint_create_address(address, port)
}

@_cdecl("nfw_endpoint_create_bonjour_service")
public func nfwEndpointCreateBonjourService(
    _ name: UnsafePointer<CChar>?,
    _ type: UnsafePointer<CChar>?,
    _ domain: UnsafePointer<CChar>?
) -> UnsafeMutableRawPointer? {
    nw_shim_endpoint_create_bonjour_service(name, type, domain)
}

@_cdecl("nfw_endpoint_create_url")
public func nfwEndpointCreateURL(_ url: UnsafePointer<CChar>?) -> UnsafeMutableRawPointer? {
    nw_shim_endpoint_create_url(url)
}

@_cdecl("nfw_endpoint_get_type")
public func nfwEndpointGetType(_ endpoint: UnsafeMutableRawPointer?) -> Int32 {
    nw_shim_endpoint_get_type(endpoint)
}

@_cdecl("nfw_endpoint_copy_hostname")
public func nfwEndpointCopyHostname(_ endpoint: UnsafeMutableRawPointer?) -> UnsafeMutablePointer<CChar>? {
    nw_shim_endpoint_copy_hostname(endpoint)
}

@_cdecl("nfw_endpoint_copy_port_string")
public func nfwEndpointCopyPortString(_ endpoint: UnsafeMutableRawPointer?) -> UnsafeMutablePointer<CChar>? {
    nw_shim_endpoint_copy_port_string(endpoint)
}

@_cdecl("nfw_endpoint_get_port")
public func nfwEndpointGetPort(_ endpoint: UnsafeMutableRawPointer?) -> UInt16 {
    nw_shim_endpoint_get_port(endpoint)
}

@_cdecl("nfw_endpoint_copy_address_string")
public func nfwEndpointCopyAddressString(_ endpoint: UnsafeMutableRawPointer?) -> UnsafeMutablePointer<CChar>? {
    nw_shim_endpoint_copy_address_string(endpoint)
}

@_cdecl("nfw_endpoint_copy_bonjour_service_name")
public func nfwEndpointCopyBonjourServiceName(_ endpoint: UnsafeMutableRawPointer?) -> UnsafeMutablePointer<CChar>? {
    nw_shim_endpoint_copy_bonjour_service_name(endpoint)
}

@_cdecl("nfw_endpoint_copy_bonjour_service_type")
public func nfwEndpointCopyBonjourServiceType(_ endpoint: UnsafeMutableRawPointer?) -> UnsafeMutablePointer<CChar>? {
    nw_shim_endpoint_copy_bonjour_service_type(endpoint)
}

@_cdecl("nfw_endpoint_copy_bonjour_service_domain")
public func nfwEndpointCopyBonjourServiceDomain(_ endpoint: UnsafeMutableRawPointer?) -> UnsafeMutablePointer<CChar>? {
    nw_shim_endpoint_copy_bonjour_service_domain(endpoint)
}

@_cdecl("nfw_endpoint_copy_url")
public func nfwEndpointCopyURL(_ endpoint: UnsafeMutableRawPointer?) -> UnsafeMutablePointer<CChar>? {
    nw_shim_endpoint_copy_url(endpoint)
}

@_cdecl("nfw_endpoint_copy_signature")
public func nfwEndpointCopySignature(
    _ endpoint: UnsafeMutableRawPointer?,
    _ outSignatureLength: UnsafeMutablePointer<Int>?
) -> UnsafeMutablePointer<UInt8>? {
    nw_shim_endpoint_copy_signature(endpoint, outSignatureLength)
}
