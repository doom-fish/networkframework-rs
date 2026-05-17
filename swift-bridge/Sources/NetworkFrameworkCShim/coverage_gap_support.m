#import <Foundation/Foundation.h>
#import <Network/Network.h>

#import "include/network_shim.h"

static inline id nw_shim_unretained_id(void *handle) {
    return handle ? (__bridge id)handle : nil;
}

static inline void *nw_shim_retained_handle(id object) {
    return object ? (void *)CFBridgingRetain(object) : NULL;
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
    void *user_info
) {
    if (options == NULL || callback == NULL) {
        return;
    }
    if (@available(macOS 10.15, *)) {
        dispatch_queue_t queue = dispatch_queue_create("networkframework-rs.ws.request", DISPATCH_QUEUE_SERIAL);
        nw_ws_options_set_client_request_handler((__bridge nw_protocol_options_t)options, queue, ^nw_ws_response_t(nw_ws_request_t request) {
            void *request_handle = nw_shim_retained_handle(request);
            void *response_handle = callback(request_handle, user_info);
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
