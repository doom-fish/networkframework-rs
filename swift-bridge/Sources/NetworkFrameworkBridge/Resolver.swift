import Foundation
import NetworkFrameworkCShim

@_cdecl("nfw_resolver_config_create_https")
public func nfwResolverConfigCreateHTTPS(_ url: UnsafePointer<CChar>?) -> UnsafeMutableRawPointer? {
    nw_shim_resolver_config_create_https(url)
}

@_cdecl("nfw_resolver_config_create_tls")
public func nfwResolverConfigCreateTLS(_ host: UnsafePointer<CChar>?, _ port: UInt16) -> UnsafeMutableRawPointer? {
    nw_shim_resolver_config_create_tls(host, port)
}

@_cdecl("nfw_resolver_config_add_server_address")
public func nfwResolverConfigAddServerAddress(
    _ resolverConfig: UnsafeMutableRawPointer?,
    _ address: UnsafePointer<CChar>?,
    _ port: UInt16
) -> Int32 {
    nw_shim_resolver_config_add_server_address(resolverConfig, address, port)
}
