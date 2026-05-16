import Foundation
import NetworkFrameworkCShim

@_cdecl("nfw_protocol_copy_tcp_definition")
public func nfwProtocolCopyTCPDefinition() -> UnsafeMutableRawPointer? { nw_shim_protocol_copy_tcp_definition() }

@_cdecl("nfw_protocol_copy_udp_definition")
public func nfwProtocolCopyUDPDefinition() -> UnsafeMutableRawPointer? { nw_shim_protocol_copy_udp_definition() }

@_cdecl("nfw_protocol_copy_tls_definition")
public func nfwProtocolCopyTLSDefinition() -> UnsafeMutableRawPointer? { nw_shim_protocol_copy_tls_definition() }

@_cdecl("nfw_protocol_copy_ip_definition")
public func nfwProtocolCopyIPDefinition() -> UnsafeMutableRawPointer? { nw_shim_protocol_copy_ip_definition() }

@_cdecl("nfw_protocol_copy_ws_definition")
public func nfwProtocolCopyWSDefinition() -> UnsafeMutableRawPointer? { nw_shim_protocol_copy_ws_definition() }

@_cdecl("nfw_protocol_copy_quic_definition")
public func nfwProtocolCopyQUICDefinition() -> UnsafeMutableRawPointer? { nw_shim_protocol_copy_quic_definition() }

@_cdecl("nfw_protocol_create_tcp_options")
public func nfwProtocolCreateTCPOptions() -> UnsafeMutableRawPointer? { nw_shim_protocol_create_tcp_options() }

@_cdecl("nfw_protocol_create_udp_options")
public func nfwProtocolCreateUDPOptions() -> UnsafeMutableRawPointer? { nw_shim_protocol_create_udp_options() }

@_cdecl("nfw_protocol_create_tls_options")
public func nfwProtocolCreateTLSOptions() -> UnsafeMutableRawPointer? { nw_shim_protocol_create_tls_options() }

@_cdecl("nfw_protocol_create_ip_options")
public func nfwProtocolCreateIPOptions() -> UnsafeMutableRawPointer? { nw_shim_protocol_create_ip_options() }

@_cdecl("nfw_protocol_create_ws_options")
public func nfwProtocolCreateWSOptions() -> UnsafeMutableRawPointer? { nw_shim_protocol_create_ws_options() }

@_cdecl("nfw_protocol_create_quic_options")
public func nfwProtocolCreateQUICOptions() -> UnsafeMutableRawPointer? { nw_shim_protocol_create_quic_options() }

@_cdecl("nfw_protocol_definition_is_equal")
public func nfwProtocolDefinitionIsEqual(_ definition1: UnsafeMutableRawPointer?, _ definition2: UnsafeMutableRawPointer?) -> Int32 {
    nw_shim_protocol_definition_is_equal(definition1, definition2)
}

@_cdecl("nfw_protocol_options_copy_definition")
public func nfwProtocolOptionsCopyDefinition(_ options: UnsafeMutableRawPointer?) -> UnsafeMutableRawPointer? {
    nw_shim_protocol_options_copy_definition(options)
}

@_cdecl("nfw_protocol_metadata_copy_definition")
public func nfwProtocolMetadataCopyDefinition(_ metadata: UnsafeMutableRawPointer?) -> UnsafeMutableRawPointer? {
    nw_shim_protocol_metadata_copy_definition(metadata)
}

@_cdecl("nfw_protocol_options_is_quic")
public func nfwProtocolOptionsIsQUIC(_ options: UnsafeMutableRawPointer?) -> Int32 {
    nw_shim_protocol_options_is_quic(options)
}
