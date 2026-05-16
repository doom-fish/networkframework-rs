import Foundation
import NetworkFrameworkCShim

@_cdecl("nfw_parameters_create")
public func nfwParametersCreate() -> UnsafeMutableRawPointer? {
    nw_shim_parameters_create()
}

@_cdecl("nfw_parameters_create_application_service")
public func nfwParametersCreateApplicationService() -> UnsafeMutableRawPointer? {
    nw_shim_parameters_create_application_service()
}

@_cdecl("nfw_parameters_create_tcp")
public func nfwParametersCreateTCP(_ useTLS: Int32) -> UnsafeMutableRawPointer? {
    nw_shim_parameters_create_tcp(useTLS)
}

@_cdecl("nfw_parameters_create_udp")
public func nfwParametersCreateUDP() -> UnsafeMutableRawPointer? {
    nw_shim_parameters_create_udp()
}

@_cdecl("nfw_parameters_create_quic")
public func nfwParametersCreateQUIC(_ alpn: UnsafePointer<CChar>?) -> UnsafeMutableRawPointer? {
    nw_shim_parameters_create_quic(alpn)
}

@_cdecl("nfw_parameters_copy")
public func nfwParametersCopy(_ handle: UnsafeMutableRawPointer?) -> UnsafeMutableRawPointer? {
    nw_shim_parameters_copy(handle)
}

@_cdecl("nfw_parameters_prepend_application_protocol")
public func nfwParametersPrependApplicationProtocol(
    _ parameters: UnsafeMutableRawPointer?,
    _ protocolOptions: UnsafeMutableRawPointer?
) -> Int32 {
    nw_shim_parameters_prepend_application_protocol(parameters, protocolOptions)
}

@_cdecl("nfw_parameters_set_privacy_context")
public func nfwParametersSetPrivacyContext(_ parameters: UnsafeMutableRawPointer?, _ privacyContext: UnsafeMutableRawPointer?) {
    nw_shim_parameters_set_privacy_context(parameters, privacyContext)
}

@_cdecl("nfw_parameters_set_prefer_no_proxy")
public func nfwParametersSetPreferNoProxy(_ parameters: UnsafeMutableRawPointer?, _ preferNoProxy: Int32) {
    nw_shim_parameters_set_prefer_no_proxy(parameters, preferNoProxy)
}

@_cdecl("nfw_parameters_set_attribution")
public func nfwParametersSetAttribution(_ parameters: UnsafeMutableRawPointer?, _ attribution: Int32) {
    nw_shim_parameters_set_attribution(parameters, attribution)
}

@_cdecl("nfw_parameters_get_attribution")
public func nfwParametersGetAttribution(_ parameters: UnsafeMutableRawPointer?) -> Int32 {
    nw_shim_parameters_get_attribution(parameters)
}

@_cdecl("nfw_parameters_set_required_interface_type")
public func nfwParametersSetRequiredInterfaceType(_ parameters: UnsafeMutableRawPointer?, _ interfaceType: Int32) {
    nw_shim_parameters_set_required_interface_type(parameters, interfaceType)
}

@_cdecl("nfw_parameters_get_required_interface_type")
public func nfwParametersGetRequiredInterfaceType(_ parameters: UnsafeMutableRawPointer?) -> Int32 {
    nw_shim_parameters_get_required_interface_type(parameters)
}

@_cdecl("nfw_parameters_set_prohibit_expensive")
public func nfwParametersSetProhibitExpensive(_ parameters: UnsafeMutableRawPointer?, _ prohibitExpensive: Int32) {
    nw_shim_parameters_set_prohibit_expensive(parameters, prohibitExpensive)
}

@_cdecl("nfw_parameters_get_prohibit_expensive")
public func nfwParametersGetProhibitExpensive(_ parameters: UnsafeMutableRawPointer?) -> Int32 {
    nw_shim_parameters_get_prohibit_expensive(parameters)
}

@_cdecl("nfw_parameters_set_prohibit_constrained")
public func nfwParametersSetProhibitConstrained(_ parameters: UnsafeMutableRawPointer?, _ prohibitConstrained: Int32) {
    nw_shim_parameters_set_prohibit_constrained(parameters, prohibitConstrained)
}

@_cdecl("nfw_parameters_get_prohibit_constrained")
public func nfwParametersGetProhibitConstrained(_ parameters: UnsafeMutableRawPointer?) -> Int32 {
    nw_shim_parameters_get_prohibit_constrained(parameters)
}

@_cdecl("nfw_parameters_set_allow_ultra_constrained")
public func nfwParametersSetAllowUltraConstrained(_ parameters: UnsafeMutableRawPointer?, _ allowUltraConstrained: Int32) {
    nw_shim_parameters_set_allow_ultra_constrained(parameters, allowUltraConstrained)
}

@_cdecl("nfw_parameters_get_allow_ultra_constrained")
public func nfwParametersGetAllowUltraConstrained(_ parameters: UnsafeMutableRawPointer?) -> Int32 {
    nw_shim_parameters_get_allow_ultra_constrained(parameters)
}
