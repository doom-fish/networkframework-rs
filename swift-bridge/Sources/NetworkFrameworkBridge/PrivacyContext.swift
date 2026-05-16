import Foundation
import NetworkFrameworkCShim

@_cdecl("nfw_privacy_context_create")
public func nfwPrivacyContextCreate(_ description: UnsafePointer<CChar>?) -> UnsafeMutableRawPointer? {
    nw_shim_privacy_context_create(description)
}

@_cdecl("nfw_privacy_context_copy_default")
public func nfwPrivacyContextCopyDefault() -> UnsafeMutableRawPointer? {
    nw_shim_privacy_context_copy_default()
}

@_cdecl("nfw_privacy_context_flush_cache")
public func nfwPrivacyContextFlushCache(_ privacyContext: UnsafeMutableRawPointer?) {
    nw_shim_privacy_context_flush_cache(privacyContext)
}

@_cdecl("nfw_privacy_context_disable_logging")
public func nfwPrivacyContextDisableLogging(_ privacyContext: UnsafeMutableRawPointer?) {
    nw_shim_privacy_context_disable_logging(privacyContext)
}

@_cdecl("nfw_privacy_context_require_encrypted_name_resolution")
public func nfwPrivacyContextRequireEncryptedNameResolution(
    _ privacyContext: UnsafeMutableRawPointer?,
    _ requireEncryptedNameResolution: Int32,
    _ fallbackResolverConfig: UnsafeMutableRawPointer?
) {
    nw_shim_privacy_context_require_encrypted_name_resolution(privacyContext, requireEncryptedNameResolution, fallbackResolverConfig)
}

@_cdecl("nfw_privacy_context_add_proxy")
public func nfwPrivacyContextAddProxy(_ privacyContext: UnsafeMutableRawPointer?, _ proxyConfig: UnsafeMutableRawPointer?) {
    nw_shim_privacy_context_add_proxy(privacyContext, proxyConfig)
}

@_cdecl("nfw_privacy_context_clear_proxies")
public func nfwPrivacyContextClearProxies(_ privacyContext: UnsafeMutableRawPointer?) {
    nw_shim_privacy_context_clear_proxies(privacyContext)
}
