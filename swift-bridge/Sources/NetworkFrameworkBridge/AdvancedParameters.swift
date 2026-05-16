import Foundation
import NetworkFrameworkCShim

@_cdecl("nfw_interface_copy_name")
public func nfwInterfaceCopyName(_ interface: UnsafeMutableRawPointer?) -> UnsafeMutablePointer<CChar>? {
    nw_shim_interface_copy_name(interface)
}

@_cdecl("nfw_interface_get_type")
public func nfwInterfaceGetType(_ interface: UnsafeMutableRawPointer?) -> Int32 {
    nw_shim_interface_get_type(interface)
}

@_cdecl("nfw_interface_get_index")
public func nfwInterfaceGetIndex(_ interface: UnsafeMutableRawPointer?) -> UInt32 {
    nw_shim_interface_get_index(interface)
}

@_cdecl("nfw_parameters_require_interface")
public func nfwParametersRequireInterface(
    _ parameters: UnsafeMutableRawPointer?,
    _ name: UnsafePointer<CChar>?,
    _ interfaceType: Int32,
    _ index: UInt32
) {
    nw_shim_parameters_require_interface(parameters, name, interfaceType, index)
}

@_cdecl("nfw_parameters_copy_required_interface")
public func nfwParametersCopyRequiredInterface(
    _ parameters: UnsafeMutableRawPointer?,
    _ outName: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?,
    _ outType: UnsafeMutablePointer<Int32>?,
    _ outIndex: UnsafeMutablePointer<UInt32>?
) -> Int32 {
    nw_shim_parameters_copy_required_interface(parameters, outName, outType, outIndex)
}

@_cdecl("nfw_parameters_prohibit_interface")
public func nfwParametersProhibitInterface(
    _ parameters: UnsafeMutableRawPointer?,
    _ name: UnsafePointer<CChar>?,
    _ interfaceType: Int32,
    _ index: UInt32
) {
    nw_shim_parameters_prohibit_interface(parameters, name, interfaceType, index)
}

@_cdecl("nfw_parameters_clear_prohibited_interfaces")
public func nfwParametersClearProhibitedInterfaces(_ parameters: UnsafeMutableRawPointer?) {
    nw_shim_parameters_clear_prohibited_interfaces(parameters)
}

@_cdecl("nfw_parameters_copy_prohibited_interfaces")
public func nfwParametersCopyProhibitedInterfaces(
    _ parameters: UnsafeMutableRawPointer?,
    _ outCount: UnsafeMutablePointer<Int>?
) -> UnsafeMutablePointer<UnsafeMutableRawPointer?>? {
    nw_shim_parameters_copy_prohibited_interfaces(parameters, outCount)
}

@_cdecl("nfw_parameters_prohibit_interface_type")
public func nfwParametersProhibitInterfaceType(_ parameters: UnsafeMutableRawPointer?, _ interfaceType: Int32) {
    nw_shim_parameters_prohibit_interface_type(parameters, interfaceType)
}

@_cdecl("nfw_parameters_clear_prohibited_interface_types")
public func nfwParametersClearProhibitedInterfaceTypes(_ parameters: UnsafeMutableRawPointer?) {
    nw_shim_parameters_clear_prohibited_interface_types(parameters)
}

@_cdecl("nfw_parameters_copy_prohibited_interface_types")
public func nfwParametersCopyProhibitedInterfaceTypes(
    _ parameters: UnsafeMutableRawPointer?,
    _ outCount: UnsafeMutablePointer<Int>?
) -> UnsafeMutablePointer<Int32>? {
    nw_shim_parameters_copy_prohibited_interface_types(parameters, outCount)
}

@_cdecl("nfw_parameters_set_reuse_local_address")
public func nfwParametersSetReuseLocalAddress(_ parameters: UnsafeMutableRawPointer?, _ reuseLocalAddress: Int32) {
    nw_shim_parameters_set_reuse_local_address(parameters, reuseLocalAddress)
}

@_cdecl("nfw_parameters_get_reuse_local_address")
public func nfwParametersGetReuseLocalAddress(_ parameters: UnsafeMutableRawPointer?) -> Int32 {
    nw_shim_parameters_get_reuse_local_address(parameters)
}

@_cdecl("nfw_parameters_set_local_endpoint")
public func nfwParametersSetLocalEndpoint(_ parameters: UnsafeMutableRawPointer?, _ localEndpoint: UnsafeMutableRawPointer?) {
    nw_shim_parameters_set_local_endpoint(parameters, localEndpoint)
}

@_cdecl("nfw_parameters_copy_local_endpoint")
public func nfwParametersCopyLocalEndpoint(_ parameters: UnsafeMutableRawPointer?) -> UnsafeMutableRawPointer? {
    nw_shim_parameters_copy_local_endpoint(parameters)
}

@_cdecl("nfw_parameters_set_include_peer_to_peer")
public func nfwParametersSetIncludePeerToPeer(_ parameters: UnsafeMutableRawPointer?, _ includePeerToPeer: Int32) {
    nw_shim_parameters_set_include_peer_to_peer(parameters, includePeerToPeer)
}

@_cdecl("nfw_parameters_get_include_peer_to_peer")
public func nfwParametersGetIncludePeerToPeer(_ parameters: UnsafeMutableRawPointer?) -> Int32 {
    nw_shim_parameters_get_include_peer_to_peer(parameters)
}

