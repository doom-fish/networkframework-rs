import Foundation
import NetworkFrameworkCShim

@_cdecl("nfw_endpoint_copy_address")
public func nfwEndpointCopyAddress(
    _ endpoint: UnsafeMutableRawPointer?,
    _ outBuffer: UnsafeMutableRawPointer?,
    _ outBufferLength: Int
) -> Int {
    nw_shim_endpoint_copy_address(endpoint, outBuffer, outBufferLength)
}

@_cdecl("nfw_endpoint_copy_txt_record")
public func nfwEndpointCopyTXTRecord(_ endpoint: UnsafeMutableRawPointer?) -> UnsafeMutableRawPointer? {
    nw_shim_endpoint_copy_txt_record(endpoint)
}

@_cdecl("nfw_txt_record_create_with_bytes")
public func nfwTXTRecordCreateWithBytes(_ txtBytes: UnsafePointer<UInt8>?, _ txtLength: Int) -> UnsafeMutableRawPointer? {
    nw_shim_txt_record_create_with_bytes(txtBytes, txtLength)
}

@_cdecl("nfw_txt_record_create_dictionary")
public func nfwTXTRecordCreateDictionary() -> UnsafeMutableRawPointer? {
    nw_shim_txt_record_create_dictionary()
}

@_cdecl("nfw_txt_record_copy")
public func nfwTXTRecordCopy(_ txtRecord: UnsafeMutableRawPointer?) -> UnsafeMutableRawPointer? {
    nw_shim_txt_record_copy(txtRecord)
}

@_cdecl("nfw_txt_record_find_key")
public func nfwTXTRecordFindKey(_ txtRecord: UnsafeMutableRawPointer?, _ key: UnsafePointer<CChar>?) -> Int32 {
    nw_shim_txt_record_find_key(txtRecord, key)
}

@_cdecl("nfw_txt_record_copy_value")
public func nfwTXTRecordCopyValue(
    _ txtRecord: UnsafeMutableRawPointer?,
    _ key: UnsafePointer<CChar>?,
    _ outValueLength: UnsafeMutablePointer<Int>?,
    _ outFound: UnsafeMutablePointer<Int32>?
) -> UnsafeMutablePointer<UInt8>? {
    nw_shim_txt_record_copy_value(txtRecord, key, outValueLength, outFound)
}

@_cdecl("nfw_txt_record_set_key")
public func nfwTXTRecordSetKey(
    _ txtRecord: UnsafeMutableRawPointer?,
    _ key: UnsafePointer<CChar>?,
    _ value: UnsafePointer<UInt8>?,
    _ valueLength: Int
) -> Int32 {
    nw_shim_txt_record_set_key(txtRecord, key, value, valueLength)
}

@_cdecl("nfw_txt_record_remove_key")
public func nfwTXTRecordRemoveKey(_ txtRecord: UnsafeMutableRawPointer?, _ key: UnsafePointer<CChar>?) -> Int32 {
    nw_shim_txt_record_remove_key(txtRecord, key)
}

@_cdecl("nfw_txt_record_get_key_count")
public func nfwTXTRecordGetKeyCount(_ txtRecord: UnsafeMutableRawPointer?) -> Int {
    nw_shim_txt_record_get_key_count(txtRecord)
}

@_cdecl("nfw_txt_record_copy_bytes")
public func nfwTXTRecordCopyBytes(_ txtRecord: UnsafeMutableRawPointer?, _ outLength: UnsafeMutablePointer<Int>?) -> UnsafeMutablePointer<UInt8>? {
    nw_shim_txt_record_copy_bytes(txtRecord, outLength)
}

@_cdecl("nfw_txt_record_apply")
public func nfwTXTRecordApply(
    _ txtRecord: UnsafeMutableRawPointer?,
    _ callback: TxtRecordEntryCallback?,
    _ userInfo: UnsafeMutableRawPointer?
) -> Int32 {
    nw_shim_txt_record_apply(txtRecord, callback, userInfo)
}

@_cdecl("nfw_txt_record_is_dictionary")
public func nfwTXTRecordIsDictionary(_ txtRecord: UnsafeMutableRawPointer?) -> Int32 {
    nw_shim_txt_record_is_dictionary(txtRecord)
}

@_cdecl("nfw_txt_record_is_equal")
public func nfwTXTRecordIsEqual(_ txtRecord: UnsafeMutableRawPointer?, _ otherTXTRecord: UnsafeMutableRawPointer?) -> Int32 {
    nw_shim_txt_record_is_equal(txtRecord, otherTXTRecord)
}
