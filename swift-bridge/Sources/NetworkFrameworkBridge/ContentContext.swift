import Foundation
import NetworkFrameworkCShim

@_cdecl("nfw_content_context_create")
public func nfwContentContextCreate(_ identifier: UnsafePointer<CChar>?) -> UnsafeMutableRawPointer? {
    nw_shim_content_context_create(identifier)
}

@_cdecl("nfw_content_context_get_identifier")
public func nfwContentContextGetIdentifier(_ context: UnsafeMutableRawPointer?) -> UnsafePointer<CChar>? {
    nw_shim_content_context_get_identifier(context)
}

@_cdecl("nfw_content_context_get_is_final")
public func nfwContentContextGetIsFinal(_ context: UnsafeMutableRawPointer?) -> Int32 { nw_shim_content_context_get_is_final(context) }

@_cdecl("nfw_content_context_set_is_final")
public func nfwContentContextSetIsFinal(_ context: UnsafeMutableRawPointer?, _ isFinal: Int32) { nw_shim_content_context_set_is_final(context, isFinal) }

@_cdecl("nfw_content_context_get_expiration_milliseconds")
public func nfwContentContextGetExpirationMilliseconds(_ context: UnsafeMutableRawPointer?) -> UInt64 {
    nw_shim_content_context_get_expiration_milliseconds(context)
}

@_cdecl("nfw_content_context_set_expiration_milliseconds")
public func nfwContentContextSetExpirationMilliseconds(_ context: UnsafeMutableRawPointer?, _ expirationMilliseconds: UInt64) {
    nw_shim_content_context_set_expiration_milliseconds(context, expirationMilliseconds)
}

@_cdecl("nfw_content_context_get_relative_priority")
public func nfwContentContextGetRelativePriority(_ context: UnsafeMutableRawPointer?) -> Double {
    nw_shim_content_context_get_relative_priority(context)
}

@_cdecl("nfw_content_context_set_relative_priority")
public func nfwContentContextSetRelativePriority(_ context: UnsafeMutableRawPointer?, _ relativePriority: Double) {
    nw_shim_content_context_set_relative_priority(context, relativePriority)
}

@_cdecl("nfw_content_context_set_antecedent")
public func nfwContentContextSetAntecedent(_ context: UnsafeMutableRawPointer?, _ antecedent: UnsafeMutableRawPointer?) {
    nw_shim_content_context_set_antecedent(context, antecedent)
}

@_cdecl("nfw_content_context_copy_antecedent")
public func nfwContentContextCopyAntecedent(_ context: UnsafeMutableRawPointer?) -> UnsafeMutableRawPointer? {
    nw_shim_content_context_copy_antecedent(context)
}

@_cdecl("nfw_content_context_set_protocol_metadata")
public func nfwContentContextSetProtocolMetadata(_ context: UnsafeMutableRawPointer?, _ metadata: UnsafeMutableRawPointer?) {
    nw_shim_content_context_set_protocol_metadata(context, metadata)
}

@_cdecl("nfw_content_context_copy_protocol_metadata_for_options")
public func nfwContentContextCopyProtocolMetadataForOptions(_ context: UnsafeMutableRawPointer?, _ protocolOptions: UnsafeMutableRawPointer?) -> UnsafeMutableRawPointer? {
    nw_shim_content_context_copy_protocol_metadata_for_options(context, protocolOptions)
}
