#import <Foundation/Foundation.h>
#import <Network/Network.h>
#import <Security/Security.h>

#import "include/network_shim.h"

static inline id nw_shim_unretained_id(void *handle) {
    return handle ? (__bridge id)handle : nil;
}

static inline void *nw_shim_retained_handle(id object) {
    return object ? (void *)CFBridgingRetain(object) : NULL;
}

@interface NFWRustContextOwner : NSObject
@property (nonatomic, readonly) void *context;
- (instancetype)initWithContext:(void *)context release:(NwShimContextCallback)release;
@end

@implementation NFWRustContextOwner {
    NwShimContextCallback _release;
}

- (instancetype)initWithContext:(void *)context release:(NwShimContextCallback)release {
    self = [super init];
    if (self) {
        _context = context;
        _release = release;
    }
    return self;
}

- (void)dealloc {
    if (_release && _context) {
        _release(_context);
    }
}

@end

static NFWRustContextOwner *nw_shim_make_owner(void *context, NwShimContextCallback release) {
    NFWRustContextOwner *owner = [[NFWRustContextOwner alloc] initWithContext:context release:release];
    if (!owner && release && context) {
        release(context);
    }
    return owner;
}

void *nw_shim_framer_definition_create(
    const char *identifier,
    uint32_t flags,
    FramerCreateInstanceCallback create_instance,
    FramerDropInstanceCallback drop_instance,
    FramerStartCallback start_callback,
    FramerInputCallback input_callback,
    FramerOutputCallback output_callback,
    FramerWakeupCallback wakeup_callback,
    FramerStopCallback stop_callback,
    FramerCleanupCallback cleanup_callback,
    void *factory,
    NwShimContextCallback release_factory
) {
    NFWRustContextOwner *owner = nw_shim_make_owner(factory, release_factory);
    if (!owner || !identifier || !create_instance || !start_callback || !input_callback || !output_callback) {
        return NULL;
    }
    nw_protocol_definition_t definition = nw_framer_create_definition(
        identifier,
        flags,
        ^nw_framer_start_result_t(nw_framer_t framer) {
            void *instance = create_instance(owner.context);
            nw_framer_set_input_handler(framer, ^size_t(nw_framer_t inner_framer) {
                return input_callback(instance, (__bridge void *)inner_framer);
            });
            nw_framer_set_output_handler(
                framer,
                ^(nw_framer_t inner_framer, nw_framer_message_t message, size_t message_length, bool is_complete) {
                    output_callback(
                        instance,
                        (__bridge void *)inner_framer,
                        (__bridge void *)message,
                        message_length,
                        is_complete ? 1 : 0);
                });
            if (wakeup_callback) {
                nw_framer_set_wakeup_handler(framer, ^(nw_framer_t inner_framer) {
                    wakeup_callback(instance, (__bridge void *)inner_framer);
                });
            }
            if (stop_callback) {
                nw_framer_set_stop_handler(framer, ^bool(nw_framer_t inner_framer) {
                    return stop_callback(instance, (__bridge void *)inner_framer) != 0;
                });
            }
            nw_framer_set_cleanup_handler(framer, ^(nw_framer_t inner_framer) {
                if (cleanup_callback) {
                    cleanup_callback(instance, (__bridge void *)inner_framer);
                }
                if (drop_instance) {
                    drop_instance(instance);
                }
            });

            int start_result = start_callback(instance, (__bridge void *)framer);
            return start_result == (int)nw_framer_start_result_will_mark_ready
                ? nw_framer_start_result_will_mark_ready
                : nw_framer_start_result_ready;
        });
    return definition ? (__bridge_retained void *)definition : NULL;
}

static bool nw_shim_evaluate_peer(sec_trust_t trust_ref, SecVerifyCallback callback, void *context) {
    SecTrustRef trust = sec_trust_copy_ref(trust_ref);
    if (!trust) {
        return false;
    }
    bool trusted = SecTrustEvaluateWithError(trust, NULL);
    CFArrayRef chain = SecTrustCopyCertificateChain(trust);
    CFIndex count = chain ? CFArrayGetCount(chain) : 0;
    bool accepted = false;
    if (count > 0) {
        NSMutableArray<NSData *> *certificates = [NSMutableArray arrayWithCapacity:(NSUInteger)count];
        for (CFIndex index = 0; index < count; index++) {
            SecCertificateRef certificate = (SecCertificateRef)CFArrayGetValueAtIndex(chain, index);
            NSData *der = CFBridgingRelease(SecCertificateCopyData(certificate));
            [certificates addObject:der ?: [NSData data]];
        }
        const uint8_t **pointers = (const uint8_t **)calloc((size_t)count, sizeof(uint8_t *));
        size_t *lengths = (size_t *)calloc((size_t)count, sizeof(size_t));
        if (pointers && lengths) {
            for (CFIndex index = 0; index < count; index++) {
                pointers[index] = (const uint8_t *)certificates[(NSUInteger)index].bytes;
                lengths[index] = certificates[(NSUInteger)index].length;
            }
            accepted = callback(pointers, lengths, (size_t)count, trusted ? 1 : 0, context) != 0;
        }
        free(pointers);
        free(lengths);
    }
    if (chain) {
        CFRelease(chain);
    }
    CFRelease(trust);
    return accepted;
}

