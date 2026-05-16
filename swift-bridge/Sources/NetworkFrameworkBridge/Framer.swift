import Foundation
import NetworkFrameworkCShim

@_cdecl("nfw_framer_definition_create")
public func nfwFramerDefinitionCreate(
    _ identifier: UnsafePointer<CChar>?,
    _ flags: UInt32,
    _ createInstance: FramerCreateInstanceCallback?,
    _ dropInstance: FramerDropInstanceCallback?,
    _ startCallback: FramerStartCallback?,
    _ inputCallback: FramerInputCallback?,
    _ outputCallback: FramerOutputCallback?,
    _ wakeupCallback: FramerWakeupCallback?,
    _ stopCallback: FramerStopCallback?,
    _ cleanupCallback: FramerCleanupCallback?,
    _ userInfo: UnsafeMutableRawPointer?
) -> UnsafeMutableRawPointer? {
    nw_shim_framer_definition_create(
        identifier,
        flags,
        createInstance,
        dropInstance,
        startCallback,
        inputCallback,
        outputCallback,
        wakeupCallback,
        stopCallback,
        cleanupCallback,
        userInfo
    )
}

@_cdecl("nfw_framer_create_options")
public func nfwFramerCreateOptions(_ definition: UnsafeMutableRawPointer?) -> UnsafeMutableRawPointer? {
    nw_shim_framer_create_options(definition)
}

@_cdecl("nfw_framer_message_create_from_options")
public func nfwFramerMessageCreateFromOptions(_ protocolOptions: UnsafeMutableRawPointer?) -> UnsafeMutableRawPointer? {
    nw_shim_framer_message_create_from_options(protocolOptions)
}

@_cdecl("nfw_framer_message_create_for_instance")
public func nfwFramerMessageCreateForInstance(_ framer: UnsafeMutableRawPointer?) -> UnsafeMutableRawPointer? {
    nw_shim_framer_message_create_for_instance(framer)
}

@_cdecl("nfw_framer_message_set_u64")
public func nfwFramerMessageSetU64(_ message: UnsafeMutableRawPointer?, _ key: UnsafePointer<CChar>?, _ value: UInt64) -> Int32 {
    nw_shim_framer_message_set_u64(message, key, value)
}

@_cdecl("nfw_framer_message_get_u64")
public func nfwFramerMessageGetU64(
    _ message: UnsafeMutableRawPointer?,
    _ key: UnsafePointer<CChar>?,
    _ outValue: UnsafeMutablePointer<UInt64>?
) -> Int32 {
    nw_shim_framer_message_get_u64(message, key, outValue)
}

@_cdecl("nfw_framer_parse_input")
public func nfwFramerParseInput(
    _ framer: UnsafeMutableRawPointer?,
    _ minimumIncompleteLength: Int,
    _ maximumLength: Int,
    _ tempBuffer: UnsafeMutablePointer<UInt8>?,
    _ parseCallback: FramerParseCallback?,
    _ userInfo: UnsafeMutableRawPointer?
) -> Int32 {
    nw_shim_framer_parse_input(framer, minimumIncompleteLength, maximumLength, tempBuffer, parseCallback, userInfo)
}

@_cdecl("nfw_framer_parse_output")
public func nfwFramerParseOutput(
    _ framer: UnsafeMutableRawPointer?,
    _ minimumIncompleteLength: Int,
    _ maximumLength: Int,
    _ tempBuffer: UnsafeMutablePointer<UInt8>?,
    _ parseCallback: FramerParseCallback?,
    _ userInfo: UnsafeMutableRawPointer?
) -> Int32 {
    nw_shim_framer_parse_output(framer, minimumIncompleteLength, maximumLength, tempBuffer, parseCallback, userInfo)
}

@_cdecl("nfw_framer_mark_ready")
public func nfwFramerMarkReady(_ framer: UnsafeMutableRawPointer?) { nw_shim_framer_mark_ready(framer) }

@_cdecl("nfw_framer_prepend_application_protocol")
public func nfwFramerPrependApplicationProtocol(_ framer: UnsafeMutableRawPointer?, _ protocolOptions: UnsafeMutableRawPointer?) -> Int32 {
    nw_shim_framer_prepend_application_protocol(framer, protocolOptions)
}

@_cdecl("nfw_framer_mark_failed_with_error")
public func nfwFramerMarkFailedWithError(_ framer: UnsafeMutableRawPointer?, _ errorCode: Int32) {
    nw_shim_framer_mark_failed_with_error(framer, errorCode)
}

@_cdecl("nfw_framer_pass_input_data")
public func nfwFramerPassInputData(_ framer: UnsafeMutableRawPointer?, _ inputLength: Int, _ message: UnsafeMutableRawPointer?, _ isComplete: Int32) -> Int32 {
    nw_shim_framer_pass_input_data(framer, inputLength, message, isComplete)
}

@_cdecl("nfw_framer_deliver_input_data")
public func nfwFramerDeliverInputData(
    _ framer: UnsafeMutableRawPointer?,
    _ inputBuffer: UnsafePointer<UInt8>?,
    _ inputLength: Int,
    _ message: UnsafeMutableRawPointer?,
    _ isComplete: Int32
) {
    nw_shim_framer_deliver_input_data(framer, inputBuffer, inputLength, message, isComplete)
}

@_cdecl("nfw_framer_pass_through_input")
public func nfwFramerPassThroughInput(_ framer: UnsafeMutableRawPointer?) { nw_shim_framer_pass_through_input(framer) }

@_cdecl("nfw_framer_pass_output_data")
public func nfwFramerPassOutputData(_ framer: UnsafeMutableRawPointer?, _ outputLength: Int) -> Int32 {
    nw_shim_framer_pass_output_data(framer, outputLength)
}

@_cdecl("nfw_framer_write_output_data")
public func nfwFramerWriteOutputData(_ framer: UnsafeMutableRawPointer?, _ outputBuffer: UnsafePointer<UInt8>?, _ outputLength: Int) {
    nw_shim_framer_write_output_data(framer, outputBuffer, outputLength)
}

@_cdecl("nfw_framer_pass_through_output")
public func nfwFramerPassThroughOutput(_ framer: UnsafeMutableRawPointer?) { nw_shim_framer_pass_through_output(framer) }

@_cdecl("nfw_framer_schedule_wakeup")
public func nfwFramerScheduleWakeup(_ framer: UnsafeMutableRawPointer?, _ milliseconds: UInt64) {
    nw_shim_framer_schedule_wakeup(framer, milliseconds)
}

@_cdecl("nfw_framer_async")
public func nfwFramerAsync(_ framer: UnsafeMutableRawPointer?, _ asyncCallback: FramerAsyncCallback?, _ userInfo: UnsafeMutableRawPointer?) {
    nw_shim_framer_async(framer, asyncCallback, userInfo)
}