@_cdecl("nfw_parameters_set_fast_open_enabled")
public func nfwParametersSetFastOpenEnabled(_ parameters: UnsafeMutableRawPointer?, _ fastOpenEnabled: Int32) {
    nw_shim_parameters_set_fast_open_enabled(parameters, fastOpenEnabled)
}

@_cdecl("nfw_parameters_get_fast_open_enabled")
public func nfwParametersGetFastOpenEnabled(_ parameters: UnsafeMutableRawPointer?) -> Int32 {
    nw_shim_parameters_get_fast_open_enabled(parameters)
}

@_cdecl("nfw_parameters_set_service_class")
public func nfwParametersSetServiceClass(_ parameters: UnsafeMutableRawPointer?, _ serviceClass: Int32) {
    nw_shim_parameters_set_service_class(parameters, serviceClass)
}

@_cdecl("nfw_parameters_get_service_class")
public func nfwParametersGetServiceClass(_ parameters: UnsafeMutableRawPointer?) -> Int32 {
    nw_shim_parameters_get_service_class(parameters)
}

@_cdecl("nfw_parameters_set_multipath_service")
public func nfwParametersSetMultipathService(_ parameters: UnsafeMutableRawPointer?, _ multipathService: Int32) {
    nw_shim_parameters_set_multipath_service(parameters, multipathService)
}

@_cdecl("nfw_parameters_get_multipath_service")
public func nfwParametersGetMultipathService(_ parameters: UnsafeMutableRawPointer?) -> Int32 {
    nw_shim_parameters_get_multipath_service(parameters)
}

@_cdecl("nfw_parameters_copy_default_protocol_stack")
public func nfwParametersCopyDefaultProtocolStack(_ parameters: UnsafeMutableRawPointer?) -> UnsafeMutableRawPointer? {
    nw_shim_parameters_copy_default_protocol_stack(parameters)
}

@_cdecl("nfw_protocol_stack_clear_application_protocols")
public func nfwProtocolStackClearApplicationProtocols(_ stack: UnsafeMutableRawPointer?) {
    nw_shim_protocol_stack_clear_application_protocols(stack)
}

@_cdecl("nfw_protocol_stack_copy_application_protocols")
public func nfwProtocolStackCopyApplicationProtocols(
    _ stack: UnsafeMutableRawPointer?,
    _ outCount: UnsafeMutablePointer<Int>?
) -> UnsafeMutablePointer<UnsafeMutableRawPointer?>? {
    nw_shim_protocol_stack_copy_application_protocols(stack, outCount)
}

@_cdecl("nfw_protocol_stack_copy_transport_protocol")
public func nfwProtocolStackCopyTransportProtocol(_ stack: UnsafeMutableRawPointer?) -> UnsafeMutableRawPointer? {
    nw_shim_protocol_stack_copy_transport_protocol(stack)
}

@_cdecl("nfw_protocol_stack_set_transport_protocol")
public func nfwProtocolStackSetTransportProtocol(_ stack: UnsafeMutableRawPointer?, _ protocolOptions: UnsafeMutableRawPointer?) {
    nw_shim_protocol_stack_set_transport_protocol(stack, protocolOptions)
}

@_cdecl("nfw_protocol_stack_copy_internet_protocol")
public func nfwProtocolStackCopyInternetProtocol(_ stack: UnsafeMutableRawPointer?) -> UnsafeMutableRawPointer? {
    nw_shim_protocol_stack_copy_internet_protocol(stack)
}

@_cdecl("nfw_parameters_set_local_only")
public func nfwParametersSetLocalOnly(_ parameters: UnsafeMutableRawPointer?, _ localOnly: Int32) {
    nw_shim_parameters_set_local_only(parameters, localOnly)
}

@_cdecl("nfw_parameters_get_local_only")
public func nfwParametersGetLocalOnly(_ parameters: UnsafeMutableRawPointer?) -> Int32 {
    nw_shim_parameters_get_local_only(parameters)
}

@_cdecl("nfw_parameters_get_prefer_no_proxy")
public func nfwParametersGetPreferNoProxy(_ parameters: UnsafeMutableRawPointer?) -> Int32 {
    nw_shim_parameters_get_prefer_no_proxy(parameters)
}

@_cdecl("nfw_parameters_set_expired_dns_behavior")
public func nfwParametersSetExpiredDnsBehavior(_ parameters: UnsafeMutableRawPointer?, _ expiredDnsBehavior: Int32) {
    nw_shim_parameters_set_expired_dns_behavior(parameters, expiredDnsBehavior)
}

@_cdecl("nfw_parameters_get_expired_dns_behavior")
public func nfwParametersGetExpiredDnsBehavior(_ parameters: UnsafeMutableRawPointer?) -> Int32 {
    nw_shim_parameters_get_expired_dns_behavior(parameters)
}

@_cdecl("nfw_parameters_set_requires_dnssec_validation")
public func nfwParametersSetRequiresDNSSECValidation(_ parameters: UnsafeMutableRawPointer?, _ requiresDNSSECValidation: Int32) {
    nw_shim_parameters_set_requires_dnssec_validation(parameters, requiresDNSSECValidation)
}

@_cdecl("nfw_parameters_requires_dnssec_validation")
public func nfwParametersRequiresDNSSECValidation(_ parameters: UnsafeMutableRawPointer?) -> Int32 {
    nw_shim_parameters_requires_dnssec_validation(parameters)
}