void nw_shim_sec_options_set_verify_callback(
    void *options,
    SecVerifyCallback callback,
    void *context,
    NwShimContextCallback release
) {
    NFWRustContextOwner *owner = nw_shim_make_owner(context, release);
    if (!owner || !options || !callback) {
        return;
    }
    dispatch_queue_t queue = dispatch_queue_create("networkframework-rs.tls.verify", DISPATCH_QUEUE_SERIAL);
    sec_protocol_options_set_verify_block(
        (__bridge sec_protocol_options_t)options,
        ^(sec_protocol_metadata_t metadata, sec_trust_t trust_ref, sec_protocol_verify_complete_t complete) {
            (void)metadata;
            bool accepted = false;
            @autoreleasepool {
                accepted = nw_shim_evaluate_peer(trust_ref, callback, owner.context);
            }
            complete(accepted);
        },
        queue);
}

API_AVAILABLE(macos(15.0))
static void *nw_shim_import_pkcs12(
    const uint8_t *data,
    size_t length,
    const char *password,
    int *out_status,
    int32_t *out_os_status
) {
    @autoreleasepool {
        NSString *passphrase = [NSString stringWithUTF8String:password];
        if (!passphrase) {
            *out_status = NW_INVALID_ARG;
            return NULL;
        }
        NSData *blob = [NSData dataWithBytes:data length:length];
        NSDictionary *options = @{
            (__bridge id)kSecImportExportPassphrase: passphrase,
            (__bridge id)kSecImportToMemoryOnly: @YES,
        };
        CFArrayRef items = NULL;
        OSStatus status = SecPKCS12Import((__bridge CFDataRef)blob, (__bridge CFDictionaryRef)options, &items);
        NSArray *imported = items ? CFBridgingRelease(items) : nil;
        if (status != errSecSuccess) {
            *out_status = NW_SECURITY_FAILED;
            *out_os_status = status;
            return NULL;
        }
        id first = imported.count > 0 ? imported[0] : nil;
        id identity = [first isKindOfClass:[NSDictionary class]]
            ? ((NSDictionary *)first)[(__bridge id)kSecImportItemIdentity]
            : nil;
        if (!identity) {
            *out_status = NW_SECURITY_FAILED;
            *out_os_status = errSecItemNotFound;
            return NULL;
        }
        sec_identity_t sec_identity = sec_identity_create((__bridge SecIdentityRef)identity);
        if (!sec_identity) {
            *out_status = NW_SECURITY_FAILED;
            *out_os_status = errSecParam;
            return NULL;
        }
        *out_status = NW_OK;
        return (__bridge_retained void *)sec_identity;
    }
}

void *nw_shim_identity_create_from_pkcs12(
    const uint8_t *data,
    size_t length,
    const char *password,
    int *out_status,
    int32_t *out_os_status
) {
    int status = NW_INVALID_ARG;
    int32_t os_status = 0;
    void *identity = NULL;
    if (data && length > 0 && password) {
        if (@available(macOS 15.0, *)) {
            @try {
                identity = nw_shim_import_pkcs12(data, length, password, &status, &os_status);
            } @catch (NSException *exception) {
                (void)exception;
                identity = NULL;
                status = NW_SECURITY_FAILED;
                os_status = errSecDecode;
            }
        } else {
            status = NW_UNSUPPORTED;
        }
    }
    if (out_status) *out_status = status;
    if (out_os_status) *out_os_status = os_status;
    return identity;
}

void nw_shim_ws_metadata_set_pong_handler(
    void *metadata,
    WsPongCallback callback,
    void *user_info,
    NwShimContextCallback release
) {
    NFWRustContextOwner *owner = nw_shim_make_owner(user_info, release);
    if (!owner || metadata == NULL || !callback) {
        return;
    }
    dispatch_queue_t queue = dispatch_queue_create("networkframework-rs.ws.pong", DISPATCH_QUEUE_SERIAL);
    nw_ws_metadata_set_pong_handler((__bridge nw_protocol_metadata_t)metadata, queue, ^(nw_error_t error) {
        callback(nw_shim_retained_handle(error), owner.context);
    });
}

