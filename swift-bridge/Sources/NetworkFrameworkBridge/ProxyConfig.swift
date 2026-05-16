import Foundation
import NetworkFrameworkCShim

@_cdecl("nfw_proxy_config_create_http_connect")
public func nfwProxyConfigCreateHTTPConnect(_ host: UnsafePointer<CChar>?, _ port: UInt16, _ useTLS: Int32) -> UnsafeMutableRawPointer? {
    nw_shim_proxy_config_create_http_connect(host, port, useTLS)
}

@_cdecl("nfw_proxy_config_create_socksv5")
public func nfwProxyConfigCreateSOCKSv5(_ host: UnsafePointer<CChar>?, _ port: UInt16) -> UnsafeMutableRawPointer? {
    nw_shim_proxy_config_create_socksv5(host, port)
}

@_cdecl("nfw_relay_hop_create")
public func nfwRelayHopCreate(
    _ http3Endpoint: UnsafeMutableRawPointer?,
    _ http2Endpoint: UnsafeMutableRawPointer?,
    _ relayTLSOptions: UnsafeMutableRawPointer?
) -> UnsafeMutableRawPointer? {
    nw_shim_relay_hop_create(http3Endpoint, http2Endpoint, relayTLSOptions)
}

@_cdecl("nfw_relay_hop_add_additional_http_header_field")
public func nfwRelayHopAddAdditionalHTTPHeaderField(
    _ relayHop: UnsafeMutableRawPointer?,
    _ fieldName: UnsafePointer<CChar>?,
    _ fieldValue: UnsafePointer<CChar>?
) {
    nw_shim_relay_hop_add_additional_http_header_field(relayHop, fieldName, fieldValue)
}

@_cdecl("nfw_proxy_config_create_relay")
public func nfwProxyConfigCreateRelay(_ firstHop: UnsafeMutableRawPointer?, _ secondHop: UnsafeMutableRawPointer?) -> UnsafeMutableRawPointer? {
    nw_shim_proxy_config_create_relay(firstHop, secondHop)
}

@_cdecl("nfw_proxy_config_create_oblivious_http")
public func nfwProxyConfigCreateObliviousHTTP(
    _ relayHop: UnsafeMutableRawPointer?,
    _ relayResourcePath: UnsafePointer<CChar>?,
    _ gatewayKeyConfig: UnsafePointer<UInt8>?,
    _ gatewayKeyConfigLength: Int
) -> UnsafeMutableRawPointer? {
    nw_shim_proxy_config_create_oblivious_http(relayHop, relayResourcePath, gatewayKeyConfig, gatewayKeyConfigLength)
}

@_cdecl("nfw_proxy_config_set_username_password")
public func nfwProxyConfigSetUsernamePassword(
    _ proxyConfig: UnsafeMutableRawPointer?,
    _ username: UnsafePointer<CChar>?,
    _ password: UnsafePointer<CChar>?
) {
    nw_shim_proxy_config_set_username_password(proxyConfig, username, password)
}

@_cdecl("nfw_proxy_config_set_failover_allowed")
public func nfwProxyConfigSetFailoverAllowed(_ proxyConfig: UnsafeMutableRawPointer?, _ failoverAllowed: Int32) {
    nw_shim_proxy_config_set_failover_allowed(proxyConfig, failoverAllowed)
}

@_cdecl("nfw_proxy_config_get_failover_allowed")
public func nfwProxyConfigGetFailoverAllowed(_ proxyConfig: UnsafeMutableRawPointer?) -> Int32 {
    nw_shim_proxy_config_get_failover_allowed(proxyConfig)
}

@_cdecl("nfw_proxy_config_add_match_domain")
public func nfwProxyConfigAddMatchDomain(_ proxyConfig: UnsafeMutableRawPointer?, _ domain: UnsafePointer<CChar>?) {
    nw_shim_proxy_config_add_match_domain(proxyConfig, domain)
}

@_cdecl("nfw_proxy_config_clear_match_domains")
public func nfwProxyConfigClearMatchDomains(_ proxyConfig: UnsafeMutableRawPointer?) { nw_shim_proxy_config_clear_match_domains(proxyConfig) }

@_cdecl("nfw_proxy_config_add_excluded_domain")
public func nfwProxyConfigAddExcludedDomain(_ proxyConfig: UnsafeMutableRawPointer?, _ domain: UnsafePointer<CChar>?) {
    nw_shim_proxy_config_add_excluded_domain(proxyConfig, domain)
}

@_cdecl("nfw_proxy_config_clear_excluded_domains")
public func nfwProxyConfigClearExcludedDomains(_ proxyConfig: UnsafeMutableRawPointer?) { nw_shim_proxy_config_clear_excluded_domains(proxyConfig) }

@_cdecl("nfw_proxy_config_enumerate_match_domains")
public func nfwProxyConfigEnumerateMatchDomains(
    _ proxyConfig: UnsafeMutableRawPointer?,
    _ callback: StringEnumerationCallback?,
    _ userInfo: UnsafeMutableRawPointer?
) {
    nw_shim_proxy_config_enumerate_match_domains(proxyConfig, callback, userInfo)
}

@_cdecl("nfw_proxy_config_enumerate_excluded_domains")
public func nfwProxyConfigEnumerateExcludedDomains(
    _ proxyConfig: UnsafeMutableRawPointer?,
    _ callback: StringEnumerationCallback?,
    _ userInfo: UnsafeMutableRawPointer?
) {
    nw_shim_proxy_config_enumerate_excluded_domains(proxyConfig, callback, userInfo)
}
