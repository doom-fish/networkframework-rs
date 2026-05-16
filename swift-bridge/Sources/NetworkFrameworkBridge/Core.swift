import Foundation
import NetworkFrameworkCShim

@_cdecl("nfw_retain_object")
public func nfwRetainObject(_ handle: UnsafeMutableRawPointer?) -> UnsafeMutableRawPointer? {
    nw_shim_retain_object(handle)
}

@_cdecl("nfw_release_object")
public func nfwReleaseObject(_ handle: UnsafeMutableRawPointer?) {
    nw_shim_release_object(handle)
}

@_cdecl("nfw_free_buffer")
public func nfwFreeBuffer(_ buffer: UnsafeMutableRawPointer?) {
    nw_shim_free_buffer(buffer)
}