void nw_shim_framer_message_set_object_value(void *message, const char *key, void *value) {
    if (message == NULL || key == NULL) {
        return;
    }
    nw_framer_message_set_object_value((__bridge nw_framer_message_t)message, key, nw_shim_unretained_id(value));
}

void *nw_shim_framer_message_copy_object_value(void *message, const char *key) {
    if (message == NULL || key == NULL) {
        return NULL;
    }
    id value = nw_framer_message_copy_object_value((__bridge nw_framer_message_t)message, key);
    return nw_shim_retained_handle(value);
}

void nw_shim_framer_options_set_object_value(void *options, const char *key, void *value) {
    if (options == NULL || key == NULL) {
        return;
    }
    if (@available(macOS 12.3, *)) {
        nw_framer_options_set_object_value((__bridge nw_protocol_options_t)options, key, nw_shim_unretained_id(value));
    }
}

void *nw_shim_framer_options_copy_object_value(void *options, const char *key) {
    if (options == NULL || key == NULL) {
        return NULL;
    }
    if (@available(macOS 12.3, *)) {
        id value = nw_framer_options_copy_object_value((__bridge nw_protocol_options_t)options, key);
        return nw_shim_retained_handle(value);
    }
    return NULL;
}

void nw_shim_ws_options_set_client_request_handler(
    void *options,
    WsClientRequestCallback callback,
    void *user_info,
    NwShimContextCallback release
) {
    NFWRustContextOwner *owner = nw_shim_make_owner(user_info, release);
    if (!owner || options == NULL || callback == NULL) {
        return;
    }
    if (@available(macOS 10.15, *)) {
        dispatch_queue_t queue = dispatch_queue_create("networkframework-rs.ws.request", DISPATCH_QUEUE_SERIAL);
        nw_ws_options_set_client_request_handler((__bridge nw_protocol_options_t)options, queue, ^nw_ws_response_t(nw_ws_request_t request) {
            void *request_handle = nw_shim_retained_handle(request);
            void *response_handle = callback(request_handle, owner.context);
            if (response_handle == NULL) {
                return nw_ws_response_create(nw_ws_response_status_reject, NULL);
            }
            return (__bridge_transfer nw_ws_response_t)response_handle;
        });
    }
}

void *nw_shim_url_session_configuration_default(void) {
    if (@available(macOS 14.0, *)) {
        return nw_shim_retained_handle([NSURLSessionConfiguration defaultSessionConfiguration]);
    }
    return NULL;
}

void *nw_shim_url_session_configuration_ephemeral(void) {
    if (@available(macOS 14.0, *)) {
        return nw_shim_retained_handle([NSURLSessionConfiguration ephemeralSessionConfiguration]);
    }
    return NULL;
}

void nw_shim_url_session_configuration_release(void *configuration) {
    if (configuration != NULL) {
        CFBridgingRelease(configuration);
    }
}

void nw_shim_url_session_configuration_set_proxy_configurations(
    void *configuration,
    void *const *items,
    size_t count
) {
    if (configuration == NULL) {
        return;
    }
    if (@available(macOS 14.0, *)) {
        NSURLSessionConfiguration *session_configuration = (__bridge NSURLSessionConfiguration *)configuration;
        NSMutableArray *configs = [[NSMutableArray alloc] initWithCapacity:count];
        for (size_t index = 0; index < count; index++) {
            id item = nw_shim_unretained_id(items ? items[index] : NULL);
            if (item != nil) {
                [configs addObject:item];
            }
        }
        [session_configuration setValue:configs forKey:@"proxyConfigurations"];
    }
}

void **nw_shim_url_session_configuration_copy_proxy_configurations(
    void *configuration,
    size_t *out_count
) {
    if (out_count != NULL) {
        *out_count = 0;
    }
    if (configuration == NULL) {
        return NULL;
    }
    if (@available(macOS 14.0, *)) {
        NSURLSessionConfiguration *session_configuration = (__bridge NSURLSessionConfiguration *)configuration;
        id value = [session_configuration valueForKey:@"proxyConfigurations"];
        if (![value isKindOfClass:[NSArray class]]) {
            return NULL;
        }

        NSArray *configs = (NSArray *)value;
        NSUInteger count = configs.count;
        if (count == 0) {
            return NULL;
        }

        void **buffer = malloc(sizeof(void *) * count);
        if (buffer == NULL) {
            return NULL;
        }

        for (NSUInteger index = 0; index < count; index++) {
            buffer[index] = nw_shim_retained_handle([configs objectAtIndex:index]);
        }

        if (out_count != NULL) {
            *out_count = count;
        }
        return buffer;
    }
    return NULL;
}
