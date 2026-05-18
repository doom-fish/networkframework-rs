// Tiny C shim that hides Network.framework's heavy use of Objective-C
// blocks and dispatch_data behind a synchronous C API. Bridged 1:1 by
// src/ffi/mod.rs on the Rust side.

#include <Network/Network.h>
#include <dispatch/dispatch.h>
#include <CoreFoundation/CoreFoundation.h>
#include <arpa/inet.h>
#include <netinet/in.h>
#include <stdatomic.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "network_shim.h"

// ---------------------------------------------------------------------
// Connection (outbound TCP)
// ---------------------------------------------------------------------

typedef struct nw_conn_handle {
    nw_connection_t conn;          // retained
    dispatch_queue_t queue;        // retained
    dispatch_semaphore_t ready;
    _Atomic int state_code;        // 0=setup,1=ready,2=cancelled,3=failed
} nw_conn_handle;

static void destroy_handle(nw_conn_handle *h) {
    if (!h) return;
    if (h->conn) nw_release(h->conn);
    if (h->queue) dispatch_release(h->queue);
    if (h->ready) dispatch_release(h->ready);
    free(h);
}

static void cancel_and_wait_handle(nw_conn_handle *h) {
    if (!h || !h->conn) return;
    nw_connection_cancel(h->conn);
    dispatch_time_t deadline = dispatch_time(DISPATCH_TIME_NOW, 5LL * NSEC_PER_SEC);
    while (atomic_load(&h->state_code) != 2 && atomic_load(&h->state_code) != 3) {
        if (dispatch_semaphore_wait(h->ready, deadline) != 0) {
            break;
        }
    }
}

static void cancel_and_destroy_handle(nw_conn_handle *h) {
    cancel_and_wait_handle(h);
    destroy_handle(h);
}

void *nw_shim_tcp_connect(const char *host, uint16_t port, int use_tls, int *out_status) {
    if (!host) { if (out_status) *out_status = NW_INVALID_ARG; return NULL; }

    char port_str[8];
    snprintf(port_str, sizeof(port_str), "%u", (unsigned)port);
    nw_endpoint_t endpoint = nw_endpoint_create_host(host, port_str);
    if (!endpoint) { if (out_status) *out_status = NW_INVALID_ARG; return NULL; }

    nw_parameters_t params = nw_parameters_create_secure_tcp(
        use_tls ? NW_PARAMETERS_DEFAULT_CONFIGURATION : NW_PARAMETERS_DISABLE_PROTOCOL,
        NW_PARAMETERS_DEFAULT_CONFIGURATION);

    nw_connection_t conn = nw_connection_create(endpoint, params);
    nw_release(endpoint);
    nw_release(params);
    if (!conn) { if (out_status) *out_status = NW_CONNECT_FAILED; return NULL; }

    nw_conn_handle *h = (nw_conn_handle *)calloc(1, sizeof(nw_conn_handle));
    h->conn = conn;
    h->queue = dispatch_queue_create("networkframework-rs.conn", DISPATCH_QUEUE_SERIAL);
    h->ready = dispatch_semaphore_create(0);
    atomic_store(&h->state_code, 0);

    nw_connection_set_queue(conn, h->queue);

    nw_connection_set_state_changed_handler(conn, ^(nw_connection_state_t state, nw_error_t error) {
        (void)error;
        if (state == nw_connection_state_ready) {
            atomic_store(&h->state_code, 1);
            dispatch_semaphore_signal(h->ready);
        } else if (state == nw_connection_state_cancelled) {
            atomic_store(&h->state_code, 2);
            dispatch_semaphore_signal(h->ready);
        } else if (state == nw_connection_state_failed) {
            atomic_store(&h->state_code, 3);
            dispatch_semaphore_signal(h->ready);
        }
    });

    nw_connection_start(conn);

    // Wait up to 30 s for ready.
    dispatch_time_t deadline = dispatch_time(DISPATCH_TIME_NOW, 30LL * NSEC_PER_SEC);
    if (dispatch_semaphore_wait(h->ready, deadline) != 0) {
        cancel_and_destroy_handle(h);
        if (out_status) *out_status = NW_TIMEOUT;
        return NULL;
    }

    int code = atomic_load(&h->state_code);
    if (code != 1) {
        cancel_and_destroy_handle(h);
        if (out_status) *out_status = NW_CONNECT_FAILED;
        return NULL;
    }

    if (out_status) *out_status = NW_OK;
    return h;
}

int nw_shim_tcp_send(void *handle, const uint8_t *data, size_t len) {
    nw_conn_handle *h = (nw_conn_handle *)handle;
    if (!h || !data) return NW_INVALID_ARG;

    dispatch_data_t payload = dispatch_data_create(
        data, len, h->queue, DISPATCH_DATA_DESTRUCTOR_DEFAULT);

    __block int result = NW_OK;
    dispatch_semaphore_t done = dispatch_semaphore_create(0);

    nw_connection_send(h->conn, payload, NW_CONNECTION_DEFAULT_MESSAGE_CONTEXT, true,
        ^(nw_error_t error) {
            if (error) result = NW_SEND_FAILED;
            dispatch_semaphore_signal(done);
        });

    dispatch_semaphore_wait(done, DISPATCH_TIME_FOREVER);
    dispatch_release(done);
    dispatch_release(payload);
    return result;
}

// Returns number of bytes written into `out_buf` (positive) or negative status code.
ssize_t nw_shim_tcp_receive(void *handle, uint8_t *out_buf, size_t max_len) {
    nw_conn_handle *h = (nw_conn_handle *)handle;
    if (!h || !out_buf || max_len == 0) return NW_INVALID_ARG;

    __block ssize_t result = 0;
    __block size_t copied_out = 0;
    dispatch_semaphore_t done = dispatch_semaphore_create(0);

    nw_connection_receive(h->conn, 1, (uint32_t)max_len,
        ^(dispatch_data_t content, nw_content_context_t ctx, bool is_complete, nw_error_t error) {
            (void)ctx; (void)is_complete;
            if (error) {
                result = NW_RECV_FAILED;
            } else if (content) {
                __block size_t copied = 0;
                dispatch_data_apply(content,
                    ^bool(dispatch_data_t region, size_t offset, const void *buffer, size_t size) {
                        (void)region; (void)offset;
                        size_t can = max_len - copied;
                        if (can == 0) return false;
                        size_t take = size < can ? size : can;
                        memcpy(out_buf + copied, buffer, take);
                        copied += take;
                        return true;
                    });
                copied_out = copied;
                result = (ssize_t)copied;
            }
            dispatch_semaphore_signal(done);
        });

    dispatch_semaphore_wait(done, DISPATCH_TIME_FOREVER);
    dispatch_release(done);
    (void)copied_out;
    return result;
}

void nw_shim_tcp_close(void *handle) {
    nw_conn_handle *h = (nw_conn_handle *)handle;
    if (!h) return;
    cancel_and_destroy_handle(h);
}

// ---------------------------------------------------------------------
// Listener (inbound TCP)
// ---------------------------------------------------------------------

typedef struct nw_listener_handle {
    nw_listener_t listener;
    dispatch_queue_t queue;
    dispatch_semaphore_t ready;
    dispatch_semaphore_t accept_sem;
    nw_connection_t pending;       // protected by accept_sem
    _Atomic int state_code;        // 0=setup,1=ready,2=cancelled,3=failed
    _Atomic uint16_t bound_port;
} nw_listener_handle;

static void destroy_listener_handle(nw_listener_handle *h) {
    if (!h) return;
    if (h->pending) {
        nw_connection_cancel(h->pending);
        nw_release(h->pending);
    }
    if (h->listener) nw_release(h->listener);
    if (h->queue) dispatch_release(h->queue);
    if (h->ready) dispatch_release(h->ready);
    if (h->accept_sem) dispatch_release(h->accept_sem);
    free(h);
}

static void cancel_and_destroy_listener_handle(nw_listener_handle *h) {
    if (!h) return;
    if (h->listener) {
        nw_listener_set_state_changed_handler(h->listener, NULL);
        nw_listener_set_new_connection_handler(h->listener, NULL);
        nw_listener_cancel(h->listener);
        if (h->queue) {
            dispatch_sync(h->queue, ^{});
        }
        atomic_store(&h->state_code, 2);
    }
    destroy_listener_handle(h);
}

void *nw_shim_listener_create(uint16_t port, int use_tls, int *out_status) {
    char port_str[8];
    snprintf(port_str, sizeof(port_str), "%u", (unsigned)port);

    nw_parameters_t params = nw_parameters_create_secure_tcp(
        use_tls ? NW_PARAMETERS_DEFAULT_CONFIGURATION : NW_PARAMETERS_DISABLE_PROTOCOL,
        NW_PARAMETERS_DEFAULT_CONFIGURATION);

    nw_listener_t listener = nw_listener_create_with_port(port_str, params);
    nw_release(params);
    if (!listener) { if (out_status) *out_status = NW_LISTEN_FAILED; return NULL; }

    nw_listener_handle *h = (nw_listener_handle *)calloc(1, sizeof(nw_listener_handle));
    h->listener = listener;
    h->queue = dispatch_queue_create("networkframework-rs.listener", DISPATCH_QUEUE_SERIAL);
    h->ready = dispatch_semaphore_create(0);
    h->accept_sem = dispatch_semaphore_create(0);
    atomic_store(&h->state_code, 0);

    nw_listener_set_queue(listener, h->queue);

    nw_listener_set_state_changed_handler(listener, ^(nw_listener_state_t state, nw_error_t error) {
        (void)error;
        if (state == nw_listener_state_ready) {
            atomic_store(&h->bound_port, nw_listener_get_port(listener));
            atomic_store(&h->state_code, 1);
            dispatch_semaphore_signal(h->ready);
        } else if (state == nw_listener_state_cancelled) {
            atomic_store(&h->state_code, 2);
            dispatch_semaphore_signal(h->ready);
        } else if (state == nw_listener_state_failed) {
            atomic_store(&h->state_code, 3);
            dispatch_semaphore_signal(h->ready);
        }
    });

    nw_listener_set_new_connection_handler(listener, ^(nw_connection_t conn) {
        // Hold the most recent pending connection. Caller must accept
        // promptly or older ones get dropped.
        nw_retain(conn);
        if (h->pending) {
            nw_connection_cancel(h->pending);
            nw_release(h->pending);
        }
        h->pending = conn;
        dispatch_semaphore_signal(h->accept_sem);
    });

    nw_listener_start(listener);

    dispatch_time_t deadline = dispatch_time(DISPATCH_TIME_NOW, 10LL * NSEC_PER_SEC);
    if (dispatch_semaphore_wait(h->ready, deadline) != 0) {
        cancel_and_destroy_listener_handle(h);
        if (out_status) *out_status = NW_LISTEN_FAILED;
        return NULL;
    }

    if (atomic_load(&h->state_code) != 1) {
        cancel_and_destroy_listener_handle(h);
        if (out_status) *out_status = NW_LISTEN_FAILED;
        return NULL;
    }

    if (out_status) *out_status = NW_OK;
    return h;
}

uint16_t nw_shim_listener_port(void *handle) {
    nw_listener_handle *h = (nw_listener_handle *)handle;
    if (!h) return 0;
    return atomic_load(&h->bound_port);
}

// Blocking accept. Returns a `nw_conn_handle*` cast to void*, or NULL.
void *nw_shim_listener_accept(void *handle, int *out_status) {
    nw_listener_handle *h = (nw_listener_handle *)handle;
    if (!h) { if (out_status) *out_status = NW_INVALID_ARG; return NULL; }

    dispatch_semaphore_wait(h->accept_sem, DISPATCH_TIME_FOREVER);
    nw_connection_t conn = h->pending;
    h->pending = NULL;
    if (!conn) { if (out_status) *out_status = NW_CANCELLED; return NULL; }

    nw_conn_handle *ch = (nw_conn_handle *)calloc(1, sizeof(nw_conn_handle));
    ch->conn = conn;  // already retained when stored
    ch->queue = dispatch_queue_create("networkframework-rs.accepted", DISPATCH_QUEUE_SERIAL);
    ch->ready = dispatch_semaphore_create(0);
    atomic_store(&ch->state_code, 0);

    nw_connection_set_queue(conn, ch->queue);
    nw_connection_set_state_changed_handler(conn, ^(nw_connection_state_t state, nw_error_t error) {
        (void)error;
        if (state == nw_connection_state_ready) {
            atomic_store(&ch->state_code, 1);
            dispatch_semaphore_signal(ch->ready);
        } else if (state == nw_connection_state_cancelled) {
            atomic_store(&ch->state_code, 2);
            dispatch_semaphore_signal(ch->ready);
        } else if (state == nw_connection_state_failed) {
            atomic_store(&ch->state_code, 3);
            dispatch_semaphore_signal(ch->ready);
        }
    });
    nw_connection_start(conn);

    dispatch_time_t deadline = dispatch_time(DISPATCH_TIME_NOW, 10LL * NSEC_PER_SEC);
    if (dispatch_semaphore_wait(ch->ready, deadline) != 0
        || atomic_load(&ch->state_code) != 1) {
        cancel_and_destroy_handle(ch);
        if (out_status) *out_status = NW_CONNECT_FAILED;
        return NULL;
    }

    if (out_status) *out_status = NW_OK;
    return ch;
}

void nw_shim_listener_close(void *handle) {
    nw_listener_handle *h = (nw_listener_handle *)handle;
    if (!h) return;
    cancel_and_destroy_listener_handle(h);
}

// ---------------------------------------------------------------------
// UDP (datagram) connection
// ---------------------------------------------------------------------

void *nw_shim_udp_connect(const char *host, uint16_t port, int *out_status) {
    if (!host) { if (out_status) *out_status = NW_INVALID_ARG; return NULL; }

    char port_str[8];
    snprintf(port_str, sizeof(port_str), "%u", (unsigned)port);
    nw_endpoint_t endpoint = nw_endpoint_create_host(host, port_str);
    if (!endpoint) { if (out_status) *out_status = NW_INVALID_ARG; return NULL; }

    nw_parameters_t params = nw_parameters_create_secure_udp(
        NW_PARAMETERS_DISABLE_PROTOCOL,
        NW_PARAMETERS_DEFAULT_CONFIGURATION);

    nw_connection_t conn = nw_connection_create(endpoint, params);
    nw_release(endpoint);
    nw_release(params);
    if (!conn) { if (out_status) *out_status = NW_CONNECT_FAILED; return NULL; }

    nw_conn_handle *h = (nw_conn_handle *)calloc(1, sizeof(nw_conn_handle));
    h->conn = conn;
    h->queue = dispatch_queue_create("networkframework-rs.udp", DISPATCH_QUEUE_SERIAL);
    h->ready = dispatch_semaphore_create(0);
    atomic_store(&h->state_code, 0);

    nw_connection_set_queue(conn, h->queue);

    nw_connection_set_state_changed_handler(conn, ^(nw_connection_state_t state, nw_error_t error) {
        (void)error;
        if (state == nw_connection_state_ready) {
            atomic_store(&h->state_code, 1);
            dispatch_semaphore_signal(h->ready);
        } else if (state == nw_connection_state_cancelled) {
            atomic_store(&h->state_code, 2);
            dispatch_semaphore_signal(h->ready);
        } else if (state == nw_connection_state_failed) {
            atomic_store(&h->state_code, 3);
            dispatch_semaphore_signal(h->ready);
        }
    });

    nw_connection_start(conn);

    dispatch_time_t deadline = dispatch_time(DISPATCH_TIME_NOW, 10LL * NSEC_PER_SEC);
    if (dispatch_semaphore_wait(h->ready, deadline) != 0
        || atomic_load(&h->state_code) != 1) {
        cancel_and_destroy_handle(h);
        if (out_status) *out_status = NW_CONNECT_FAILED;
        return NULL;
    }

    if (out_status) *out_status = NW_OK;
    return h;
}

// UDP send / receive / close all use the same TCP shim entrypoints
// (nw_connection_send / nw_connection_receive), so callers can just use
// nw_shim_tcp_send / nw_shim_tcp_receive / nw_shim_tcp_close with a UDP
// handle.

// ---------------------------------------------------------------------
// Path monitor (network reachability / interface changes)
// ---------------------------------------------------------------------

typedef struct nw_path_handle {
    nw_path_monitor_t monitor;
    dispatch_queue_t queue;
    void (*callback)(int satisfied, int interface_type, void *user_info);
    void *user_info;
    nw_path_t latest_path;
} nw_path_handle;

// interface_type matches nw_interface_type_t:
//   0=other, 1=wifi, 2=cellular, 3=wired, 4=loopback
void *nw_shim_path_monitor_start(
    void (*callback)(int satisfied, int interface_type, void *user_info),
    void *user_info
) {
    nw_path_handle *h = (nw_path_handle *)calloc(1, sizeof(nw_path_handle));
    h->monitor = nw_path_monitor_create();
    h->queue = dispatch_queue_create("networkframework-rs.path", DISPATCH_QUEUE_SERIAL);
    h->callback = callback;
    h->user_info = user_info;

    nw_path_monitor_set_queue(h->monitor, h->queue);
    nw_path_monitor_set_update_handler(h->monitor, ^(nw_path_t path) {
        if (h->latest_path) {
            nw_release(h->latest_path);
            h->latest_path = NULL;
        }
        if (path) {
            h->latest_path = nw_retain(path);
        }
        if (!h->callback) return;
        int satisfied = (nw_path_get_status(path) == nw_path_status_satisfied) ? 1 : 0;
        int iface = 0;
        if (nw_path_uses_interface_type(path, nw_interface_type_wifi)) iface = 1;
        else if (nw_path_uses_interface_type(path, nw_interface_type_cellular)) iface = 2;
        else if (nw_path_uses_interface_type(path, nw_interface_type_wired)) iface = 3;
        else if (nw_path_uses_interface_type(path, nw_interface_type_loopback)) iface = 4;
        h->callback(satisfied, iface, h->user_info);
    });
    nw_path_monitor_start(h->monitor);
    return h;
}

void nw_shim_path_monitor_stop(void *handle) {
    nw_path_handle *h = (nw_path_handle *)handle;
    if (!h) return;
    nw_path_monitor_cancel(h->monitor);
    dispatch_sync(h->queue, ^{});
    if (h->latest_path) {
        nw_release(h->latest_path);
    }
    nw_release(h->monitor);
    dispatch_release(h->queue);
    free(h);
}

// ---------------------------------------------------------------------
// Bonjour browser (nw_browser)
// ---------------------------------------------------------------------

typedef struct nw_browser_handle {
    nw_browser_t browser;
    dispatch_queue_t queue;
    void (*found_callback)(const char *name, const char *service_type,
                           const char *domain, void *user_info);
    void (*lost_callback)(const char *name, const char *service_type,
                          const char *domain, void *user_info);
    void *user_info;
} nw_browser_handle;

void *nw_shim_browser_start(
    const char *service_type,
    const char *domain,
    void (*found_callback)(const char *name, const char *service_type,
                           const char *domain, void *user_info),
    void (*lost_callback)(const char *name, const char *service_type,
                          const char *domain, void *user_info),
    void *user_info
) {
    if (!service_type) return NULL;
    nw_browse_descriptor_t desc = nw_browse_descriptor_create_bonjour_service(
        service_type, domain);
    if (!desc) return NULL;
    nw_parameters_t params = nw_parameters_create();
    nw_browser_t browser = nw_browser_create(desc, params);
    nw_release(desc);
    nw_release(params);
    if (!browser) return NULL;

    nw_browser_handle *h = (nw_browser_handle *)calloc(1, sizeof(nw_browser_handle));
    h->browser = browser;
    h->queue = dispatch_queue_create("networkframework-rs.browser", DISPATCH_QUEUE_SERIAL);
    h->found_callback = found_callback;
    h->lost_callback = lost_callback;
    h->user_info = user_info;

    nw_browser_set_queue(browser, h->queue);

    nw_browser_set_browse_results_changed_handler(browser,
        ^(nw_browse_result_t old_result, nw_browse_result_t new_result, bool batch_complete) {
            (void)batch_complete;
            if (!old_result && new_result) {
                // added
                nw_endpoint_t ep = nw_browse_result_copy_endpoint(new_result);
                if (ep && h->found_callback) {
                    const char *name = nw_endpoint_get_bonjour_service_name(ep);
                    const char *type = nw_endpoint_get_bonjour_service_type(ep);
                    const char *dom = nw_endpoint_get_bonjour_service_domain(ep);
                    h->found_callback(
                        name ? name : "", type ? type : "", dom ? dom : "",
                        h->user_info);
                }
                if (ep) nw_release(ep);
            } else if (old_result && !new_result) {
                // removed
                nw_endpoint_t ep = nw_browse_result_copy_endpoint(old_result);
                if (ep && h->lost_callback) {
                    const char *name = nw_endpoint_get_bonjour_service_name(ep);
                    const char *type = nw_endpoint_get_bonjour_service_type(ep);
                    const char *dom = nw_endpoint_get_bonjour_service_domain(ep);
                    h->lost_callback(
                        name ? name : "", type ? type : "", dom ? dom : "",
                        h->user_info);
                }
                if (ep) nw_release(ep);
            }
        });

    nw_browser_start(browser);
    return h;
}

void nw_shim_browser_stop(void *handle) {
    nw_browser_handle *h = (nw_browser_handle *)handle;
    if (!h) return;
    nw_browser_cancel(h->browser);
    dispatch_sync(h->queue, ^{});
    nw_release(h->browser);
    dispatch_release(h->queue);
    free(h);
}

// ---------------------------------------------------------------------
// WebSocket (nw_ws_*) connection — v0.5
// ---------------------------------------------------------------------
// Builds a websocket connection by chaining ws -> tcp / tls protocols
// into a custom nw_parameters_t.

void *nw_shim_ws_connect(const char *host, uint16_t port, const char *path, int use_tls, int *out_status) {
    if (!host) { if (out_status) *out_status = NW_INVALID_ARG; return NULL; }
    (void)path; // path is encoded in the URL endpoint below.

    nw_parameters_t params = nw_parameters_create_secure_tcp(
        use_tls ? NW_PARAMETERS_DEFAULT_CONFIGURATION : NW_PARAMETERS_DISABLE_PROTOCOL,
        NW_PARAMETERS_DEFAULT_CONFIGURATION);

    // Insert WebSocket framing on top of (TLS)TCP.
    nw_protocol_options_t ws_opts = nw_ws_create_options(nw_ws_version_13);
    nw_protocol_stack_t stack = nw_parameters_copy_default_protocol_stack(params);
    nw_protocol_stack_prepend_application_protocol(stack, ws_opts);
    nw_release(ws_opts);
    nw_release(stack);

    // URL endpoint encodes ws:// or wss:// + host + port + path.
    char url[2048];
    snprintf(url, sizeof(url), "%s://%s:%u%s",
             use_tls ? "wss" : "ws", host, (unsigned)port,
             (path && path[0]) ? path : "/");
    nw_endpoint_t endpoint = nw_endpoint_create_url(url);
    if (!endpoint) {
        nw_release(params);
        if (out_status) *out_status = NW_INVALID_ARG;
        return NULL;
    }

    nw_connection_t conn = nw_connection_create(endpoint, params);
    nw_release(endpoint);
    nw_release(params);
    if (!conn) { if (out_status) *out_status = NW_CONNECT_FAILED; return NULL; }

    nw_conn_handle *h = (nw_conn_handle *)calloc(1, sizeof(nw_conn_handle));
    h->conn = conn;
    h->queue = dispatch_queue_create("networkframework-rs.ws", DISPATCH_QUEUE_SERIAL);
    h->ready = dispatch_semaphore_create(0);
    atomic_store(&h->state_code, 0);

    nw_connection_set_queue(conn, h->queue);
    nw_connection_set_state_changed_handler(conn, ^(nw_connection_state_t state, nw_error_t error) {
        (void)error;
        if (state == nw_connection_state_ready) {
            atomic_store(&h->state_code, 1);
            dispatch_semaphore_signal(h->ready);
        } else if (state == nw_connection_state_cancelled) {
            atomic_store(&h->state_code, 2);
            dispatch_semaphore_signal(h->ready);
        } else if (state == nw_connection_state_failed) {
            atomic_store(&h->state_code, 3);
            dispatch_semaphore_signal(h->ready);
        }
    });
    nw_connection_start(conn);

    dispatch_time_t deadline = dispatch_time(DISPATCH_TIME_NOW, 30LL * NSEC_PER_SEC);
    if (dispatch_semaphore_wait(h->ready, deadline) != 0
        || atomic_load(&h->state_code) != 1) {
        cancel_and_destroy_handle(h);
        if (out_status) *out_status = NW_CONNECT_FAILED;
        return NULL;
    }

    if (out_status) *out_status = NW_OK;
    return h;
}

// Send a text or binary websocket message. opcode:
//   1 = text, 2 = binary, 8 = close, 9 = ping, 10 = pong.
int nw_shim_ws_send(void *handle, const uint8_t *data, size_t len, int opcode) {
    nw_conn_handle *h = (nw_conn_handle *)handle;
    if (!h || !data) return NW_INVALID_ARG;

    nw_ws_opcode_t op = (nw_ws_opcode_t)opcode;
    nw_protocol_metadata_t metadata = nw_ws_create_metadata(op);
    nw_content_context_t ctx = nw_content_context_create("ws-send");
    nw_content_context_set_metadata_for_protocol(ctx, metadata);
    nw_release(metadata);

    dispatch_data_t payload = dispatch_data_create(
        data, len, h->queue, DISPATCH_DATA_DESTRUCTOR_DEFAULT);

    __block int result = NW_OK;
    dispatch_semaphore_t done = dispatch_semaphore_create(0);

    nw_connection_send(h->conn, payload, ctx, true, ^(nw_error_t error) {
        if (error) result = NW_SEND_FAILED;
        dispatch_semaphore_signal(done);
    });

    dispatch_semaphore_wait(done, DISPATCH_TIME_FOREVER);
    dispatch_release(done);
    dispatch_release(payload);
    nw_release(ctx);
    return result;
}

ssize_t nw_shim_ws_receive(void *handle, uint8_t *out_buf, size_t max_len, int *out_opcode) {
    nw_conn_handle *h = (nw_conn_handle *)handle;
    if (!h || !out_buf || max_len == 0) return NW_INVALID_ARG;

    __block ssize_t result = 0;
    __block int op = 0;
    dispatch_semaphore_t done = dispatch_semaphore_create(0);

    nw_connection_receive_message(h->conn,
        ^(dispatch_data_t content, nw_content_context_t ctx, bool is_complete, nw_error_t error) {
            (void)is_complete;
            if (error) {
                result = NW_RECV_FAILED;
            } else {
                if (ctx) {
                    nw_protocol_metadata_t md = nw_content_context_copy_protocol_metadata(
                        ctx, nw_protocol_copy_ws_definition());
                    if (md) {
                        op = (int)nw_ws_metadata_get_opcode(md);
                        nw_release(md);
                    }
                }
                if (content) {
                    __block size_t copied = 0;
                    dispatch_data_apply(content,
                        ^bool(dispatch_data_t region, size_t offset, const void *buffer, size_t size) {
                            (void)region; (void)offset;
                            size_t can = max_len - copied;
                            if (can == 0) return false;
                            size_t take = size < can ? size : can;
                            memcpy(out_buf + copied, buffer, take);
                            copied += take;
                            return true;
                        });
                    result = (ssize_t)copied;
                }
            }
            dispatch_semaphore_signal(done);
        });

    dispatch_semaphore_wait(done, DISPATCH_TIME_FOREVER);
    dispatch_release(done);
    if (out_opcode) *out_opcode = op;
    return result;
}

// ---------------------------------------------------------------------
// QUIC (single-stream) — v0.6
// ---------------------------------------------------------------------
// Builds a QUIC connection with one bidirectional stream by chaining
// quic + IP into a custom nw_parameters_t. send/receive on the returned
// handle work the same as the TCP variant.

void *nw_shim_quic_connect(const char *host, uint16_t port, const char *alpn, int *out_status) {
    if (!host) { if (out_status) *out_status = NW_INVALID_ARG; return NULL; }

    nw_protocol_options_t quic_opts = nw_quic_create_options();
    if (alpn && alpn[0]) {
        nw_quic_add_tls_application_protocol(quic_opts, alpn);
    }

    nw_parameters_configure_protocol_block_t cfg_protocols =
        ^(nw_protocol_options_t opts) {
            (void)opts;
        };
    nw_parameters_t params = nw_parameters_create_secure_udp(
        cfg_protocols,
        NW_PARAMETERS_DEFAULT_CONFIGURATION);

    nw_protocol_stack_t stack = nw_parameters_copy_default_protocol_stack(params);
    nw_protocol_stack_prepend_application_protocol(stack, quic_opts);
    nw_release(quic_opts);
    nw_release(stack);

    char port_str[8];
    snprintf(port_str, sizeof(port_str), "%u", (unsigned)port);
    nw_endpoint_t endpoint = nw_endpoint_create_host(host, port_str);
    if (!endpoint) {
        nw_release(params);
        if (out_status) *out_status = NW_INVALID_ARG;
        return NULL;
    }

    nw_connection_t conn = nw_connection_create(endpoint, params);
    nw_release(endpoint);
    nw_release(params);
    if (!conn) { if (out_status) *out_status = NW_CONNECT_FAILED; return NULL; }

    nw_conn_handle *h = (nw_conn_handle *)calloc(1, sizeof(nw_conn_handle));
    h->conn = conn;
    h->queue = dispatch_queue_create("networkframework-rs.quic", DISPATCH_QUEUE_SERIAL);
    h->ready = dispatch_semaphore_create(0);
    atomic_store(&h->state_code, 0);

    nw_connection_set_queue(conn, h->queue);
    nw_connection_set_state_changed_handler(conn, ^(nw_connection_state_t state, nw_error_t error) {
        (void)error;
        if (state == nw_connection_state_ready) {
            atomic_store(&h->state_code, 1);
            dispatch_semaphore_signal(h->ready);
        } else if (state == nw_connection_state_cancelled) {
            atomic_store(&h->state_code, 2);
            dispatch_semaphore_signal(h->ready);
        } else if (state == nw_connection_state_failed) {
            atomic_store(&h->state_code, 3);
            dispatch_semaphore_signal(h->ready);
        }
    });
    nw_connection_start(conn);

    dispatch_time_t deadline = dispatch_time(DISPATCH_TIME_NOW, 30LL * NSEC_PER_SEC);
    if (dispatch_semaphore_wait(h->ready, deadline) != 0
        || atomic_load(&h->state_code) != 1) {
        cancel_and_destroy_handle(h);
        if (out_status) *out_status = NW_CONNECT_FAILED;
        return NULL;
    }

    if (out_status) *out_status = NW_OK;
    return h;
}

// ---------------------------------------------------------------------
// Bonjour advertisement (nw_listener with a bonjour service endpoint)
// ---------------------------------------------------------------------

typedef struct nw_bonjour_advertise_handle {
    nw_listener_t listener;
    dispatch_queue_t queue;
    dispatch_semaphore_t ready;
    _Atomic int state_code;
} nw_bonjour_advertise_handle;

void *nw_shim_bonjour_advertise_start(
    const char *service_type,
    const char *service_name,
    const char *domain,
    uint16_t port,
    int *out_status
) {
    if (!service_type || !service_name) {
        if (out_status) *out_status = NW_INVALID_ARG;
        return NULL;
    }

    nw_parameters_t params = nw_parameters_create_secure_tcp(
        NW_PARAMETERS_DISABLE_PROTOCOL,
        NW_PARAMETERS_DEFAULT_CONFIGURATION);

    char port_str[8];
    snprintf(port_str, sizeof(port_str), "%u", (unsigned)port);
    nw_listener_t listener = nw_listener_create_with_port(port_str, params);
    nw_release(params);
    if (!listener) {
        if (out_status) *out_status = NW_LISTEN_FAILED;
        return NULL;
    }

    nw_advertise_descriptor_t adv = nw_advertise_descriptor_create_bonjour_service(
        service_name,
        service_type,
        (domain && domain[0]) ? domain : NULL);
    if (!adv) {
        nw_release(listener);
        if (out_status) *out_status = NW_INVALID_ARG;
        return NULL;
    }
    nw_listener_set_advertise_descriptor(listener, adv);
    nw_release(adv);

    nw_bonjour_advertise_handle *h = (nw_bonjour_advertise_handle *)calloc(
        1, sizeof(nw_bonjour_advertise_handle));
    h->listener = listener;
    h->queue = dispatch_queue_create("networkframework-rs.bonjour-adv", DISPATCH_QUEUE_SERIAL);
    h->ready = dispatch_semaphore_create(0);
    atomic_store(&h->state_code, 0);

    nw_listener_set_queue(listener, h->queue);
    nw_listener_set_state_changed_handler(listener, ^(nw_listener_state_t state, nw_error_t error) {
        (void)error;
        if (state == nw_listener_state_ready) {
            atomic_store(&h->state_code, 1);
            dispatch_semaphore_signal(h->ready);
        } else if (state == nw_listener_state_cancelled) {
            atomic_store(&h->state_code, 2);
            dispatch_semaphore_signal(h->ready);
        } else if (state == nw_listener_state_failed) {
            atomic_store(&h->state_code, 3);
            dispatch_semaphore_signal(h->ready);
        }
    });
    nw_listener_set_new_connection_handler(listener, ^(nw_connection_t conn) {
        // Drop inbound connections — we're advertising only.
        nw_connection_cancel(conn);
    });
    nw_listener_start(listener);

    dispatch_time_t deadline = dispatch_time(DISPATCH_TIME_NOW, 10LL * NSEC_PER_SEC);
    if (dispatch_semaphore_wait(h->ready, deadline) != 0
        || atomic_load(&h->state_code) != 1) {
        nw_listener_cancel(listener);
        dispatch_time_t cancel_deadline = dispatch_time(DISPATCH_TIME_NOW, 5LL * NSEC_PER_SEC);
        while (atomic_load(&h->state_code) != 2 && atomic_load(&h->state_code) != 3) {
            if (dispatch_semaphore_wait(h->ready, cancel_deadline) != 0) {
                break;
            }
        }
        nw_release(h->listener);
        dispatch_release(h->queue);
        dispatch_release(h->ready);
        free(h);
        if (out_status) *out_status = NW_LISTEN_FAILED;
        return NULL;
    }

    if (out_status) *out_status = NW_OK;
    return h;
}

void nw_shim_bonjour_advertise_stop(void *handle) {
    nw_bonjour_advertise_handle *h = (nw_bonjour_advertise_handle *)handle;
    if (!h) return;
    nw_listener_cancel(h->listener);
    dispatch_time_t deadline = dispatch_time(DISPATCH_TIME_NOW, 5LL * NSEC_PER_SEC);
    while (atomic_load(&h->state_code) != 2 && atomic_load(&h->state_code) != 3) {
        if (dispatch_semaphore_wait(h->ready, deadline) != 0) {
            break;
        }
    }
    nw_release(h->listener);
    dispatch_release(h->queue);
    dispatch_release(h->ready);
    free(h);
}

// ---------------------------------------------------------------------
// Generic Network.framework object helpers
// ---------------------------------------------------------------------

static nw_endpoint_t nw_shim_create_host_endpoint(const char *host, uint16_t port) {
    if (!host) {
        return NULL;
    }
    char port_str[8];
    snprintf(port_str, sizeof(port_str), "%u", (unsigned)port);
    return nw_endpoint_create_host(host, port_str);
}

static nw_endpoint_t nw_shim_create_address_endpoint(const char *address, uint16_t port) {
    if (!address) {
        return NULL;
    }

    struct sockaddr_in addr4;
    memset(&addr4, 0, sizeof(addr4));
    if (inet_pton(AF_INET, address, &addr4.sin_addr) == 1) {
        addr4.sin_len = sizeof(addr4);
        addr4.sin_family = AF_INET;
        addr4.sin_port = htons(port);
        return nw_endpoint_create_address((const struct sockaddr *)&addr4);
    }

    struct sockaddr_in6 addr6;
    memset(&addr6, 0, sizeof(addr6));
    if (inet_pton(AF_INET6, address, &addr6.sin6_addr) == 1) {
        addr6.sin6_len = sizeof(addr6);
        addr6.sin6_family = AF_INET6;
        addr6.sin6_port = htons(port);
        return nw_endpoint_create_address((const struct sockaddr *)&addr6);
    }

    return NULL;
}

static size_t nw_shim_copy_dispatch_data(dispatch_data_t data, uint8_t **out_bytes) {
    if (out_bytes) {
        *out_bytes = NULL;
    }
    if (!data || !out_bytes) {
        return 0;
    }

    size_t length = dispatch_data_get_size(data);
    if (length == 0) {
        return 0;
    }

    uint8_t *bytes = (uint8_t *)malloc(length);
    if (!bytes) {
        return 0;
    }

    __block size_t copied = 0;
    dispatch_data_apply(data, ^bool(dispatch_data_t region, size_t offset, const void *buffer, size_t size) {
        (void)region;
        (void)offset;
        memcpy(bytes + copied, buffer, size);
        copied += size;
        return true;
    });

    *out_bytes = bytes;
    return copied;
}

void *nw_shim_retain_object(void *handle) {
    if (!handle) {
        return NULL;
    }
    return nw_retain((nw_object_t)handle);
}

void nw_shim_release_object(void *handle) {
    if (!handle) {
        return;
    }
    nw_release((nw_object_t)handle);
}

// ---------------------------------------------------------------------
// Parameters + advanced connection creation
// ---------------------------------------------------------------------

static nw_parameters_t nw_shim_create_quic_parameters(const char *alpn) {
    nw_protocol_options_t quic_opts = nw_quic_create_options();
    if (!quic_opts) {
        return NULL;
    }
    if (alpn && alpn[0]) {
        nw_quic_add_tls_application_protocol(quic_opts, alpn);
    }

    nw_parameters_t params = nw_parameters_create_secure_udp(
        NW_PARAMETERS_DISABLE_PROTOCOL,
        NW_PARAMETERS_DEFAULT_CONFIGURATION);
    if (!params) {
        nw_release(quic_opts);
        return NULL;
    }

    nw_protocol_stack_t stack = nw_parameters_copy_default_protocol_stack(params);
    if (stack) {
        nw_protocol_stack_prepend_application_protocol(stack, quic_opts);
        nw_release(stack);
    }
    nw_release(quic_opts);
    return params;
}

void *nw_shim_parameters_create_tcp(int use_tls) {
    return nw_parameters_create_secure_tcp(
        use_tls ? NW_PARAMETERS_DEFAULT_CONFIGURATION : NW_PARAMETERS_DISABLE_PROTOCOL,
        NW_PARAMETERS_DEFAULT_CONFIGURATION);
}

void *nw_shim_parameters_create_udp(void) {
    return nw_parameters_create_secure_udp(
        NW_PARAMETERS_DISABLE_PROTOCOL,
        NW_PARAMETERS_DEFAULT_CONFIGURATION);
}

void *nw_shim_parameters_create_quic(const char *alpn) {
    if (__builtin_available(macOS 10.15, *)) {
        return nw_shim_create_quic_parameters(alpn);
    }
    return NULL;
}

void *nw_shim_parameters_copy(void *handle) {
    if (!handle) {
        return NULL;
    }
    return nw_parameters_copy((nw_parameters_t)handle);
}

int nw_shim_parameters_prepend_application_protocol(void *parameters, void *protocol_options) {
    if (!parameters || !protocol_options) {
        return NW_INVALID_ARG;
    }

    nw_protocol_stack_t stack = nw_parameters_copy_default_protocol_stack((nw_parameters_t)parameters);
    if (!stack) {
        return NW_INVALID_ARG;
    }

    nw_protocol_stack_prepend_application_protocol(stack, (nw_protocol_options_t)protocol_options);
    nw_release(stack);
    return NW_OK;
}

void nw_shim_parameters_set_privacy_context(void *parameters, void *privacy_context) {
    if (!parameters || !privacy_context) {
        return;
    }
    if (__builtin_available(macOS 11.0, *)) {
        nw_parameters_set_privacy_context((nw_parameters_t)parameters, (nw_privacy_context_t)privacy_context);
    }
}

void nw_shim_parameters_set_prefer_no_proxy(void *parameters, int prefer_no_proxy) {
    if (!parameters) {
        return;
    }
    nw_parameters_set_prefer_no_proxy((nw_parameters_t)parameters, prefer_no_proxy != 0);
}

void *nw_shim_connection_create_with_parameters(
    const char *host,
    uint16_t port,
    void *parameters,
    int *out_status
) {
    if (!host || !parameters) {
        if (out_status) *out_status = NW_INVALID_ARG;
        return NULL;
    }

    nw_endpoint_t endpoint = nw_shim_create_host_endpoint(host, port);
    if (!endpoint) {
        if (out_status) *out_status = NW_INVALID_ARG;
        return NULL;
    }

    nw_connection_t conn = nw_connection_create(endpoint, (nw_parameters_t)parameters);
    nw_release(endpoint);
    if (!conn) {
        if (out_status) *out_status = NW_CONNECT_FAILED;
        return NULL;
    }

    nw_conn_handle *h = (nw_conn_handle *)calloc(1, sizeof(nw_conn_handle));
    h->conn = conn;
    h->queue = dispatch_queue_create("networkframework-rs.conn.params", DISPATCH_QUEUE_SERIAL);
    h->ready = dispatch_semaphore_create(0);
    atomic_store(&h->state_code, 0);

    nw_connection_set_queue(conn, h->queue);
    nw_connection_set_state_changed_handler(conn, ^(nw_connection_state_t state, nw_error_t error) {
        (void)error;
        if (state == nw_connection_state_ready) {
            atomic_store(&h->state_code, 1);
            dispatch_semaphore_signal(h->ready);
        } else if (state == nw_connection_state_cancelled) {
            atomic_store(&h->state_code, 2);
            dispatch_semaphore_signal(h->ready);
        } else if (state == nw_connection_state_failed) {
            atomic_store(&h->state_code, 3);
            dispatch_semaphore_signal(h->ready);
        }
    });

    nw_connection_start(conn);

    dispatch_time_t deadline = dispatch_time(DISPATCH_TIME_NOW, 30LL * NSEC_PER_SEC);
    if (dispatch_semaphore_wait(h->ready, deadline) != 0) {
        cancel_and_destroy_handle(h);
        if (out_status) *out_status = NW_TIMEOUT;
        return NULL;
    }

    if (atomic_load(&h->state_code) != 1) {
        cancel_and_destroy_handle(h);
        if (out_status) *out_status = NW_CONNECT_FAILED;
        return NULL;
    }

    if (out_status) *out_status = NW_OK;
    return h;
}

void *nw_shim_test_copy_failed_connection_error(const char *host, uint16_t port, int use_tls) {
    if (!host) {
        return NULL;
    }

    nw_endpoint_t endpoint = nw_shim_create_host_endpoint(host, port);
    if (!endpoint) {
        return NULL;
    }

    nw_parameters_t parameters = nw_parameters_create_secure_tcp(
        use_tls ? NW_PARAMETERS_DEFAULT_CONFIGURATION : NW_PARAMETERS_DISABLE_PROTOCOL,
        NW_PARAMETERS_DEFAULT_CONFIGURATION);
    if (!parameters) {
        nw_release(endpoint);
        return NULL;
    }

    nw_connection_t connection = nw_connection_create(endpoint, parameters);
    nw_release(parameters);
    nw_release(endpoint);
    if (!connection) {
        return NULL;
    }

    dispatch_queue_t queue = dispatch_queue_create("networkframework-rs.test.error", DISPATCH_QUEUE_SERIAL);
    dispatch_semaphore_t ready = dispatch_semaphore_create(0);
    __block void *retained_error = NULL;
    __block int finished = 0;

    nw_connection_set_queue(connection, queue);
    nw_connection_set_state_changed_handler(connection, ^(nw_connection_state_t state, nw_error_t error) {
        if (!retained_error && error) {
            retained_error = nw_retain(error);
        }
        if (finished) {
            return;
        }
        if (retained_error || state == nw_connection_state_ready || state == nw_connection_state_cancelled || state == nw_connection_state_failed || state == nw_connection_state_waiting) {
            finished = 1;
            dispatch_semaphore_signal(ready);
        }
    });
    nw_connection_start(connection);

    dispatch_time_t deadline = dispatch_time(DISPATCH_TIME_NOW, 10LL * NSEC_PER_SEC);
    if (dispatch_semaphore_wait(ready, deadline) != 0) {
        nw_connection_cancel(connection);
    }
    dispatch_sync(queue, ^{});
    nw_release(connection);
    dispatch_release(ready);
    dispatch_release(queue);
    return retained_error;
}

void *nw_shim_listener_create_with_parameters(void *parameters, uint16_t port, int *out_status) {
    if (!parameters) {
        if (out_status) *out_status = NW_INVALID_ARG;
        return NULL;
    }

    char port_str[8];
    snprintf(port_str, sizeof(port_str), "%u", (unsigned)port);

    nw_listener_t listener = nw_listener_create_with_port(port_str, (nw_parameters_t)parameters);
    if (!listener) {
        if (out_status) *out_status = NW_LISTEN_FAILED;
        return NULL;
    }

    nw_listener_handle *h = (nw_listener_handle *)calloc(1, sizeof(nw_listener_handle));
    h->listener = listener;
    h->queue = dispatch_queue_create("networkframework-rs.listener.params", DISPATCH_QUEUE_SERIAL);
    h->ready = dispatch_semaphore_create(0);
    h->accept_sem = dispatch_semaphore_create(0);
    atomic_store(&h->state_code, 0);

    nw_listener_set_queue(listener, h->queue);
    nw_listener_set_state_changed_handler(listener, ^(nw_listener_state_t state, nw_error_t error) {
        (void)error;
        if (state == nw_listener_state_ready) {
            atomic_store(&h->bound_port, nw_listener_get_port(listener));
            atomic_store(&h->state_code, 1);
            dispatch_semaphore_signal(h->ready);
        } else if (state == nw_listener_state_cancelled) {
            atomic_store(&h->state_code, 2);
            dispatch_semaphore_signal(h->ready);
        } else if (state == nw_listener_state_failed) {
            atomic_store(&h->state_code, 3);
            dispatch_semaphore_signal(h->ready);
        }
    });

    nw_listener_set_new_connection_handler(listener, ^(nw_connection_t conn) {
        nw_retain(conn);
        if (h->pending) {
            nw_connection_cancel(h->pending);
            nw_release(h->pending);
        }
        h->pending = conn;
        dispatch_semaphore_signal(h->accept_sem);
    });

    nw_listener_start(listener);

    dispatch_time_t deadline = dispatch_time(DISPATCH_TIME_NOW, 10LL * NSEC_PER_SEC);
    if (dispatch_semaphore_wait(h->ready, deadline) != 0 || atomic_load(&h->state_code) != 1) {
        cancel_and_destroy_listener_handle(h);
        if (out_status) *out_status = NW_LISTEN_FAILED;
        return NULL;
    }

    if (out_status) *out_status = NW_OK;
    return h;
}

// ---------------------------------------------------------------------
// Content contexts + send/receive helpers
// ---------------------------------------------------------------------

void *nw_shim_content_context_create(const char *identifier) {
    if (!identifier) {
        return NULL;
    }
    return nw_content_context_create(identifier);
}

const char *nw_shim_content_context_get_identifier(void *context) {
    return context ? nw_content_context_get_identifier((nw_content_context_t)context) : NULL;
}

int nw_shim_content_context_get_is_final(void *context) {
    return context && nw_content_context_get_is_final((nw_content_context_t)context) ? 1 : 0;
}

void nw_shim_content_context_set_is_final(void *context, int is_final) {
    if (!context) {
        return;
    }
    nw_content_context_set_is_final((nw_content_context_t)context, is_final != 0);
}

uint64_t nw_shim_content_context_get_expiration_milliseconds(void *context) {
    return context ? nw_content_context_get_expiration_milliseconds((nw_content_context_t)context) : 0;
}

void nw_shim_content_context_set_expiration_milliseconds(void *context, uint64_t expiration_milliseconds) {
    if (!context) {
        return;
    }
    nw_content_context_set_expiration_milliseconds((nw_content_context_t)context, expiration_milliseconds);
}

double nw_shim_content_context_get_relative_priority(void *context) {
    return context ? nw_content_context_get_relative_priority((nw_content_context_t)context) : 0.5;
}

void nw_shim_content_context_set_relative_priority(void *context, double relative_priority) {
    if (!context) {
        return;
    }
    nw_content_context_set_relative_priority((nw_content_context_t)context, relative_priority);
}

void nw_shim_content_context_set_antecedent(void *context, void *antecedent) {
    if (!context) {
        return;
    }
    nw_content_context_set_antecedent((nw_content_context_t)context, (nw_content_context_t)antecedent);
}

void *nw_shim_content_context_copy_antecedent(void *context) {
    if (!context) {
        return NULL;
    }
    return nw_content_context_copy_antecedent((nw_content_context_t)context);
}

void nw_shim_content_context_set_protocol_metadata(void *context, void *metadata) {
    if (!context || !metadata) {
        return;
    }
    nw_content_context_set_metadata_for_protocol((nw_content_context_t)context, (nw_protocol_metadata_t)metadata);
}

void *nw_shim_content_context_copy_protocol_metadata_for_options(void *context, void *protocol_options) {
    if (!context || !protocol_options) {
        return NULL;
    }

    nw_protocol_definition_t definition = nw_protocol_options_copy_definition((nw_protocol_options_t)protocol_options);
    if (!definition) {
        return NULL;
    }
    nw_protocol_metadata_t metadata = nw_content_context_copy_protocol_metadata((nw_content_context_t)context, definition);
    nw_release(definition);
    return metadata;
}

int nw_shim_connection_send_with_context(void *handle, const uint8_t *data, size_t len, void *context) {
    nw_conn_handle *h = (nw_conn_handle *)handle;
    if (!h || (!data && len != 0)) {
        return NW_INVALID_ARG;
    }

    dispatch_data_t payload = dispatch_data_create(
        data, len, h->queue, DISPATCH_DATA_DESTRUCTOR_DEFAULT);

    __block int result = NW_OK;
    dispatch_semaphore_t done = dispatch_semaphore_create(0);

    nw_connection_send(h->conn, payload, (nw_content_context_t)context, true, ^(nw_error_t error) {
        if (error) {
            result = NW_SEND_FAILED;
        }
        dispatch_semaphore_signal(done);
    });

    dispatch_semaphore_wait(done, DISPATCH_TIME_FOREVER);
    dispatch_release(done);
    dispatch_release(payload);
    return result;
}

ssize_t nw_shim_connection_receive_with_context(
    void *handle,
    uint8_t *out_buf,
    size_t max_len,
    void **out_context,
    int *out_is_complete
) {
    nw_conn_handle *h = (nw_conn_handle *)handle;
    if (!h || !out_buf || max_len == 0) {
        return NW_INVALID_ARG;
    }
    if (out_context) {
        *out_context = NULL;
    }
    if (out_is_complete) {
        *out_is_complete = 0;
    }

    __block ssize_t result = 0;
    __block void *retained_context = NULL;
    __block int is_complete_result = 0;
    dispatch_semaphore_t done = dispatch_semaphore_create(0);

    nw_connection_receive(h->conn, 1, (uint32_t)max_len,
        ^(dispatch_data_t content, nw_content_context_t ctx, bool is_complete, nw_error_t error) {
            if (error) {
                result = NW_RECV_FAILED;
            } else {
                is_complete_result = is_complete ? 1 : 0;
                if (ctx) {
                    retained_context = nw_retain(ctx);
                }
                if (content) {
                    __block size_t copied = 0;
                    dispatch_data_apply(content,
                        ^bool(dispatch_data_t region, size_t offset, const void *buffer, size_t size) {
                            (void)region;
                            (void)offset;
                            size_t can = max_len - copied;
                            if (can == 0) return false;
                            size_t take = size < can ? size : can;
                            memcpy(out_buf + copied, buffer, take);
                            copied += take;
                            return true;
                        });
                    result = (ssize_t)copied;
                }
            }
            dispatch_semaphore_signal(done);
        });

    dispatch_semaphore_wait(done, DISPATCH_TIME_FOREVER);
    dispatch_release(done);

    if (result >= 0) {
        if (out_context) {
            *out_context = retained_context;
            retained_context = NULL;
        } else if (retained_context) {
            nw_release((nw_content_context_t)retained_context);
        }
        if (out_is_complete) {
            *out_is_complete = is_complete_result;
        }
    } else if (retained_context) {
        nw_release((nw_content_context_t)retained_context);
    }

    return result;
}

// ---------------------------------------------------------------------
// Interface enumeration
// ---------------------------------------------------------------------

static int nw_shim_enumerate_path_interfaces(
    nw_path_t path,
    int (*callback)(const char *name, int interface_type, uint32_t index, void *user_info),
    void *user_info
) {
    if (!path || !callback) {
        return 0;
    }

    __block int count = 0;
    nw_path_enumerate_interfaces(path, ^bool(nw_interface_t interface) {
        const char *name = nw_interface_get_name(interface);
        int keep_going = callback(
            name ? name : "",
            (int)nw_interface_get_type(interface),
            nw_interface_get_index(interface),
            user_info);
        count += 1;
        return keep_going != 0;
    });
    return count;
}

int nw_shim_path_monitor_enumerate_interfaces(
    void *handle,
    int (*callback)(const char *name, int interface_type, uint32_t index, void *user_info),
    void *user_info
) {
    nw_path_handle *h = (nw_path_handle *)handle;
    if (!h || !callback) {
        return 0;
    }

    __block nw_path_t snapshot = NULL;
    dispatch_sync(h->queue, ^{
        if (h->latest_path) {
            snapshot = nw_retain(h->latest_path);
        }
    });

    int count = nw_shim_enumerate_path_interfaces(snapshot, callback, user_info);
    if (snapshot) {
        nw_release(snapshot);
    }
    return count;
}

int nw_shim_list_interfaces(
    int (*callback)(const char *name, int interface_type, uint32_t index, void *user_info),
    void *user_info
) {
    if (!callback) {
        return 0;
    }

    __block nw_path_t captured_path = NULL;
    dispatch_queue_t queue = dispatch_queue_create("networkframework-rs.interface-list", DISPATCH_QUEUE_SERIAL);
    dispatch_semaphore_t ready = dispatch_semaphore_create(0);
    nw_path_monitor_t monitor = nw_path_monitor_create();
    if (!monitor) {
        dispatch_release(queue);
        dispatch_release(ready);
        return 0;
    }

    nw_path_monitor_set_queue(monitor, queue);
    nw_path_monitor_set_update_handler(monitor, ^(nw_path_t path) {
        if (!captured_path && path) {
            captured_path = nw_retain(path);
            dispatch_semaphore_signal(ready);
        }
    });
    nw_path_monitor_start(monitor);

    dispatch_time_t deadline = dispatch_time(DISPATCH_TIME_NOW, 5LL * NSEC_PER_SEC);
    if (dispatch_semaphore_wait(ready, deadline) != 0) {
        nw_path_monitor_cancel(monitor);
        nw_release(monitor);
        dispatch_release(queue);
        dispatch_release(ready);
        return 0;
    }

    int count = nw_shim_enumerate_path_interfaces(captured_path, callback, user_info);

    nw_path_monitor_cancel(monitor);
    if (captured_path) {
        nw_release(captured_path);
    }
    nw_release(monitor);
    dispatch_release(queue);
    dispatch_release(ready);
    return count;
}

// ---------------------------------------------------------------------
// Privacy contexts, proxy configs, resolver configs
// ---------------------------------------------------------------------

void *nw_shim_privacy_context_create(const char *description) {
    if (!description) {
        return NULL;
    }
    if (__builtin_available(macOS 11.0, *)) {
        return nw_privacy_context_create(description);
    }
    return NULL;
}

void nw_shim_privacy_context_flush_cache(void *privacy_context) {
    if (!privacy_context) {
        return;
    }
    if (__builtin_available(macOS 11.0, *)) {
        nw_privacy_context_flush_cache((nw_privacy_context_t)privacy_context);
    }
}

void nw_shim_privacy_context_disable_logging(void *privacy_context) {
    if (!privacy_context) {
        return;
    }
    if (__builtin_available(macOS 11.0, *)) {
        nw_privacy_context_disable_logging((nw_privacy_context_t)privacy_context);
    }
}

void nw_shim_privacy_context_require_encrypted_name_resolution(
    void *privacy_context,
    int require_encrypted_name_resolution,
    void *fallback_resolver_config
) {
    if (!privacy_context) {
        return;
    }
    if (__builtin_available(macOS 11.0, *)) {
        nw_privacy_context_require_encrypted_name_resolution(
            (nw_privacy_context_t)privacy_context,
            require_encrypted_name_resolution != 0,
            (nw_resolver_config_t)fallback_resolver_config);
    }
}

void nw_shim_privacy_context_add_proxy(void *privacy_context, void *proxy_config) {
    if (!privacy_context || !proxy_config) {
        return;
    }
    if (__builtin_available(macOS 14.0, *)) {
        nw_privacy_context_add_proxy((nw_privacy_context_t)privacy_context, (nw_proxy_config_t)proxy_config);
    }
}

void nw_shim_privacy_context_clear_proxies(void *privacy_context) {
    if (!privacy_context) {
        return;
    }
    if (__builtin_available(macOS 14.0, *)) {
        nw_privacy_context_clear_proxies((nw_privacy_context_t)privacy_context);
    }
}

void *nw_shim_resolver_config_create_https(const char *url) {
    if (!url) {
        return NULL;
    }
    if (__builtin_available(macOS 11.0, *)) {
        nw_endpoint_t endpoint = nw_endpoint_create_url(url);
        if (!endpoint) {
            return NULL;
        }
        nw_resolver_config_t config = nw_resolver_config_create_https(endpoint);
        nw_release(endpoint);
        return config;
    }
    return NULL;
}

void *nw_shim_resolver_config_create_tls(const char *host, uint16_t port) {
    if (!host) {
        return NULL;
    }
    if (__builtin_available(macOS 11.0, *)) {
        nw_endpoint_t endpoint = nw_shim_create_host_endpoint(host, port);
        if (!endpoint) {
            return NULL;
        }
        nw_resolver_config_t config = nw_resolver_config_create_tls(endpoint);
        nw_release(endpoint);
        return config;
    }
    return NULL;
}

int nw_shim_resolver_config_add_server_address(void *resolver_config, const char *address, uint16_t port) {
    if (!resolver_config || !address) {
        return NW_INVALID_ARG;
    }
    if (__builtin_available(macOS 11.0, *)) {
        nw_endpoint_t endpoint = nw_shim_create_host_endpoint(address, port);
        if (!endpoint) {
            return NW_INVALID_ARG;
        }
        nw_resolver_config_add_server_address((nw_resolver_config_t)resolver_config, endpoint);
        nw_release(endpoint);
        return NW_OK;
    }
    return NW_INVALID_ARG;
}

void *nw_shim_proxy_config_create_http_connect(const char *host, uint16_t port, int use_tls) {
    if (!host) {
        return NULL;
    }
    if (__builtin_available(macOS 14.0, *)) {
        nw_endpoint_t endpoint = nw_shim_create_host_endpoint(host, port);
        if (!endpoint) {
            return NULL;
        }
        nw_protocol_options_t tls_options = NULL;
        if (use_tls) {
            tls_options = nw_tls_create_options();
        }
        nw_proxy_config_t config = nw_proxy_config_create_http_connect(endpoint, tls_options);
        if (tls_options) {
            nw_release(tls_options);
        }
        nw_release(endpoint);
        return config;
    }
    return NULL;
}

void *nw_shim_proxy_config_create_socksv5(const char *host, uint16_t port) {
    if (!host) {
        return NULL;
    }
    if (__builtin_available(macOS 14.0, *)) {
        nw_endpoint_t endpoint = nw_shim_create_host_endpoint(host, port);
        if (!endpoint) {
            return NULL;
        }
        nw_proxy_config_t config = nw_proxy_config_create_socksv5(endpoint);
        nw_release(endpoint);
        return config;
    }
    return NULL;
}

void nw_shim_proxy_config_set_username_password(void *proxy_config, const char *username, const char *password) {
    if (!proxy_config || !username) {
        return;
    }
    if (__builtin_available(macOS 14.0, *)) {
        nw_proxy_config_set_username_and_password((nw_proxy_config_t)proxy_config, username, password);
    }
}

void nw_shim_proxy_config_set_failover_allowed(void *proxy_config, int failover_allowed) {
    if (!proxy_config) {
        return;
    }
    if (__builtin_available(macOS 14.0, *)) {
        nw_proxy_config_set_failover_allowed((nw_proxy_config_t)proxy_config, failover_allowed != 0);
    }
}

int nw_shim_proxy_config_get_failover_allowed(void *proxy_config) {
    if (!proxy_config) {
        return 0;
    }
    if (__builtin_available(macOS 14.0, *)) {
        return nw_proxy_config_get_failover_allowed((nw_proxy_config_t)proxy_config) ? 1 : 0;
    }
    return 0;
}

void nw_shim_proxy_config_add_match_domain(void *proxy_config, const char *domain) {
    if (!proxy_config || !domain) {
        return;
    }
    if (__builtin_available(macOS 14.0, *)) {
        nw_proxy_config_add_match_domain((nw_proxy_config_t)proxy_config, domain);
    }
}

void nw_shim_proxy_config_clear_match_domains(void *proxy_config) {
    if (!proxy_config) {
        return;
    }
    if (__builtin_available(macOS 14.0, *)) {
        nw_proxy_config_clear_match_domains((nw_proxy_config_t)proxy_config);
    }
}

void nw_shim_proxy_config_add_excluded_domain(void *proxy_config, const char *domain) {
    if (!proxy_config || !domain) {
        return;
    }
    if (__builtin_available(macOS 14.0, *)) {
        nw_proxy_config_add_excluded_domain((nw_proxy_config_t)proxy_config, domain);
    }
}

void nw_shim_proxy_config_clear_excluded_domains(void *proxy_config) {
    if (!proxy_config) {
        return;
    }
    if (__builtin_available(macOS 14.0, *)) {
        nw_proxy_config_clear_excluded_domains((nw_proxy_config_t)proxy_config);
    }
}

// ---------------------------------------------------------------------
// Framer definitions, messages, and actions
// ---------------------------------------------------------------------

typedef void *(*nw_shim_framer_create_instance_fn)(void *user_info);
typedef void (*nw_shim_framer_drop_instance_fn)(void *instance);
typedef int (*nw_shim_framer_start_fn)(void *instance, void *framer);
typedef size_t (*nw_shim_framer_input_fn)(void *instance, void *framer);
typedef void (*nw_shim_framer_output_fn)(
    void *instance,
    void *framer,
    void *message,
    size_t message_length,
    int is_complete);
typedef void (*nw_shim_framer_wakeup_fn)(void *instance, void *framer);
typedef int (*nw_shim_framer_stop_fn)(void *instance, void *framer);
typedef void (*nw_shim_framer_cleanup_fn)(void *instance, void *framer);
typedef size_t (*nw_shim_framer_parse_fn)(
    const uint8_t *buffer,
    size_t buffer_length,
    int is_complete,
    void *user_info);
typedef void (*nw_shim_framer_async_fn)(void *framer, void *user_info);

void *nw_shim_framer_definition_create(
    const char *identifier,
    uint32_t flags,
    nw_shim_framer_create_instance_fn create_instance,
    nw_shim_framer_drop_instance_fn drop_instance,
    nw_shim_framer_start_fn start_callback,
    nw_shim_framer_input_fn input_callback,
    nw_shim_framer_output_fn output_callback,
    nw_shim_framer_wakeup_fn wakeup_callback,
    nw_shim_framer_stop_fn stop_callback,
    nw_shim_framer_cleanup_fn cleanup_callback,
    void *user_info
) {
    if (!identifier || !create_instance || !start_callback || !input_callback || !output_callback) {
        return NULL;
    }
    if (__builtin_available(macOS 10.15, *)) {
        nw_protocol_definition_t definition = nw_framer_create_definition(
            identifier,
            flags,
            ^nw_framer_start_result_t(nw_framer_t framer) {
                void *instance = create_instance(user_info);
                nw_framer_set_input_handler(framer, ^size_t(nw_framer_t inner_framer) {
                    return input_callback(instance, inner_framer);
                });
                nw_framer_set_output_handler(
                    framer,
                    ^(nw_framer_t inner_framer, nw_framer_message_t message, size_t message_length, bool is_complete) {
                        output_callback(
                            instance,
                            inner_framer,
                            message,
                            message_length,
                            is_complete ? 1 : 0);
                    });
                if (wakeup_callback) {
                    nw_framer_set_wakeup_handler(framer, ^(nw_framer_t inner_framer) {
                        wakeup_callback(instance, inner_framer);
                    });
                }
                if (stop_callback) {
                    nw_framer_set_stop_handler(framer, ^bool(nw_framer_t inner_framer) {
                        return stop_callback(instance, inner_framer) != 0;
                    });
                }
                nw_framer_set_cleanup_handler(framer, ^(nw_framer_t inner_framer) {
                    if (cleanup_callback) {
                        cleanup_callback(instance, inner_framer);
                    }
                    if (drop_instance) {
                        drop_instance(instance);
                    }
                });

                int start_result = start_callback(instance, framer);
                return start_result == (int)nw_framer_start_result_will_mark_ready
                    ? nw_framer_start_result_will_mark_ready
                    : nw_framer_start_result_ready;
            });
        return definition;
    }
    return NULL;
}

void *nw_shim_framer_create_options(void *definition) {
    if (!definition) {
        return NULL;
    }
    if (__builtin_available(macOS 10.15, *)) {
        return nw_framer_create_options((nw_protocol_definition_t)definition);
    }
    return NULL;
}

void *nw_shim_framer_message_create_from_options(void *protocol_options) {
    if (!protocol_options) {
        return NULL;
    }
    if (__builtin_available(macOS 10.15, *)) {
        nw_protocol_definition_t definition = nw_protocol_options_copy_definition((nw_protocol_options_t)protocol_options);
        if (!definition) {
            return NULL;
        }
        nw_framer_message_t message = nw_framer_protocol_create_message(definition);
        nw_release(definition);
        return message;
    }
    return NULL;
}

void *nw_shim_framer_message_create_for_instance(void *framer) {
    if (!framer) {
        return NULL;
    }
    if (__builtin_available(macOS 10.15, *)) {
        return nw_framer_message_create((nw_framer_t)framer);
    }
    return NULL;
}

int nw_shim_framer_message_set_u64(void *message, const char *key, uint64_t value) {
    if (!message || !key) {
        return NW_INVALID_ARG;
    }
    if (__builtin_available(macOS 10.15, *)) {
        uint64_t *stored = (uint64_t *)malloc(sizeof(uint64_t));
        if (!stored) {
            return NW_INVALID_ARG;
        }
        *stored = value;
        nw_framer_message_set_value((nw_framer_message_t)message, key, stored, ^(void *value_ptr) {
            free(value_ptr);
        });
        return NW_OK;
    }
    return NW_INVALID_ARG;
}

int nw_shim_framer_message_get_u64(void *message, const char *key, uint64_t *out_value) {
    if (!message || !key || !out_value) {
        return NW_INVALID_ARG;
    }
    if (__builtin_available(macOS 10.15, *)) {
        if (!nw_protocol_metadata_is_framer_message((nw_protocol_metadata_t)message)) {
            return 0;
        }

        __block int found = 0;
        nw_framer_message_access_value((nw_framer_message_t)message, key, ^bool(const void *value_ptr) {
            if (value_ptr) {
                *out_value = *(const uint64_t *)value_ptr;
                found = 1;
            }
            return false;
        });
        return found;
    }
    return NW_INVALID_ARG;
}

int nw_shim_framer_parse_input(
    void *framer,
    size_t minimum_incomplete_length,
    size_t maximum_length,
    uint8_t *temp_buffer,
    nw_shim_framer_parse_fn parse_callback,
    void *user_info
) {
    if (!framer || !parse_callback) {
        return 0;
    }
    if (__builtin_available(macOS 10.15, *)) {
        return nw_framer_parse_input(
            (nw_framer_t)framer,
            minimum_incomplete_length,
            maximum_length,
            temp_buffer,
            ^size_t(uint8_t *buffer, size_t buffer_length, bool is_complete) {
                return parse_callback(buffer, buffer_length, is_complete ? 1 : 0, user_info);
            }) ? 1 : 0;
    }
    return 0;
}

int nw_shim_framer_parse_output(
    void *framer,
    size_t minimum_incomplete_length,
    size_t maximum_length,
    uint8_t *temp_buffer,
    nw_shim_framer_parse_fn parse_callback,
    void *user_info
) {
    if (!framer || !parse_callback) {
        return 0;
    }
    if (__builtin_available(macOS 10.15, *)) {
        return nw_framer_parse_output(
            (nw_framer_t)framer,
            minimum_incomplete_length,
            maximum_length,
            temp_buffer,
            ^size_t(uint8_t *buffer, size_t buffer_length, bool is_complete) {
                return parse_callback(buffer, buffer_length, is_complete ? 1 : 0, user_info);
            }) ? 1 : 0;
    }
    return 0;
}

void nw_shim_framer_mark_ready(void *framer) {
    if (!framer) {
        return;
    }
    if (__builtin_available(macOS 10.15, *)) {
        nw_framer_mark_ready((nw_framer_t)framer);
    }
}

int nw_shim_framer_prepend_application_protocol(void *framer, void *protocol_options) {
    if (!framer || !protocol_options) {
        return 0;
    }
    if (__builtin_available(macOS 10.15, *)) {
        return nw_framer_prepend_application_protocol((nw_framer_t)framer, (nw_protocol_options_t)protocol_options) ? 1 : 0;
    }
    return 0;
}

void nw_shim_framer_mark_failed_with_error(void *framer, int error_code) {
    if (!framer) {
        return;
    }
    if (__builtin_available(macOS 10.15, *)) {
        nw_framer_mark_failed_with_error((nw_framer_t)framer, error_code);
    }
}

int nw_shim_framer_pass_input_data(void *framer, size_t input_length, void *message, int is_complete) {
    if (!framer) {
        return 0;
    }
    if (__builtin_available(macOS 10.15, *)) {
        nw_framer_message_t delivered_message = NULL;
        if (message) {
            delivered_message = nw_retain((nw_framer_message_t)message);
        }
        return nw_framer_deliver_input_no_copy(
            (nw_framer_t)framer,
            input_length,
            delivered_message,
            is_complete != 0) ? 1 : 0;
    }
    return 0;
}

void nw_shim_framer_deliver_input_data(
    void *framer,
    const uint8_t *input_buffer,
    size_t input_length,
    void *message,
    int is_complete
) {
    if (!framer) {
        return;
    }
    if (__builtin_available(macOS 10.15, *)) {
        nw_framer_message_t delivered_message = NULL;
        if (message) {
            delivered_message = nw_retain((nw_framer_message_t)message);
        }
        if (input_length == 0) {
            nw_framer_deliver_input_no_copy((nw_framer_t)framer, 0, delivered_message, is_complete != 0);
            return;
        }
        nw_framer_deliver_input(
            (nw_framer_t)framer,
            input_buffer,
            input_length,
            delivered_message,
            is_complete != 0);
    }
}

void nw_shim_framer_pass_through_input(void *framer) {
    if (!framer) {
        return;
    }
    if (__builtin_available(macOS 10.15, *)) {
        nw_framer_pass_through_input((nw_framer_t)framer);
    }
}

int nw_shim_framer_pass_output_data(void *framer, size_t output_length) {
    if (!framer) {
        return 0;
    }
    if (__builtin_available(macOS 10.15, *)) {
        return nw_framer_write_output_no_copy((nw_framer_t)framer, output_length) ? 1 : 0;
    }
    return 0;
}

void nw_shim_framer_write_output_data(void *framer, const uint8_t *output_buffer, size_t output_length) {
    if (!framer) {
        return;
    }
    if (__builtin_available(macOS 10.15, *)) {
        if (output_length == 0) {
            return;
        }
        nw_framer_write_output((nw_framer_t)framer, output_buffer, output_length);
    }
}

void nw_shim_framer_pass_through_output(void *framer) {
    if (!framer) {
        return;
    }
    if (__builtin_available(macOS 10.15, *)) {
        nw_framer_pass_through_output((nw_framer_t)framer);
    }
}

void nw_shim_framer_schedule_wakeup(void *framer, uint64_t milliseconds) {
    if (!framer) {
        return;
    }
    if (__builtin_available(macOS 10.15, *)) {
        nw_framer_schedule_wakeup((nw_framer_t)framer, milliseconds);
    }
}

void nw_shim_framer_async(void *framer, nw_shim_framer_async_fn async_callback, void *user_info) {
    if (!framer || !async_callback) {
        return;
    }
    if (__builtin_available(macOS 10.15, *)) {
        nw_framer_async((nw_framer_t)framer, ^{
            async_callback(framer, user_info);
        });
    }
}

// ---------------------------------------------------------------------
// Group descriptors + connection groups
// ---------------------------------------------------------------------

typedef void (*nw_shim_connection_group_state_fn)(int state, void *user_info);
typedef void (*nw_shim_connection_group_receive_fn)(
    const uint8_t *data,
    size_t len,
    void *context,
    int is_complete,
    void *user_info);

typedef struct nw_connection_group_handle {
    nw_connection_group_t group;
    dispatch_queue_t queue;
    dispatch_semaphore_t state_sem;
    _Atomic int state_code;
    nw_shim_connection_group_state_fn state_callback;
    void *state_user_info;
    nw_shim_connection_group_receive_fn receive_callback;
    void *receive_user_info;
} nw_connection_group_handle;

void *nw_shim_group_descriptor_create_multiplex(const char *host, uint16_t port) {
    if (!host) {
        return NULL;
    }
    if (__builtin_available(macOS 12.0, *)) {
        nw_endpoint_t endpoint = nw_shim_create_host_endpoint(host, port);
        if (!endpoint) {
            return NULL;
        }
        nw_group_descriptor_t descriptor = nw_group_descriptor_create_multiplex(endpoint);
        nw_release(endpoint);
        return descriptor;
    }
    return NULL;
}

void *nw_shim_group_descriptor_create_multicast(const char *group_address, uint16_t port) {
    if (!group_address) {
        return NULL;
    }
    if (__builtin_available(macOS 11.0, *)) {
        nw_endpoint_t endpoint = nw_shim_create_address_endpoint(group_address, port);
        if (!endpoint) {
            return NULL;
        }
        nw_group_descriptor_t descriptor = nw_group_descriptor_create_multicast(endpoint);
        nw_release(endpoint);
        return descriptor;
    }
    return NULL;
}

int nw_shim_group_descriptor_add_endpoint(void *descriptor, const char *host, uint16_t port) {
    if (!descriptor || !host) {
        return 0;
    }

    nw_endpoint_t endpoint = nw_shim_create_host_endpoint(host, port);
    if (!endpoint) {
        return 0;
    }

    bool added = nw_group_descriptor_add_endpoint((nw_group_descriptor_t)descriptor, endpoint);
    nw_release(endpoint);
    return added ? 1 : 0;
}

void *nw_shim_connection_group_create(void *descriptor, void *parameters) {
    if (!descriptor || !parameters) {
        return NULL;
    }
    if (__builtin_available(macOS 11.0, *)) {
        nw_connection_group_t group = nw_connection_group_create(
            (nw_group_descriptor_t)descriptor,
            (nw_parameters_t)parameters);
        if (!group) {
            return NULL;
        }

        nw_connection_group_handle *h = (nw_connection_group_handle *)calloc(1, sizeof(nw_connection_group_handle));
        h->group = group;
        h->queue = dispatch_queue_create("networkframework-rs.connection-group", DISPATCH_QUEUE_SERIAL);
        h->state_sem = dispatch_semaphore_create(0);
        atomic_store(&h->state_code, (int)nw_connection_group_state_invalid);

        nw_connection_group_set_queue(group, h->queue);
        nw_connection_group_set_state_changed_handler(group, ^(nw_connection_group_state_t state, nw_error_t error) {
            (void)error;
            atomic_store(&h->state_code, (int)state);
            dispatch_semaphore_signal(h->state_sem);
            if (h->state_callback) {
                h->state_callback((int)state, h->state_user_info);
            }
        });

        return h;
    }
    return NULL;
}

void nw_shim_connection_group_set_state_changed_handler(
    void *handle,
    nw_shim_connection_group_state_fn state_callback,
    void *user_info
) {
    nw_connection_group_handle *h = (nw_connection_group_handle *)handle;
    if (!h) {
        return;
    }
    h->state_callback = state_callback;
    h->state_user_info = user_info;
}

void nw_shim_connection_group_set_receive_handler(
    void *handle,
    uint32_t maximum_message_size,
    int reject_oversized_messages,
    nw_shim_connection_group_receive_fn receive_callback,
    void *user_info
) {
    nw_connection_group_handle *h = (nw_connection_group_handle *)handle;
    if (!h) {
        return;
    }
    h->receive_callback = receive_callback;
    h->receive_user_info = user_info;

    if (!receive_callback) {
        nw_connection_group_set_receive_handler(h->group, maximum_message_size, reject_oversized_messages != 0, NULL);
        return;
    }

    nw_connection_group_set_receive_handler(
        h->group,
        maximum_message_size,
        reject_oversized_messages != 0,
        ^(dispatch_data_t content, nw_content_context_t context, bool is_complete) {
            uint8_t *bytes = NULL;
            size_t length = nw_shim_copy_dispatch_data(content, &bytes);
            void *retained_context = context ? nw_retain(context) : NULL;
            receive_callback(bytes, length, retained_context, is_complete ? 1 : 0, h->receive_user_info);
            if (bytes) {
                free(bytes);
            }
        });
}

int nw_shim_connection_group_start(void *handle) {
    nw_connection_group_handle *h = (nw_connection_group_handle *)handle;
    if (!h) {
        return NW_INVALID_ARG;
    }

    nw_connection_group_start(h->group);

    if (atomic_load(&h->state_code) == (int)nw_connection_group_state_invalid) {
        dispatch_time_t deadline = dispatch_time(DISPATCH_TIME_NOW, 10LL * NSEC_PER_SEC);
        if (dispatch_semaphore_wait(h->state_sem, deadline) != 0) {
            return NW_TIMEOUT;
        }
    }

    int state = atomic_load(&h->state_code);
    if (state == (int)nw_connection_group_state_failed || state == (int)nw_connection_group_state_cancelled) {
        return NW_CONNECT_FAILED;
    }
    return NW_OK;
}

void nw_shim_connection_group_cancel(void *handle) {
    nw_connection_group_handle *h = (nw_connection_group_handle *)handle;
    if (!h) {
        return;
    }
    nw_connection_group_cancel(h->group);
}

int nw_shim_connection_group_send(
    void *handle,
    const uint8_t *data,
    size_t len,
    const char *host,
    uint16_t port,
    void *context
) {
    nw_connection_group_handle *h = (nw_connection_group_handle *)handle;
    if (!h || (!data && len != 0) || !context) {
        return NW_INVALID_ARG;
    }

    dispatch_data_t payload = NULL;
    if (data || len != 0) {
        payload = dispatch_data_create(data, len, h->queue, DISPATCH_DATA_DESTRUCTOR_DEFAULT);
    }

    nw_endpoint_t endpoint = NULL;
    if (host && host[0]) {
        endpoint = nw_shim_create_host_endpoint(host, port);
        if (!endpoint) {
            if (payload) {
                dispatch_release(payload);
            }
            return NW_INVALID_ARG;
        }
    }

    __block int result = NW_OK;
    dispatch_semaphore_t done = dispatch_semaphore_create(0);
    nw_connection_group_send_message(
        h->group,
        payload,
        endpoint,
        (nw_content_context_t)context,
        ^(nw_error_t error) {
            if (error) {
                result = NW_SEND_FAILED;
            }
            dispatch_semaphore_signal(done);
        });

    dispatch_semaphore_wait(done, DISPATCH_TIME_FOREVER);
    dispatch_release(done);
    if (payload) {
        dispatch_release(payload);
    }
    if (endpoint) {
        nw_release(endpoint);
    }
    return result;
}

void nw_shim_connection_group_release(void *handle) {
    nw_connection_group_handle *h = (nw_connection_group_handle *)handle;
    if (!h) {
        return;
    }

    nw_connection_group_cancel(h->group);
    if (atomic_load(&h->state_code) != (int)nw_connection_group_state_cancelled) {
        dispatch_time_t deadline = dispatch_time(DISPATCH_TIME_NOW, 5LL * NSEC_PER_SEC);
        (void)dispatch_semaphore_wait(h->state_sem, deadline);
    }

    nw_release(h->group);
    dispatch_release(h->queue);
    dispatch_release(h->state_sem);
    free(h);
}

void nw_shim_free_buffer(void *buffer) {
    free(buffer);
}

void *nw_shim_parameters_create(void) {
    return nw_parameters_create();
}

void *nw_shim_parameters_create_application_service(void) {
    if (__builtin_available(macOS 13.0, *)) {
        return nw_parameters_create_application_service();
    }
    return NULL;
}

void nw_shim_parameters_set_attribution(void *parameters, int attribution) {
    if (!parameters) {
        return;
    }
    nw_parameters_set_attribution((nw_parameters_t)parameters, (nw_parameters_attribution_t)attribution);
}

int nw_shim_parameters_get_attribution(void *parameters) {
    if (!parameters) {
        return 0;
    }
    return (int)nw_parameters_get_attribution((nw_parameters_t)parameters);
}

void nw_shim_parameters_set_required_interface_type(void *parameters, int interface_type) {
    if (!parameters) {
        return;
    }
    nw_parameters_set_required_interface_type((nw_parameters_t)parameters, (nw_interface_type_t)interface_type);
}

int nw_shim_parameters_get_required_interface_type(void *parameters) {
    if (!parameters) {
        return 0;
    }
    return (int)nw_parameters_get_required_interface_type((nw_parameters_t)parameters);
}

void nw_shim_parameters_set_prohibit_expensive(void *parameters, int prohibit_expensive) {
    if (!parameters) {
        return;
    }
    nw_parameters_set_prohibit_expensive((nw_parameters_t)parameters, prohibit_expensive != 0);
}

int nw_shim_parameters_get_prohibit_expensive(void *parameters) {
    if (!parameters) {
        return 0;
    }
    return nw_parameters_get_prohibit_expensive((nw_parameters_t)parameters) ? 1 : 0;
}

void nw_shim_parameters_set_prohibit_constrained(void *parameters, int prohibit_constrained) {
    if (!parameters) {
        return;
    }
    nw_parameters_set_prohibit_constrained((nw_parameters_t)parameters, prohibit_constrained != 0);
}

int nw_shim_parameters_get_prohibit_constrained(void *parameters) {
    if (!parameters) {
        return 0;
    }
    return nw_parameters_get_prohibit_constrained((nw_parameters_t)parameters) ? 1 : 0;
}

void nw_shim_parameters_set_allow_ultra_constrained(void *parameters, int allow_ultra_constrained) {
    if (!parameters) {
        return;
    }
    if (__builtin_available(macOS 26.0, *)) {
        nw_parameters_set_allow_ultra_constrained((nw_parameters_t)parameters, allow_ultra_constrained != 0);
    }
}

int nw_shim_parameters_get_allow_ultra_constrained(void *parameters) {
    if (!parameters) {
        return 0;
    }
    if (__builtin_available(macOS 26.0, *)) {
        return nw_parameters_get_allow_ultra_constrained((nw_parameters_t)parameters) ? 1 : 0;
    }
    return 0;
}

void *nw_shim_connection_copy_endpoint(void *handle) {
    nw_conn_handle *h = (nw_conn_handle *)handle;
    if (!h || !h->conn) {
        return NULL;
    }
    return nw_connection_copy_endpoint(h->conn);
}

void *nw_shim_connection_copy_parameters(void *handle) {
    nw_conn_handle *h = (nw_conn_handle *)handle;
    if (!h || !h->conn) {
        return NULL;
    }
    return nw_connection_copy_parameters(h->conn);
}

void *nw_shim_connection_copy_current_path(void *handle) {
    nw_conn_handle *h = (nw_conn_handle *)handle;
    if (!h || !h->conn) {
        return NULL;
    }
    return nw_connection_copy_current_path(h->conn);
}

void *nw_shim_path_monitor_copy_latest_path(void *handle) {
    nw_path_handle *h = (nw_path_handle *)handle;
    if (!h) {
        return NULL;
    }

    __block nw_path_t snapshot = NULL;
    dispatch_sync(h->queue, ^{
        if (h->latest_path) {
            snapshot = nw_retain(h->latest_path);
        }
    });
    return snapshot;
}

void *nw_shim_browser_start_with_descriptor(
    void *descriptor,
    void *parameters,
    void (*found_callback)(const char *name, const char *service_type, const char *domain, void *user_info),
    void (*lost_callback)(const char *name, const char *service_type, const char *domain, void *user_info),
    void *user_info
) {
    if (!descriptor) {
        return NULL;
    }

    nw_browse_descriptor_t desc = nw_retain((nw_browse_descriptor_t)descriptor);
    nw_parameters_t params = parameters ? nw_parameters_copy((nw_parameters_t)parameters) : nw_parameters_create();
    if (!params) {
        nw_release(desc);
        return NULL;
    }

    nw_browser_t browser = nw_browser_create(desc, params);
    nw_release(desc);
    nw_release(params);
    if (!browser) {
        return NULL;
    }

    nw_browser_handle *h = (nw_browser_handle *)calloc(1, sizeof(nw_browser_handle));
    h->browser = browser;
    h->queue = dispatch_queue_create("networkframework-rs.browser", DISPATCH_QUEUE_SERIAL);
    h->found_callback = found_callback;
    h->lost_callback = lost_callback;
    h->user_info = user_info;

    nw_browser_set_queue(browser, h->queue);
    nw_browser_set_browse_results_changed_handler(browser,
        ^(nw_browse_result_t old_result, nw_browse_result_t new_result, bool batch_complete) {
            (void)batch_complete;
            nw_browse_result_t result = new_result ? new_result : old_result;
            if (!result) {
                return;
            }
            nw_endpoint_t ep = nw_browse_result_copy_endpoint(result);
            if (!ep) {
                return;
            }
            const char *name = nw_endpoint_get_bonjour_service_name(ep);
            const char *type = nw_endpoint_get_bonjour_service_type(ep);
            const char *dom = nw_endpoint_get_bonjour_service_domain(ep);
            if (!name || !name[0]) {
                name = nw_endpoint_get_hostname(ep);
            }
            if (new_result && !old_result && h->found_callback) {
                h->found_callback(name ? name : "", type ? type : "", dom ? dom : "", h->user_info);
            } else if (old_result && !new_result && h->lost_callback) {
                h->lost_callback(name ? name : "", type ? type : "", dom ? dom : "", h->user_info);
            }
            nw_release(ep);
        });
    nw_browser_start(browser);
    return h;
}

void *nw_shim_bonjour_advertise_start_with_descriptor(void *descriptor, uint16_t port, int *out_status) {
    if (!descriptor) {
        if (out_status) *out_status = NW_INVALID_ARG;
        return NULL;
    }

    nw_parameters_t params = nw_parameters_create_secure_tcp(
        NW_PARAMETERS_DISABLE_PROTOCOL,
        NW_PARAMETERS_DEFAULT_CONFIGURATION);

    char port_str[8];
    snprintf(port_str, sizeof(port_str), "%u", (unsigned)port);
    nw_listener_t listener = nw_listener_create_with_port(port_str, params);
    nw_release(params);
    if (!listener) {
        if (out_status) *out_status = NW_LISTEN_FAILED;
        return NULL;
    }

    nw_advertise_descriptor_t advertise_descriptor = nw_retain((nw_advertise_descriptor_t)descriptor);
    nw_listener_set_advertise_descriptor(listener, advertise_descriptor);
    nw_release(advertise_descriptor);

    nw_bonjour_advertise_handle *h = (nw_bonjour_advertise_handle *)calloc(1, sizeof(nw_bonjour_advertise_handle));
    h->listener = listener;
    h->queue = dispatch_queue_create("networkframework-rs.advertise-descriptor", DISPATCH_QUEUE_SERIAL);
    h->ready = dispatch_semaphore_create(0);
    atomic_store(&h->state_code, 0);

    nw_listener_set_queue(listener, h->queue);
    nw_listener_set_state_changed_handler(listener, ^(nw_listener_state_t state, nw_error_t error) {
        (void)error;
        if (state == nw_listener_state_ready) {
            atomic_store(&h->state_code, 1);
            dispatch_semaphore_signal(h->ready);
        } else if (state == nw_listener_state_cancelled) {
            atomic_store(&h->state_code, 2);
            dispatch_semaphore_signal(h->ready);
        } else if (state == nw_listener_state_failed) {
            atomic_store(&h->state_code, 3);
            dispatch_semaphore_signal(h->ready);
        }
    });
    nw_listener_set_new_connection_handler(listener, ^(nw_connection_t conn) {
        nw_connection_cancel(conn);
    });
    nw_listener_start(listener);

    dispatch_time_t deadline = dispatch_time(DISPATCH_TIME_NOW, 10LL * NSEC_PER_SEC);
    if (dispatch_semaphore_wait(h->ready, deadline) != 0 || atomic_load(&h->state_code) != 1) {
        nw_listener_cancel(listener);
        nw_release(h->listener);
        dispatch_release(h->queue);
        dispatch_release(h->ready);
        free(h);
        if (out_status) *out_status = NW_LISTEN_FAILED;
        return NULL;
    }

    if (out_status) *out_status = NW_OK;
    return h;
}

void *nw_shim_privacy_context_copy_default(void) {
    return nw_retain(NW_DEFAULT_PRIVACY_CONTEXT);
}

void *nw_shim_relay_hop_create(void *http3_endpoint, void *http2_endpoint, void *relay_tls_options) {
    if (!http3_endpoint && !http2_endpoint) {
        return NULL;
    }
    return nw_relay_hop_create(
        (nw_endpoint_t)http3_endpoint,
        (nw_endpoint_t)http2_endpoint,
        (nw_protocol_options_t)relay_tls_options);
}

void nw_shim_relay_hop_add_additional_http_header_field(void *relay_hop, const char *field_name, const char *field_value) {
    if (!relay_hop || !field_name || !field_value) {
        return;
    }
    nw_relay_hop_add_additional_http_header_field((nw_relay_hop_t)relay_hop, field_name, field_value);
}

void *nw_shim_proxy_config_create_relay(void *first_hop, void *second_hop) {
    if (!first_hop) {
        return NULL;
    }
    return nw_proxy_config_create_relay((nw_relay_hop_t)first_hop, (nw_relay_hop_t)second_hop);
}

void *nw_shim_proxy_config_create_oblivious_http(
    void *relay_hop,
    const char *relay_resource_path,
    const uint8_t *gateway_key_config,
    size_t gateway_key_config_length
) {
    if (!relay_hop || !relay_resource_path || !gateway_key_config || gateway_key_config_length == 0) {
        return NULL;
    }
    return nw_proxy_config_create_oblivious_http(
        (nw_relay_hop_t)relay_hop,
        relay_resource_path,
        gateway_key_config,
        gateway_key_config_length);
}

void nw_shim_proxy_config_enumerate_match_domains(
    void *proxy_config,
    void (*callback)(const char *value, void *user_info),
    void *user_info
) {
    if (!proxy_config || !callback) {
        return;
    }
    nw_proxy_config_enumerate_match_domains((nw_proxy_config_t)proxy_config, ^(const char *domain) {
        callback(domain ? domain : "", user_info);
    });
}

void nw_shim_proxy_config_enumerate_excluded_domains(
    void *proxy_config,
    void (*callback)(const char *value, void *user_info),
    void *user_info
) {
    if (!proxy_config || !callback) {
        return;
    }
    nw_proxy_config_enumerate_excluded_domains((nw_proxy_config_t)proxy_config, ^(const char *domain) {
        callback(domain ? domain : "", user_info);
    });
}

void *nw_shim_endpoint_create_host(const char *host, uint16_t port) {
    return nw_shim_create_host_endpoint(host, port);
}

void *nw_shim_endpoint_create_address(const char *address, uint16_t port) {
    return nw_shim_create_address_endpoint(address, port);
}

void *nw_shim_endpoint_create_bonjour_service(const char *name, const char *type, const char *domain) {
    if (!type) {
        return NULL;
    }
    return nw_endpoint_create_bonjour_service(name, type, domain);
}

void *nw_shim_endpoint_create_url(const char *url) {
    if (!url) {
        return NULL;
    }
    return nw_endpoint_create_url(url);
}

int nw_shim_endpoint_get_type(void *endpoint) {
    if (!endpoint) {
        return 0;
    }
    return (int)nw_endpoint_get_type((nw_endpoint_t)endpoint);
}

char *nw_shim_endpoint_copy_hostname(void *endpoint) {
    if (!endpoint) {
        return NULL;
    }
    const char *hostname = nw_endpoint_get_hostname((nw_endpoint_t)endpoint);
    return hostname ? strdup(hostname) : NULL;
}

char *nw_shim_endpoint_copy_port_string(void *endpoint) {
    if (!endpoint) {
        return NULL;
    }
    return nw_endpoint_copy_port_string((nw_endpoint_t)endpoint);
}

uint16_t nw_shim_endpoint_get_port(void *endpoint) {
    if (!endpoint) {
        return 0;
    }
    return nw_endpoint_get_port((nw_endpoint_t)endpoint);
}

char *nw_shim_endpoint_copy_address_string(void *endpoint) {
    if (!endpoint) {
        return NULL;
    }
    return nw_endpoint_copy_address_string((nw_endpoint_t)endpoint);
}

char *nw_shim_endpoint_copy_bonjour_service_name(void *endpoint) {
    if (!endpoint) {
        return NULL;
    }
    const char *value = nw_endpoint_get_bonjour_service_name((nw_endpoint_t)endpoint);
    return value ? strdup(value) : NULL;
}

char *nw_shim_endpoint_copy_bonjour_service_type(void *endpoint) {
    if (!endpoint) {
        return NULL;
    }
    const char *value = nw_endpoint_get_bonjour_service_type((nw_endpoint_t)endpoint);
    return value ? strdup(value) : NULL;
}

char *nw_shim_endpoint_copy_bonjour_service_domain(void *endpoint) {
    if (!endpoint) {
        return NULL;
    }
    const char *value = nw_endpoint_get_bonjour_service_domain((nw_endpoint_t)endpoint);
    return value ? strdup(value) : NULL;
}

char *nw_shim_endpoint_copy_url(void *endpoint) {
    if (!endpoint) {
        return NULL;
    }
    const char *value = nw_endpoint_get_url((nw_endpoint_t)endpoint);
    return value ? strdup(value) : NULL;
}

uint8_t *nw_shim_endpoint_copy_signature(void *endpoint, size_t *out_signature_length) {
    if (out_signature_length) {
        *out_signature_length = 0;
    }
    if (!endpoint) {
        return NULL;
    }
    size_t length = 0;
    const uint8_t *signature = nw_endpoint_get_signature((nw_endpoint_t)endpoint, &length);
    if (!signature || length == 0) {
        return NULL;
    }
    uint8_t *copy = (uint8_t *)malloc(length);
    if (!copy) {
        return NULL;
    }
    memcpy(copy, signature, length);
    if (out_signature_length) {
        *out_signature_length = length;
    }
    return copy;
}

int nw_shim_path_get_status(void *path) {
    return path ? (int)nw_path_get_status((nw_path_t)path) : 0;
}

int nw_shim_path_get_unsatisfied_reason(void *path) {
    return path ? (int)nw_path_get_unsatisfied_reason((nw_path_t)path) : 0;
}

int nw_shim_path_is_equal(void *path, void *other_path) {
    if (!path || !other_path) {
        return 0;
    }
    return nw_path_is_equal((nw_path_t)path, (nw_path_t)other_path) ? 1 : 0;
}

int nw_shim_path_is_expensive(void *path) {
    return path && nw_path_is_expensive((nw_path_t)path) ? 1 : 0;
}

int nw_shim_path_is_constrained(void *path) {
    return path && nw_path_is_constrained((nw_path_t)path) ? 1 : 0;
}

int nw_shim_path_is_ultra_constrained(void *path) {
    if (!path) {
        return 0;
    }
    if (__builtin_available(macOS 26.0, *)) {
        return nw_path_is_ultra_constrained((nw_path_t)path) ? 1 : 0;
    }
    return 0;
}

int nw_shim_path_has_ipv4(void *path) {
    return path && nw_path_has_ipv4((nw_path_t)path) ? 1 : 0;
}

int nw_shim_path_has_ipv6(void *path) {
    return path && nw_path_has_ipv6((nw_path_t)path) ? 1 : 0;
}

int nw_shim_path_has_dns(void *path) {
    return path && nw_path_has_dns((nw_path_t)path) ? 1 : 0;
}

int nw_shim_path_uses_interface_type(void *path, int interface_type) {
    if (!path) {
        return 0;
    }
    return nw_path_uses_interface_type((nw_path_t)path, (nw_interface_type_t)interface_type) ? 1 : 0;
}

void *nw_shim_path_copy_effective_local_endpoint(void *path) {
    return path ? nw_path_copy_effective_local_endpoint((nw_path_t)path) : NULL;
}

void *nw_shim_path_copy_effective_remote_endpoint(void *path) {
    return path ? nw_path_copy_effective_remote_endpoint((nw_path_t)path) : NULL;
}

int nw_shim_path_get_link_quality(void *path) {
    if (!path) {
        return 0;
    }
    if (__builtin_available(macOS 26.0, *)) {
        return (int)nw_path_get_link_quality((nw_path_t)path);
    }
    return 0;
}

int nw_shim_path_enumerate_interfaces(
    void *path,
    int (*callback)(const char *name, int interface_type, uint32_t index, void *user_info),
    void *user_info
) {
    if (!path || !callback) {
        return 0;
    }
    return nw_shim_enumerate_path_interfaces((nw_path_t)path, callback, user_info);
}

void *nw_shim_browse_descriptor_create_bonjour_service(const char *type, const char *domain) {
    if (!type) {
        return NULL;
    }
    return nw_browse_descriptor_create_bonjour_service(type, domain);
}

char *nw_shim_browse_descriptor_copy_bonjour_service_type(void *descriptor) {
    if (!descriptor) {
        return NULL;
    }
    const char *value = nw_browse_descriptor_get_bonjour_service_type((nw_browse_descriptor_t)descriptor);
    return value ? strdup(value) : NULL;
}

char *nw_shim_browse_descriptor_copy_bonjour_service_domain(void *descriptor) {
    if (!descriptor) {
        return NULL;
    }
    const char *value = nw_browse_descriptor_get_bonjour_service_domain((nw_browse_descriptor_t)descriptor);
    return value ? strdup(value) : NULL;
}

void nw_shim_browse_descriptor_set_include_txt_record(void *descriptor, int include_txt_record) {
    if (!descriptor) {
        return;
    }
    nw_browse_descriptor_set_include_txt_record((nw_browse_descriptor_t)descriptor, include_txt_record != 0);
}

int nw_shim_browse_descriptor_get_include_txt_record(void *descriptor) {
    if (!descriptor) {
        return 0;
    }
    return nw_browse_descriptor_get_include_txt_record((nw_browse_descriptor_t)descriptor) ? 1 : 0;
}

void *nw_shim_browse_descriptor_create_application_service(const char *application_service_name) {
    if (!application_service_name) {
        return NULL;
    }
    if (__builtin_available(macOS 13.0, *)) {
        return nw_browse_descriptor_create_application_service(application_service_name);
    }
    return NULL;
}

char *nw_shim_browse_descriptor_copy_application_service_name(void *descriptor) {
    if (!descriptor) {
        return NULL;
    }
    if (__builtin_available(macOS 13.0, *)) {
        const char *value = nw_browse_descriptor_get_application_service_name((nw_browse_descriptor_t)descriptor);
        return value ? strdup(value) : NULL;
    }
    return NULL;
}

void *nw_shim_advertise_descriptor_create_bonjour_service(const char *name, const char *type, const char *domain) {
    if (!type) {
        return NULL;
    }
    return nw_advertise_descriptor_create_bonjour_service(name, type, domain);
}

void *nw_shim_advertise_descriptor_create_application_service(const char *application_service_name) {
    if (!application_service_name) {
        return NULL;
    }
    if (__builtin_available(macOS 13.0, *)) {
        return nw_advertise_descriptor_create_application_service(application_service_name);
    }
    return NULL;
}

void nw_shim_advertise_descriptor_set_txt_record(void *descriptor, const uint8_t *txt_record, size_t txt_length) {
    if (!descriptor) {
        return;
    }
    nw_advertise_descriptor_set_txt_record((nw_advertise_descriptor_t)descriptor, txt_record, txt_length);
}

void nw_shim_advertise_descriptor_set_no_auto_rename(void *descriptor, int no_auto_rename) {
    if (!descriptor) {
        return;
    }
    nw_advertise_descriptor_set_no_auto_rename((nw_advertise_descriptor_t)descriptor, no_auto_rename != 0);
}

int nw_shim_advertise_descriptor_get_no_auto_rename(void *descriptor) {
    if (!descriptor) {
        return 0;
    }
    return nw_advertise_descriptor_get_no_auto_rename((nw_advertise_descriptor_t)descriptor) ? 1 : 0;
}

char *nw_shim_advertise_descriptor_copy_application_service_name(void *descriptor) {
    if (!descriptor) {
        return NULL;
    }
    if (__builtin_available(macOS 13.0, *)) {
        const char *value = nw_advertise_descriptor_get_application_service_name((nw_advertise_descriptor_t)descriptor);
        return value ? strdup(value) : NULL;
    }
    return NULL;
}

void *nw_shim_protocol_copy_tcp_definition(void) {
    return nw_protocol_copy_tcp_definition();
}

void *nw_shim_protocol_copy_udp_definition(void) {
    return nw_protocol_copy_udp_definition();
}

void *nw_shim_protocol_copy_tls_definition(void) {
    return nw_protocol_copy_tls_definition();
}

void *nw_shim_protocol_copy_ip_definition(void) {
    return nw_protocol_copy_ip_definition();
}

void *nw_shim_protocol_copy_ws_definition(void) {
    return nw_protocol_copy_ws_definition();
}

void *nw_shim_protocol_copy_quic_definition(void) {
    return nw_protocol_copy_quic_definition();
}

void *nw_shim_protocol_create_tcp_options(void) {
    return nw_tcp_create_options();
}

void *nw_shim_protocol_create_udp_options(void) {
    return nw_udp_create_options();
}

void *nw_shim_protocol_create_tls_options(void) {
    return nw_tls_create_options();
}

void *nw_shim_protocol_create_ip_options(void) {
    nw_parameters_t parameters = nw_parameters_create();
    if (!parameters) {
        return NULL;
    }
    nw_protocol_stack_t stack = nw_parameters_copy_default_protocol_stack(parameters);
    if (!stack) {
        nw_release(parameters);
        return NULL;
    }
    nw_protocol_options_t options = nw_protocol_stack_copy_internet_protocol(stack);
    nw_release(stack);
    nw_release(parameters);
    return options;
}

void *nw_shim_protocol_create_ws_options(void) {
    return nw_ws_create_options(nw_ws_version_13);
}

void *nw_shim_protocol_create_quic_options(void) {
    return nw_quic_create_options();
}

int nw_shim_protocol_definition_is_equal(void *definition1, void *definition2) {
    if (!definition1 || !definition2) {
        return 0;
    }
    return nw_protocol_definition_is_equal((nw_protocol_definition_t)definition1, (nw_protocol_definition_t)definition2) ? 1 : 0;
}

void *nw_shim_protocol_options_copy_definition(void *options) {
    return options ? nw_protocol_options_copy_definition((nw_protocol_options_t)options) : NULL;
}

void *nw_shim_protocol_metadata_copy_definition(void *metadata) {
    return metadata ? nw_protocol_metadata_copy_definition((nw_protocol_metadata_t)metadata) : NULL;
}

int nw_shim_protocol_options_is_quic(void *options) {
    return options && nw_protocol_options_is_quic((nw_protocol_options_t)options) ? 1 : 0;
}

void nw_shim_quic_add_tls_application_protocol(void *options, const char *application_protocol) {
    if (!options || !application_protocol) {
        return;
    }
    nw_quic_add_tls_application_protocol((nw_protocol_options_t)options, application_protocol);
}

int nw_shim_quic_get_stream_is_unidirectional(void *options) {
    return options && nw_quic_get_stream_is_unidirectional((nw_protocol_options_t)options) ? 1 : 0;
}

void nw_shim_quic_set_stream_is_unidirectional(void *options, int is_unidirectional) {
    if (!options) {
        return;
    }
    nw_quic_set_stream_is_unidirectional((nw_protocol_options_t)options, is_unidirectional != 0);
}

int nw_shim_quic_get_stream_is_datagram(void *options) {
    return options && nw_quic_get_stream_is_datagram((nw_protocol_options_t)options) ? 1 : 0;
}

void nw_shim_quic_set_stream_is_datagram(void *options, int is_datagram) {
    if (!options) {
        return;
    }
    nw_quic_set_stream_is_datagram((nw_protocol_options_t)options, is_datagram != 0);
}

uint64_t nw_shim_quic_get_initial_max_data(void *options) {
    return options ? nw_quic_get_initial_max_data((nw_protocol_options_t)options) : 0;
}

void nw_shim_quic_set_initial_max_data(void *options, uint64_t initial_max_data) {
    if (!options) {
        return;
    }
    nw_quic_set_initial_max_data((nw_protocol_options_t)options, initial_max_data);
}

uint16_t nw_shim_quic_get_max_udp_payload_size(void *options) {
    return options ? nw_quic_get_max_udp_payload_size((nw_protocol_options_t)options) : 0;
}

void nw_shim_quic_set_max_udp_payload_size(void *options, uint16_t max_udp_payload_size) {
    if (!options) {
        return;
    }
    nw_quic_set_max_udp_payload_size((nw_protocol_options_t)options, max_udp_payload_size);
}

uint32_t nw_shim_quic_get_idle_timeout(void *options) {
    return options ? nw_quic_get_idle_timeout((nw_protocol_options_t)options) : 0;
}

void nw_shim_quic_set_idle_timeout(void *options, uint32_t idle_timeout) {
    if (!options) {
        return;
    }
    nw_quic_set_idle_timeout((nw_protocol_options_t)options, idle_timeout);
}


uint64_t nw_shim_quic_get_initial_max_streams_bidirectional(void *options) {
    return options ? nw_quic_get_initial_max_streams_bidirectional((nw_protocol_options_t)options) : 0;
}

void nw_shim_quic_set_initial_max_streams_bidirectional(void *options, uint64_t initial_max_streams_bidirectional) {
    if (!options) {
        return;
    }
    nw_quic_set_initial_max_streams_bidirectional((nw_protocol_options_t)options, initial_max_streams_bidirectional);
}

uint64_t nw_shim_quic_get_initial_max_streams_unidirectional(void *options) {
    return options ? nw_quic_get_initial_max_streams_unidirectional((nw_protocol_options_t)options) : 0;
}

void nw_shim_quic_set_initial_max_streams_unidirectional(void *options, uint64_t initial_max_streams_unidirectional) {
    if (!options) {
        return;
    }
    nw_quic_set_initial_max_streams_unidirectional((nw_protocol_options_t)options, initial_max_streams_unidirectional);
}

uint64_t nw_shim_quic_get_initial_max_stream_data_bidirectional_local(void *options) {
    return options ? nw_quic_get_initial_max_stream_data_bidirectional_local((nw_protocol_options_t)options) : 0;
}

void nw_shim_quic_set_initial_max_stream_data_bidirectional_local(void *options, uint64_t initial_max_stream_data_bidirectional_local) {
    if (!options) {
        return;
    }
    nw_quic_set_initial_max_stream_data_bidirectional_local((nw_protocol_options_t)options, initial_max_stream_data_bidirectional_local);
}

uint64_t nw_shim_quic_get_initial_max_stream_data_bidirectional_remote(void *options) {
    return options ? nw_quic_get_initial_max_stream_data_bidirectional_remote((nw_protocol_options_t)options) : 0;
}

void nw_shim_quic_set_initial_max_stream_data_bidirectional_remote(void *options, uint64_t initial_max_stream_data_bidirectional_remote) {
    if (!options) {
        return;
    }
    nw_quic_set_initial_max_stream_data_bidirectional_remote((nw_protocol_options_t)options, initial_max_stream_data_bidirectional_remote);
}

uint64_t nw_shim_quic_get_initial_max_stream_data_unidirectional(void *options) {
    return options ? nw_quic_get_initial_max_stream_data_unidirectional((nw_protocol_options_t)options) : 0;
}

void nw_shim_quic_set_initial_max_stream_data_unidirectional(void *options, uint64_t initial_max_stream_data_unidirectional) {
    if (!options) {
        return;
    }
    nw_quic_set_initial_max_stream_data_unidirectional((nw_protocol_options_t)options, initial_max_stream_data_unidirectional);
}

uint16_t nw_shim_quic_get_max_datagram_frame_size(void *options) {
    return options ? nw_quic_get_max_datagram_frame_size((nw_protocol_options_t)options) : 0;
}

void nw_shim_quic_set_max_datagram_frame_size(void *options, uint16_t max_datagram_frame_size) {
    if (!options) {
        return;
    }
    nw_quic_set_max_datagram_frame_size((nw_protocol_options_t)options, max_datagram_frame_size);
}

static nw_protocol_metadata_t nw_shim_copy_quic_metadata_for_definition_from_connection(nw_connection_t connection) {
    if (!connection) {
        return NULL;
    }
    nw_protocol_definition_t definition = nw_protocol_copy_quic_definition();
    if (!definition) {
        return NULL;
    }
    nw_protocol_metadata_t metadata = nw_connection_copy_protocol_metadata(connection, definition);
    nw_release(definition);
    return metadata;
}

void *nw_shim_connection_copy_quic_metadata(void *handle) {
    nw_conn_handle *h = (nw_conn_handle *)handle;
    if (!h) {
        return NULL;
    }
    return nw_shim_copy_quic_metadata_for_definition_from_connection(h->conn);
}

void *nw_shim_content_context_copy_quic_metadata(void *context) {
    if (!context) {
        return NULL;
    }
    nw_protocol_definition_t definition = nw_protocol_copy_quic_definition();
    if (!definition) {
        return NULL;
    }
    nw_protocol_metadata_t metadata = nw_content_context_copy_protocol_metadata((nw_content_context_t)context, definition);
    nw_release(definition);
    return metadata;
}

int nw_shim_protocol_metadata_is_quic(void *metadata) {
    return metadata && nw_protocol_metadata_is_quic((nw_protocol_metadata_t)metadata) ? 1 : 0;
}

void *nw_shim_quic_copy_sec_protocol_options(void *options) {
    return options ? nw_quic_copy_sec_protocol_options((nw_protocol_options_t)options) : NULL;
}

void *nw_shim_quic_copy_sec_protocol_metadata(void *metadata) {
    return metadata ? nw_quic_copy_sec_protocol_metadata((nw_protocol_metadata_t)metadata) : NULL;
}

uint64_t nw_shim_quic_get_stream_id(void *metadata) {
    return metadata ? nw_quic_get_stream_id((nw_protocol_metadata_t)metadata) : 0;
}

int nw_shim_quic_get_stream_type(void *metadata) {
    return metadata ? (int)nw_quic_get_stream_type((nw_protocol_metadata_t)metadata) : 0;
}

uint64_t nw_shim_quic_get_stream_application_error(void *metadata) {
    return metadata ? nw_quic_get_stream_application_error((nw_protocol_metadata_t)metadata) : UINT64_MAX;
}

void nw_shim_quic_set_stream_application_error(void *metadata, uint64_t application_error) {
    if (!metadata) {
        return;
    }
    nw_quic_set_stream_application_error((nw_protocol_metadata_t)metadata, application_error);
}

uint64_t nw_shim_quic_get_local_max_streams_bidirectional(void *metadata) {
    return metadata ? nw_quic_get_local_max_streams_bidirectional((nw_protocol_metadata_t)metadata) : 0;
}

void nw_shim_quic_set_local_max_streams_bidirectional(void *metadata, uint64_t max_streams_bidirectional) {
    if (!metadata) {
        return;
    }
    nw_quic_set_local_max_streams_bidirectional((nw_protocol_metadata_t)metadata, max_streams_bidirectional);
}

uint64_t nw_shim_quic_get_local_max_streams_unidirectional(void *metadata) {
    return metadata ? nw_quic_get_local_max_streams_unidirectional((nw_protocol_metadata_t)metadata) : 0;
}

void nw_shim_quic_set_local_max_streams_unidirectional(void *metadata, uint64_t max_streams_unidirectional) {
    if (!metadata) {
        return;
    }
    nw_quic_set_local_max_streams_unidirectional((nw_protocol_metadata_t)metadata, max_streams_unidirectional);
}

uint64_t nw_shim_quic_get_remote_max_streams_bidirectional(void *metadata) {
    return metadata ? nw_quic_get_remote_max_streams_bidirectional((nw_protocol_metadata_t)metadata) : 0;
}

uint64_t nw_shim_quic_get_remote_max_streams_unidirectional(void *metadata) {
    return metadata ? nw_quic_get_remote_max_streams_unidirectional((nw_protocol_metadata_t)metadata) : 0;
}

uint16_t nw_shim_quic_get_stream_usable_datagram_frame_size(void *metadata) {
    return metadata ? nw_quic_get_stream_usable_datagram_frame_size((nw_protocol_metadata_t)metadata) : 0;
}

uint64_t nw_shim_quic_get_application_error(void *metadata) {
    return metadata ? nw_quic_get_application_error((nw_protocol_metadata_t)metadata) : UINT64_MAX;
}

char *nw_shim_quic_copy_application_error_reason(void *metadata) {
    if (!metadata) {
        return NULL;
    }
    const char *reason = nw_quic_get_application_error_reason((nw_protocol_metadata_t)metadata);
    return reason ? strdup(reason) : NULL;
}

void nw_shim_quic_set_application_error(void *metadata, uint64_t application_error, const char *reason) {
    if (!metadata) {
        return;
    }
    nw_quic_set_application_error((nw_protocol_metadata_t)metadata, application_error, reason);
}

uint16_t nw_shim_quic_get_keepalive_interval(void *metadata) {
    return metadata ? nw_quic_get_keepalive_interval((nw_protocol_metadata_t)metadata) : 0;
}

void nw_shim_quic_set_keepalive_interval(void *metadata, uint16_t keepalive_interval) {
    if (!metadata) {
        return;
    }
    nw_quic_set_keepalive_interval((nw_protocol_metadata_t)metadata, keepalive_interval);
}

uint64_t nw_shim_quic_get_remote_idle_timeout(void *metadata) {
    return metadata ? nw_quic_get_remote_idle_timeout((nw_protocol_metadata_t)metadata) : 0;
}

void *nw_shim_sec_retain(void *object) {
    return object ? sec_retain(object) : NULL;
}

void nw_shim_sec_release(void *object) {
    if (!object) {
        return;
    }
    sec_release(object);
}

char *nw_shim_interface_copy_name(void *interface) {
    if (!interface) {
        return NULL;
    }
    const char *name = nw_interface_get_name((nw_interface_t)interface);
    return name ? strdup(name) : NULL;
}

int nw_shim_interface_get_type(void *interface) {
    return interface ? (int)nw_interface_get_type((nw_interface_t)interface) : 0;
}

uint32_t nw_shim_interface_get_index(void *interface) {
    return interface ? nw_interface_get_index((nw_interface_t)interface) : 0;
}

static bool nw_shim_interface_matches(nw_interface_t interface, const char *name, int interface_type, uint32_t index) {
    if (!interface) {
        return false;
    }
    if (name && name[0] != '\0') {
        const char *candidate = nw_interface_get_name(interface);
        if (!candidate || strcmp(candidate, name) != 0) {
            return false;
        }
    }
    if ((int)nw_interface_get_type(interface) != interface_type) {
        return false;
    }
    if (index != 0 && nw_interface_get_index(interface) != index) {
        return false;
    }
    return true;
}

static nw_interface_t nw_shim_copy_matching_interface_from_path(nw_path_t path, const char *name, int interface_type, uint32_t index) {
    if (!path) {
        return NULL;
    }
    __block nw_interface_t result = NULL;
    nw_path_enumerate_interfaces(path, ^bool(nw_interface_t interface) {
        if (nw_shim_interface_matches(interface, name, interface_type, index)) {
            result = nw_retain(interface);
            return false;
        }
        return true;
    });
    return result;
}

static nw_interface_t nw_shim_copy_matching_interface(const char *name, int interface_type, uint32_t index) {
    __block nw_path_t captured_path = NULL;
    dispatch_queue_t queue = dispatch_queue_create("networkframework-rs.interface-lookup", DISPATCH_QUEUE_SERIAL);
    dispatch_semaphore_t ready = dispatch_semaphore_create(0);
    nw_path_monitor_t monitor = nw_path_monitor_create();
    if (!monitor) {
        dispatch_release(queue);
        dispatch_release(ready);
        return NULL;
    }

    nw_path_monitor_set_queue(monitor, queue);
    nw_path_monitor_set_update_handler(monitor, ^(nw_path_t path) {
        if (!captured_path && path) {
            captured_path = nw_retain(path);
            dispatch_semaphore_signal(ready);
        }
    });
    nw_path_monitor_start(monitor);

    dispatch_time_t deadline = dispatch_time(DISPATCH_TIME_NOW, 5LL * NSEC_PER_SEC);
    if (dispatch_semaphore_wait(ready, deadline) != 0) {
        nw_path_monitor_cancel(monitor);
        nw_release(monitor);
        dispatch_release(queue);
        dispatch_release(ready);
        return NULL;
    }

    nw_interface_t interface = nw_shim_copy_matching_interface_from_path(captured_path, name, interface_type, index);
    nw_path_monitor_cancel(monitor);
    if (captured_path) {
        nw_release(captured_path);
    }
    nw_release(monitor);
    dispatch_release(queue);
    dispatch_release(ready);
    return interface;
}

void nw_shim_parameters_require_interface(void *parameters, const char *name, int interface_type, uint32_t index) {
    if (!parameters) {
        return;
    }
    if (!name && index == 0) {
        nw_parameters_require_interface((nw_parameters_t)parameters, NULL);
        return;
    }
    nw_interface_t interface = nw_shim_copy_matching_interface(name, interface_type, index);
    nw_parameters_require_interface((nw_parameters_t)parameters, interface);
    if (interface) {
        nw_release(interface);
    }
}

int nw_shim_parameters_copy_required_interface(void *parameters, char **out_name, int *out_type, uint32_t *out_index) {
    if (out_name) {
        *out_name = NULL;
    }
    if (out_type) {
        *out_type = 0;
    }
    if (out_index) {
        *out_index = 0;
    }
    if (!parameters) {
        return 0;
    }
    nw_interface_t interface = nw_parameters_copy_required_interface((nw_parameters_t)parameters);
    if (!interface) {
        return 0;
    }
    if (out_name) {
        const char *name = nw_interface_get_name(interface);
        *out_name = name ? strdup(name) : NULL;
    }
    if (out_type) {
        *out_type = (int)nw_interface_get_type(interface);
    }
    if (out_index) {
        *out_index = nw_interface_get_index(interface);
    }
    nw_release(interface);
    return 1;
}

void nw_shim_parameters_prohibit_interface(void *parameters, const char *name, int interface_type, uint32_t index) {
    if (!parameters || !name) {
        return;
    }
    nw_interface_t interface = nw_shim_copy_matching_interface(name, interface_type, index);
    if (!interface) {
        return;
    }
    nw_parameters_prohibit_interface((nw_parameters_t)parameters, interface);
    nw_release(interface);
}

void nw_shim_parameters_clear_prohibited_interfaces(void *parameters) {
    if (!parameters) {
        return;
    }
    nw_parameters_clear_prohibited_interfaces((nw_parameters_t)parameters);
}

void **nw_shim_parameters_copy_prohibited_interfaces(void *parameters, size_t *out_count) {
    if (out_count) {
        *out_count = 0;
    }
    if (!parameters) {
        return NULL;
    }
    __block size_t count = 0;
    nw_parameters_iterate_prohibited_interfaces((nw_parameters_t)parameters, ^bool(nw_interface_t interface) {
        (void)interface;
        count += 1;
        return true;
    });
    if (count == 0) {
        return NULL;
    }
    void **items = (void **)calloc(count, sizeof(void *));
    if (!items) {
        return NULL;
    }
    __block size_t index = 0;
    nw_parameters_iterate_prohibited_interfaces((nw_parameters_t)parameters, ^bool(nw_interface_t interface) {
        if (index < count) {
            items[index++] = nw_retain(interface);
        }
        return true;
    });
    if (out_count) {
        *out_count = count;
    }
    return items;
}

void nw_shim_parameters_prohibit_interface_type(void *parameters, int interface_type) {
    if (!parameters) {
        return;
    }
    nw_parameters_prohibit_interface_type((nw_parameters_t)parameters, (nw_interface_type_t)interface_type);
}

void nw_shim_parameters_clear_prohibited_interface_types(void *parameters) {
    if (!parameters) {
        return;
    }
    nw_parameters_clear_prohibited_interface_types((nw_parameters_t)parameters);
}

int *nw_shim_parameters_copy_prohibited_interface_types(void *parameters, size_t *out_count) {
    if (out_count) {
        *out_count = 0;
    }
    if (!parameters) {
        return NULL;
    }
    __block size_t count = 0;
    nw_parameters_iterate_prohibited_interface_types((nw_parameters_t)parameters, ^bool(nw_interface_type_t interface_type) {
        (void)interface_type;
        count += 1;
        return true;
    });
    if (count == 0) {
        return NULL;
    }
    int *items = (int *)calloc(count, sizeof(int));
    if (!items) {
        return NULL;
    }
    __block size_t index = 0;
    nw_parameters_iterate_prohibited_interface_types((nw_parameters_t)parameters, ^bool(nw_interface_type_t interface_type) {
        if (index < count) {
            items[index++] = (int)interface_type;
        }
        return true;
    });
    if (out_count) {
        *out_count = count;
    }
    return items;
}

void nw_shim_parameters_set_reuse_local_address(void *parameters, int reuse_local_address) {
    if (!parameters) {
        return;
    }
    nw_parameters_set_reuse_local_address((nw_parameters_t)parameters, reuse_local_address != 0);
}

int nw_shim_parameters_get_reuse_local_address(void *parameters) {
    return parameters && nw_parameters_get_reuse_local_address((nw_parameters_t)parameters) ? 1 : 0;
}

void nw_shim_parameters_set_local_endpoint(void *parameters, void *local_endpoint) {
    if (!parameters) {
        return;
    }
    nw_parameters_set_local_endpoint((nw_parameters_t)parameters, (nw_endpoint_t)local_endpoint);
}

void *nw_shim_parameters_copy_local_endpoint(void *parameters) {
    return parameters ? nw_parameters_copy_local_endpoint((nw_parameters_t)parameters) : NULL;
}

void nw_shim_parameters_set_include_peer_to_peer(void *parameters, int include_peer_to_peer) {
    if (!parameters) {
        return;
    }
    nw_parameters_set_include_peer_to_peer((nw_parameters_t)parameters, include_peer_to_peer != 0);
}

int nw_shim_parameters_get_include_peer_to_peer(void *parameters) {
    return parameters && nw_parameters_get_include_peer_to_peer((nw_parameters_t)parameters) ? 1 : 0;
}

void nw_shim_parameters_set_fast_open_enabled(void *parameters, int fast_open_enabled) {
    if (!parameters) {
        return;
    }
    nw_parameters_set_fast_open_enabled((nw_parameters_t)parameters, fast_open_enabled != 0);
}

int nw_shim_parameters_get_fast_open_enabled(void *parameters) {
    return parameters && nw_parameters_get_fast_open_enabled((nw_parameters_t)parameters) ? 1 : 0;
}

void nw_shim_parameters_set_service_class(void *parameters, int service_class) {
    if (!parameters) {
        return;
    }
    nw_parameters_set_service_class((nw_parameters_t)parameters, (nw_service_class_t)service_class);
}

int nw_shim_parameters_get_service_class(void *parameters) {
    return parameters ? (int)nw_parameters_get_service_class((nw_parameters_t)parameters) : 0;
}

void nw_shim_parameters_set_multipath_service(void *parameters, int multipath_service) {
    if (!parameters) {
        return;
    }
    nw_parameters_set_multipath_service((nw_parameters_t)parameters, (nw_multipath_service_t)multipath_service);
}

int nw_shim_parameters_get_multipath_service(void *parameters) {
    return parameters ? (int)nw_parameters_get_multipath_service((nw_parameters_t)parameters) : 0;
}

void *nw_shim_parameters_copy_default_protocol_stack(void *parameters) {
    return parameters ? nw_parameters_copy_default_protocol_stack((nw_parameters_t)parameters) : NULL;
}

void nw_shim_protocol_stack_clear_application_protocols(void *stack) {
    if (!stack) {
        return;
    }
    nw_protocol_stack_clear_application_protocols((nw_protocol_stack_t)stack);
}

void **nw_shim_protocol_stack_copy_application_protocols(void *stack, size_t *out_count) {
    if (out_count) {
        *out_count = 0;
    }
    if (!stack) {
        return NULL;
    }
    __block size_t count = 0;
    nw_protocol_stack_iterate_application_protocols((nw_protocol_stack_t)stack, ^(nw_protocol_options_t protocol) {
        (void)protocol;
        count += 1;
    });
    if (count == 0) {
        return NULL;
    }
    void **items = (void **)calloc(count, sizeof(void *));
    if (!items) {
        return NULL;
    }
    __block size_t index = 0;
    nw_protocol_stack_iterate_application_protocols((nw_protocol_stack_t)stack, ^(nw_protocol_options_t protocol) {
        if (index < count) {
            items[index++] = nw_retain(protocol);
        }
    });
    if (out_count) {
        *out_count = count;
    }
    return items;
}

void *nw_shim_protocol_stack_copy_transport_protocol(void *stack) {
    return stack ? nw_protocol_stack_copy_transport_protocol((nw_protocol_stack_t)stack) : NULL;
}

void nw_shim_protocol_stack_set_transport_protocol(void *stack, void *protocol) {
    if (!stack) {
        return;
    }
    nw_protocol_stack_set_transport_protocol((nw_protocol_stack_t)stack, (nw_protocol_options_t)protocol);
}

void *nw_shim_protocol_stack_copy_internet_protocol(void *stack) {
    return stack ? nw_protocol_stack_copy_internet_protocol((nw_protocol_stack_t)stack) : NULL;
}

void nw_shim_parameters_set_local_only(void *parameters, int local_only) {
    if (!parameters) {
        return;
    }
    nw_parameters_set_local_only((nw_parameters_t)parameters, local_only != 0);
}

int nw_shim_parameters_get_local_only(void *parameters) {
    return parameters && nw_parameters_get_local_only((nw_parameters_t)parameters) ? 1 : 0;
}

int nw_shim_parameters_get_prefer_no_proxy(void *parameters) {
    return parameters && nw_parameters_get_prefer_no_proxy((nw_parameters_t)parameters) ? 1 : 0;
}

void nw_shim_parameters_set_expired_dns_behavior(void *parameters, int expired_dns_behavior) {
    if (!parameters) {
        return;
    }
    nw_parameters_set_expired_dns_behavior((nw_parameters_t)parameters, (nw_parameters_expired_dns_behavior_t)expired_dns_behavior);
}

int nw_shim_parameters_get_expired_dns_behavior(void *parameters) {
    return parameters ? (int)nw_parameters_get_expired_dns_behavior((nw_parameters_t)parameters) : 0;
}

void nw_shim_parameters_set_requires_dnssec_validation(void *parameters, int requires_dnssec_validation) {
    if (!parameters) {
        return;
    }
    nw_parameters_set_requires_dnssec_validation((nw_parameters_t)parameters, requires_dnssec_validation != 0);
}

int nw_shim_parameters_requires_dnssec_validation(void *parameters) {
    return parameters && nw_parameters_requires_dnssec_validation((nw_parameters_t)parameters) ? 1 : 0;
}

void *nw_shim_connection_copy_establishment_report(void *handle) {
    nw_conn_handle *h = (nw_conn_handle *)handle;
    if (!h) {
        return NULL;
    }
    __block nw_establishment_report_t report = NULL;
    dispatch_semaphore_t ready = dispatch_semaphore_create(0);
    nw_connection_access_establishment_report(h->conn, h->queue, ^(nw_establishment_report_t accessed_report) {
        if (accessed_report) {
            report = nw_retain(accessed_report);
        }
        dispatch_semaphore_signal(ready);
    });
    dispatch_time_t deadline = dispatch_time(DISPATCH_TIME_NOW, 30LL * NSEC_PER_SEC);
    if (dispatch_semaphore_wait(ready, deadline) != 0) {
        dispatch_release(ready);
        if (report) {
            nw_release(report);
        }
        return NULL;
    }
    dispatch_release(ready);
    return report;
}

uint64_t nw_shim_establishment_report_get_duration_milliseconds(void *report) {
    return report ? nw_establishment_report_get_duration_milliseconds((nw_establishment_report_t)report) : 0;
}

uint64_t nw_shim_establishment_report_get_attempt_started_after_milliseconds(void *report) {
    return report ? nw_establishment_report_get_attempt_started_after_milliseconds((nw_establishment_report_t)report) : 0;
}

uint32_t nw_shim_establishment_report_get_previous_attempt_count(void *report) {
    return report ? nw_establishment_report_get_previous_attempt_count((nw_establishment_report_t)report) : 0;
}

int nw_shim_establishment_report_get_used_proxy(void *report) {
    return report && nw_establishment_report_get_used_proxy((nw_establishment_report_t)report) ? 1 : 0;
}

int nw_shim_establishment_report_get_proxy_configured(void *report) {
    return report && nw_establishment_report_get_proxy_configured((nw_establishment_report_t)report) ? 1 : 0;
}

void *nw_shim_establishment_report_copy_proxy_endpoint(void *report) {
    return report ? nw_establishment_report_copy_proxy_endpoint((nw_establishment_report_t)report) : NULL;
}

nw_shim_establishment_protocol_info *nw_shim_establishment_report_copy_protocols(void *report, size_t *out_count) {
    if (out_count) {
        *out_count = 0;
    }
    if (!report) {
        return NULL;
    }
    __block size_t count = 0;
    nw_establishment_report_enumerate_protocols((nw_establishment_report_t)report, ^bool(nw_protocol_definition_t protocol, uint64_t handshake_milliseconds, uint64_t handshake_rtt_milliseconds) {
        (void)protocol;
        (void)handshake_milliseconds;
        (void)handshake_rtt_milliseconds;
        count += 1;
        return true;
    });
    if (count == 0) {
        return NULL;
    }
    nw_shim_establishment_protocol_info *items = (nw_shim_establishment_protocol_info *)calloc(count, sizeof(nw_shim_establishment_protocol_info));
    if (!items) {
        return NULL;
    }
    __block size_t index = 0;
    nw_establishment_report_enumerate_protocols((nw_establishment_report_t)report, ^bool(nw_protocol_definition_t protocol, uint64_t handshake_milliseconds, uint64_t handshake_rtt_milliseconds) {
        if (index < count) {
            items[index].protocol_definition = protocol ? nw_retain(protocol) : NULL;
            items[index].handshake_milliseconds = handshake_milliseconds;
            items[index].handshake_rtt_milliseconds = handshake_rtt_milliseconds;
            index += 1;
        }
        return true;
    });
    if (out_count) {
        *out_count = count;
    }
    return items;
}

nw_shim_resolution_step_info *nw_shim_establishment_report_copy_resolutions(void *report, size_t *out_count) {
    if (out_count) {
        *out_count = 0;
    }
    if (!report) {
        return NULL;
    }
    __block size_t count = 0;
    nw_establishment_report_enumerate_resolutions((nw_establishment_report_t)report, ^bool(nw_report_resolution_source_t source, uint64_t milliseconds, uint32_t endpoint_count, nw_endpoint_t successful_endpoint, nw_endpoint_t preferred_endpoint) {
        (void)source;
        (void)milliseconds;
        (void)endpoint_count;
        (void)successful_endpoint;
        (void)preferred_endpoint;
        count += 1;
        return true;
    });
    if (count == 0) {
        return NULL;
    }
    nw_shim_resolution_step_info *items = (nw_shim_resolution_step_info *)calloc(count, sizeof(nw_shim_resolution_step_info));
    if (!items) {
        return NULL;
    }
    __block size_t index = 0;
    nw_establishment_report_enumerate_resolutions((nw_establishment_report_t)report, ^bool(nw_report_resolution_source_t source, uint64_t milliseconds, uint32_t endpoint_count, nw_endpoint_t successful_endpoint, nw_endpoint_t preferred_endpoint) {
        if (index < count) {
            items[index].source = (int)source;
            items[index].milliseconds = milliseconds;
            items[index].endpoint_count = endpoint_count;
            items[index].successful_endpoint = successful_endpoint ? nw_retain(successful_endpoint) : NULL;
            items[index].preferred_endpoint = preferred_endpoint ? nw_retain(preferred_endpoint) : NULL;
            index += 1;
        }
        return true;
    });
    if (out_count) {
        *out_count = count;
    }
    return items;
}

void **nw_shim_establishment_report_copy_resolution_reports(void *report, size_t *out_count) {
    if (out_count) {
        *out_count = 0;
    }
    if (!report) {
        return NULL;
    }
    __block size_t count = 0;
    nw_establishment_report_enumerate_resolution_reports((nw_establishment_report_t)report, ^bool(nw_resolution_report_t resolution_report) {
        (void)resolution_report;
        count += 1;
        return true;
    });
    if (count == 0) {
        return NULL;
    }
    void **items = (void **)calloc(count, sizeof(void *));
    if (!items) {
        return NULL;
    }
    __block size_t index = 0;
    nw_establishment_report_enumerate_resolution_reports((nw_establishment_report_t)report, ^bool(nw_resolution_report_t resolution_report) {
        if (index < count) {
            items[index++] = resolution_report ? nw_retain(resolution_report) : NULL;
        }
        return true;
    });
    if (out_count) {
        *out_count = count;
    }
    return items;
}

int nw_shim_resolution_report_get_source(void *report) {
    return report ? (int)nw_resolution_report_get_source((nw_resolution_report_t)report) : 0;
}

uint64_t nw_shim_resolution_report_get_milliseconds(void *report) {
    return report ? nw_resolution_report_get_milliseconds((nw_resolution_report_t)report) : 0;
}

uint32_t nw_shim_resolution_report_get_endpoint_count(void *report) {
    return report ? nw_resolution_report_get_endpoint_count((nw_resolution_report_t)report) : 0;
}

void *nw_shim_resolution_report_copy_successful_endpoint(void *report) {
    return report ? nw_resolution_report_copy_successful_endpoint((nw_resolution_report_t)report) : NULL;
}

void *nw_shim_resolution_report_copy_preferred_endpoint(void *report) {
    return report ? nw_resolution_report_copy_preferred_endpoint((nw_resolution_report_t)report) : NULL;
}

int nw_shim_resolution_report_get_protocol(void *report) {
    return report ? (int)nw_resolution_report_get_protocol((nw_resolution_report_t)report) : 0;
}

void *nw_shim_connection_create_data_transfer_report(void *handle) {
    nw_conn_handle *h = (nw_conn_handle *)handle;
    if (!h) {
        return NULL;
    }
    return nw_connection_create_new_data_transfer_report(h->conn);
}

int nw_shim_data_transfer_report_collect(void *report) {
    if (!report) {
        return NW_INVALID_ARG;
    }
    dispatch_queue_t queue = dispatch_queue_create("networkframework-rs.data-transfer-report", DISPATCH_QUEUE_SERIAL);
    dispatch_semaphore_t ready = dispatch_semaphore_create(0);
    nw_data_transfer_report_collect((nw_data_transfer_report_t)report, queue, ^(nw_data_transfer_report_t collected_report) {
        (void)collected_report;
        dispatch_semaphore_signal(ready);
    });
    dispatch_time_t deadline = dispatch_time(DISPATCH_TIME_NOW, 30LL * NSEC_PER_SEC);
    int status = dispatch_semaphore_wait(ready, deadline) == 0 ? NW_OK : NW_TIMEOUT;
    dispatch_release(queue);
    dispatch_release(ready);
    return status;
}

int nw_shim_data_transfer_report_get_state(void *report) {
    return report ? (int)nw_data_transfer_report_get_state((nw_data_transfer_report_t)report) : 0;
}

uint32_t nw_shim_data_transfer_report_all_paths(void) {
    return NW_ALL_PATHS;
}

uint64_t nw_shim_data_transfer_report_get_duration_milliseconds(void *report) {
    return report ? nw_data_transfer_report_get_duration_milliseconds((nw_data_transfer_report_t)report) : 0;
}

uint32_t nw_shim_data_transfer_report_get_path_count(void *report) {
    return report ? nw_data_transfer_report_get_path_count((nw_data_transfer_report_t)report) : 0;
}

uint64_t nw_shim_data_transfer_report_get_received_ip_packet_count(void *report, uint32_t path_index) {
    return report ? nw_data_transfer_report_get_received_ip_packet_count((nw_data_transfer_report_t)report, path_index) : 0;
}

uint64_t nw_shim_data_transfer_report_get_sent_ip_packet_count(void *report, uint32_t path_index) {
    return report ? nw_data_transfer_report_get_sent_ip_packet_count((nw_data_transfer_report_t)report, path_index) : 0;
}

uint64_t nw_shim_data_transfer_report_get_received_transport_byte_count(void *report, uint32_t path_index) {
    return report ? nw_data_transfer_report_get_received_transport_byte_count((nw_data_transfer_report_t)report, path_index) : 0;
}

uint64_t nw_shim_data_transfer_report_get_received_transport_duplicate_byte_count(void *report, uint32_t path_index) {
    return report ? nw_data_transfer_report_get_received_transport_duplicate_byte_count((nw_data_transfer_report_t)report, path_index) : 0;
}

uint64_t nw_shim_data_transfer_report_get_received_transport_out_of_order_byte_count(void *report, uint32_t path_index) {
    return report ? nw_data_transfer_report_get_received_transport_out_of_order_byte_count((nw_data_transfer_report_t)report, path_index) : 0;
}

uint64_t nw_shim_data_transfer_report_get_sent_transport_byte_count(void *report, uint32_t path_index) {
    return report ? nw_data_transfer_report_get_sent_transport_byte_count((nw_data_transfer_report_t)report, path_index) : 0;
}

uint64_t nw_shim_data_transfer_report_get_sent_transport_retransmitted_byte_count(void *report, uint32_t path_index) {
    return report ? nw_data_transfer_report_get_sent_transport_retransmitted_byte_count((nw_data_transfer_report_t)report, path_index) : 0;
}

uint64_t nw_shim_data_transfer_report_get_transport_smoothed_rtt_milliseconds(void *report, uint32_t path_index) {
    return report ? nw_data_transfer_report_get_transport_smoothed_rtt_milliseconds((nw_data_transfer_report_t)report, path_index) : 0;
}

uint64_t nw_shim_data_transfer_report_get_transport_minimum_rtt_milliseconds(void *report, uint32_t path_index) {
    return report ? nw_data_transfer_report_get_transport_minimum_rtt_milliseconds((nw_data_transfer_report_t)report, path_index) : 0;
}

uint64_t nw_shim_data_transfer_report_get_transport_rtt_variance(void *report, uint32_t path_index) {
    return report ? nw_data_transfer_report_get_transport_rtt_variance((nw_data_transfer_report_t)report, path_index) : 0;
}

uint64_t nw_shim_data_transfer_report_get_received_application_byte_count(void *report, uint32_t path_index) {
    return report ? nw_data_transfer_report_get_received_application_byte_count((nw_data_transfer_report_t)report, path_index) : 0;
}

uint64_t nw_shim_data_transfer_report_get_sent_application_byte_count(void *report, uint32_t path_index) {
    return report ? nw_data_transfer_report_get_sent_application_byte_count((nw_data_transfer_report_t)report, path_index) : 0;
}

void *nw_shim_data_transfer_report_copy_path_interface(void *report, uint32_t path_index) {
    return report ? nw_data_transfer_report_copy_path_interface((nw_data_transfer_report_t)report, path_index) : NULL;
}

int nw_shim_data_transfer_report_get_path_radio_type(void *report, uint32_t path_index) {
    return report ? (int)nw_data_transfer_report_get_path_radio_type((nw_data_transfer_report_t)report, path_index) : 0;
}

size_t nw_shim_endpoint_copy_address(void *endpoint, void *out_buffer, size_t out_buffer_length) {
    if (!endpoint) {
        return 0;
    }
    const struct sockaddr *address = nw_endpoint_get_address((nw_endpoint_t)endpoint);
    if (!address) {
        return 0;
    }
    size_t address_length = address->sa_len;
    if (address_length == 0) {
        switch (address->sa_family) {
            case AF_INET:
                address_length = sizeof(struct sockaddr_in);
                break;
            case AF_INET6:
                address_length = sizeof(struct sockaddr_in6);
                break;
            default:
                address_length = sizeof(struct sockaddr);
                break;
        }
    }
    if (out_buffer && out_buffer_length >= address_length) {
        memcpy(out_buffer, address, address_length);
    }
    return address_length;
}

void *nw_shim_endpoint_copy_txt_record(void *endpoint) {
    return endpoint ? nw_endpoint_copy_txt_record((nw_endpoint_t)endpoint) : NULL;
}

void *nw_shim_txt_record_create_with_bytes(const uint8_t *txt_bytes, size_t txt_length) {
    if (!txt_bytes || txt_length == 0) {
        return NULL;
    }
    return nw_txt_record_create_with_bytes(txt_bytes, txt_length);
}

void *nw_shim_txt_record_create_dictionary(void) {
    return nw_txt_record_create_dictionary();
}

void *nw_shim_txt_record_copy(void *txt_record) {
    return txt_record ? nw_txt_record_copy((nw_txt_record_t)txt_record) : NULL;
}

int nw_shim_txt_record_find_key(void *txt_record, const char *key) {
    if (!txt_record || !key) {
        return 0;
    }
    return (int)nw_txt_record_find_key((nw_txt_record_t)txt_record, key);
}

uint8_t *nw_shim_txt_record_copy_value(void *txt_record, const char *key, size_t *out_value_length, int *out_found) {
    if (out_value_length) {
        *out_value_length = 0;
    }
    if (out_found) {
        *out_found = 0;
    }
    if (!txt_record || !key) {
        return NULL;
    }
    __block int found = 0;
    __block uint8_t *bytes = NULL;
    __block size_t length = 0;
    nw_txt_record_access_key((nw_txt_record_t)txt_record, key, ^bool(const char *access_key, const nw_txt_record_find_key_t access_found, const uint8_t *value, const size_t value_len) {
        (void)access_key;
        found = (int)access_found;
        if (value && value_len > 0) {
            bytes = (uint8_t *)malloc(value_len);
            if (bytes) {
                memcpy(bytes, value, value_len);
                length = value_len;
            }
        }
        return true;
    });
    if (out_value_length) {
        *out_value_length = length;
    }
    if (out_found) {
        *out_found = found;
    }
    return bytes;
}

int nw_shim_txt_record_set_key(void *txt_record, const char *key, const uint8_t *value, size_t value_length) {
    if (!txt_record || !key) {
        return 0;
    }
    return nw_txt_record_set_key((nw_txt_record_t)txt_record, key, value, value_length) ? 1 : 0;
}

int nw_shim_txt_record_remove_key(void *txt_record, const char *key) {
    if (!txt_record || !key) {
        return 0;
    }
    return nw_txt_record_remove_key((nw_txt_record_t)txt_record, key) ? 1 : 0;
}

size_t nw_shim_txt_record_get_key_count(void *txt_record) {
    return txt_record ? nw_txt_record_get_key_count((nw_txt_record_t)txt_record) : 0;
}

uint8_t *nw_shim_txt_record_copy_bytes(void *txt_record, size_t *out_length) {
    if (out_length) {
        *out_length = 0;
    }
    if (!txt_record) {
        return NULL;
    }
    __block uint8_t *bytes = NULL;
    __block size_t length = 0;
    nw_txt_record_access_bytes((nw_txt_record_t)txt_record, ^bool(const uint8_t *raw_txt_record, size_t raw_length) {
        if (raw_txt_record && raw_length > 0) {
            bytes = (uint8_t *)malloc(raw_length);
            if (bytes) {
                memcpy(bytes, raw_txt_record, raw_length);
                length = raw_length;
            }
        }
        return true;
    });
    if (out_length) {
        *out_length = length;
    }
    return bytes;
}

int nw_shim_txt_record_apply(void *txt_record, TxtRecordEntryCallback callback, void *user_info) {
    if (!txt_record || !callback) {
        return 0;
    }
    __block int count = 0;
    nw_txt_record_apply((nw_txt_record_t)txt_record, ^bool(const char *key, const nw_txt_record_find_key_t found, const uint8_t *value, const size_t value_len) {
        count += 1;
        return callback(key ? key : "", (int)found, value, value_len, user_info) != 0;
    });
    return count;
}

int nw_shim_txt_record_is_dictionary(void *txt_record) {
    return txt_record && nw_txt_record_is_dictionary((nw_txt_record_t)txt_record) ? 1 : 0;
}

int nw_shim_txt_record_is_equal(void *txt_record, void *other_txt_record) {
    if (!txt_record || !other_txt_record) {
        return 0;
    }
    return nw_txt_record_is_equal((nw_txt_record_t)txt_record, (nw_txt_record_t)other_txt_record) ? 1 : 0;
}

typedef struct nw_ethernet_channel_handle {
    nw_ethernet_channel_t channel;
    dispatch_queue_t queue;
} nw_ethernet_channel_handle;

static void nw_shim_destroy_ethernet_channel_handle(nw_ethernet_channel_handle *handle) {
    if (!handle) {
        return;
    }
    if (handle->channel) {
        nw_release(handle->channel);
    }
    if (handle->queue) {
        dispatch_release(handle->queue);
    }
    free(handle);
}

static nw_ethernet_channel_handle *nw_shim_create_ethernet_channel_handle(nw_ethernet_channel_t channel) {
    if (!channel) {
        return NULL;
    }
    nw_ethernet_channel_handle *handle = (nw_ethernet_channel_handle *)calloc(1, sizeof(nw_ethernet_channel_handle));
    if (!handle) {
        nw_release(channel);
        return NULL;
    }
    handle->channel = channel;
    handle->queue = dispatch_queue_create("networkframework-rs.ethernet-channel", DISPATCH_QUEUE_SERIAL);
    nw_ethernet_channel_set_queue(channel, handle->queue);
    return handle;
}

void *nw_shim_ethernet_channel_create(uint16_t ether_type, const char *name, int interface_type, uint32_t index) {
    if (!name) {
        return NULL;
    }
    nw_interface_t interface = nw_shim_copy_matching_interface(name, interface_type, index);
    if (!interface) {
        return NULL;
    }
    nw_ethernet_channel_t channel = nw_ethernet_channel_create(ether_type, interface);
    nw_release(interface);
    return nw_shim_create_ethernet_channel_handle(channel);
}

void *nw_shim_ethernet_channel_create_with_parameters(uint16_t ether_type, const char *name, int interface_type, uint32_t index, void *parameters) {
    if (!name || !parameters) {
        return NULL;
    }
    nw_interface_t interface = nw_shim_copy_matching_interface(name, interface_type, index);
    if (!interface) {
        return NULL;
    }
    nw_ethernet_channel_t channel = nw_ethernet_channel_create_with_parameters(ether_type, interface, (nw_parameters_t)parameters);
    nw_release(interface);
    return nw_shim_create_ethernet_channel_handle(channel);
}

void nw_shim_ethernet_channel_set_state_changed_handler(void *handle, EthernetChannelStateCallback callback, void *user_info) {
    nw_ethernet_channel_handle *h = (nw_ethernet_channel_handle *)handle;
    if (!h) {
        return;
    }
    nw_ethernet_channel_set_state_changed_handler(h->channel, callback ? ^(nw_ethernet_channel_state_t state, nw_error_t error) {
        (void)error;
        callback((int)state, user_info);
    } : NULL);
}

void nw_shim_ethernet_channel_set_receive_handler(void *handle, EthernetChannelReceiveCallback callback, void *user_info) {
    nw_ethernet_channel_handle *h = (nw_ethernet_channel_handle *)handle;
    if (!h) {
        return;
    }
    nw_ethernet_channel_set_receive_handler(h->channel, callback ? ^(dispatch_data_t content, uint16_t vlan_tag, nw_ethernet_address_t local_address, nw_ethernet_address_t remote_address) {
        uint8_t *bytes = NULL;
        size_t length = nw_shim_copy_dispatch_data(content, &bytes);
        callback(bytes, length, vlan_tag, local_address, remote_address, user_info);
        if (bytes) {
            free(bytes);
        }
    } : NULL);
}

uint32_t nw_shim_ethernet_channel_get_maximum_payload_size(void *handle) {
    nw_ethernet_channel_handle *h = (nw_ethernet_channel_handle *)handle;
    return h ? nw_ethernet_channel_get_maximum_payload_size(h->channel) : 0;
}

void nw_shim_ethernet_channel_start(void *handle) {
    nw_ethernet_channel_handle *h = (nw_ethernet_channel_handle *)handle;
    if (!h) {
        return;
    }
    nw_ethernet_channel_start(h->channel);
}

void nw_shim_ethernet_channel_cancel(void *handle) {
    nw_ethernet_channel_handle *h = (nw_ethernet_channel_handle *)handle;
    if (!h) {
        return;
    }
    nw_ethernet_channel_cancel(h->channel);
}

int nw_shim_ethernet_channel_send(void *handle, const uint8_t *data, size_t len, uint16_t vlan_tag, const uint8_t *remote_address) {
    nw_ethernet_channel_handle *h = (nw_ethernet_channel_handle *)handle;
    if (!h || !data || !remote_address) {
        return NW_INVALID_ARG;
    }
    dispatch_data_t payload = dispatch_data_create(data, len, h->queue, DISPATCH_DATA_DESTRUCTOR_DEFAULT);
    nw_ethernet_address_t destination;
    memcpy(destination, remote_address, sizeof(destination));
    __block int result = NW_OK;
    dispatch_semaphore_t done = dispatch_semaphore_create(0);
    nw_ethernet_channel_send(h->channel, payload, vlan_tag, destination, ^(nw_error_t error) {
        if (error) {
            result = NW_SEND_FAILED;
        }
        dispatch_semaphore_signal(done);
    });
    dispatch_semaphore_wait(done, DISPATCH_TIME_FOREVER);
    dispatch_release(done);
    dispatch_release(payload);
    return result;
}

void nw_shim_ethernet_channel_release(void *handle) {
    nw_ethernet_channel_handle *h = (nw_ethernet_channel_handle *)handle;
    if (!h) {
        return;
    }
    nw_shim_destroy_ethernet_channel_handle(h);
}


// ---------------------------------------------------------------------
// Coverage gap support additions
// ---------------------------------------------------------------------

static char *nw_shim_copy_cfstring_value(CFStringRef value) {
    if (!value) {
        return NULL;
    }
    CFIndex length = CFStringGetLength(value);
    CFIndex maximum = CFStringGetMaximumSizeForEncoding(length, kCFStringEncodingUTF8) + 1;
    char *buffer = (char *)malloc((size_t)maximum);
    if (!buffer) {
        return NULL;
    }
    if (!CFStringGetCString(value, buffer, maximum, kCFStringEncodingUTF8)) {
        free(buffer);
        return NULL;
    }
    return buffer;
}

static int nw_shim_invoke_interface_callback(nw_interface_t interface, InterfaceEnumerationCallback callback, void *user_info) {
    if (!interface || !callback) {
        return 0;
    }
    const char *name = nw_interface_get_name(interface);
    return callback(name ? name : "", (int)nw_interface_get_type(interface), nw_interface_get_index(interface), user_info);
}

static int nw_shim_invoke_endpoint_callback(nw_endpoint_t endpoint, EndpointEnumerationCallback callback, void *user_info) {
    if (!endpoint || !callback) {
        return 0;
    }
    void *retained = nw_retain(endpoint);
    return callback(retained, user_info);
}

static nw_conn_handle *nw_shim_wrap_started_connection(nw_connection_t conn, const char *label, int *out_status) {
    if (!conn) {
        if (out_status) *out_status = NW_INVALID_ARG;
        return NULL;
    }

    nw_conn_handle *h = (nw_conn_handle *)calloc(1, sizeof(nw_conn_handle));
    if (!h) {
        if (out_status) *out_status = NW_CONNECT_FAILED;
        return NULL;
    }
    h->conn = conn;
    h->queue = dispatch_queue_create(label ? label : "networkframework-rs.conn.wrap", DISPATCH_QUEUE_SERIAL);
    h->ready = dispatch_semaphore_create(0);
    atomic_store(&h->state_code, 0);

    nw_connection_set_queue(conn, h->queue);
    nw_connection_set_state_changed_handler(conn, ^(nw_connection_state_t state, nw_error_t error) {
        (void)error;
        if (state == nw_connection_state_ready) {
            atomic_store(&h->state_code, 1);
            dispatch_semaphore_signal(h->ready);
        } else if (state == nw_connection_state_cancelled) {
            atomic_store(&h->state_code, 2);
            dispatch_semaphore_signal(h->ready);
        } else if (state == nw_connection_state_failed) {
            atomic_store(&h->state_code, 3);
            dispatch_semaphore_signal(h->ready);
        }
    });
    nw_connection_start(conn);

    dispatch_time_t deadline = dispatch_time(DISPATCH_TIME_NOW, 30LL * NSEC_PER_SEC);
    if (dispatch_semaphore_wait(h->ready, deadline) != 0) {
        cancel_and_destroy_handle(h);
        if (out_status) *out_status = NW_TIMEOUT;
        return NULL;
    }

    if (atomic_load(&h->state_code) != 1) {
        cancel_and_destroy_handle(h);
        if (out_status) *out_status = NW_CONNECT_FAILED;
        return NULL;
    }

    if (out_status) *out_status = NW_OK;
    return h;
}

static void *nw_shim_wrap_listener_common(nw_listener_t listener, const char *label, int *out_status) {
    if (!listener) {
        if (out_status) *out_status = NW_LISTEN_FAILED;
        return NULL;
    }

    nw_listener_handle *h = (nw_listener_handle *)calloc(1, sizeof(nw_listener_handle));
    if (!h) {
        if (out_status) *out_status = NW_LISTEN_FAILED;
        return NULL;
    }
    h->listener = listener;
    h->queue = dispatch_queue_create(label ? label : "networkframework-rs.listener.wrap", DISPATCH_QUEUE_SERIAL);
    h->ready = dispatch_semaphore_create(0);
    h->accept_sem = dispatch_semaphore_create(0);
    atomic_store(&h->state_code, 0);

    nw_listener_set_queue(listener, h->queue);
    nw_listener_set_state_changed_handler(listener, ^(nw_listener_state_t state, nw_error_t error) {
        (void)error;
        if (state == nw_listener_state_ready) {
            atomic_store(&h->bound_port, nw_listener_get_port(listener));
            atomic_store(&h->state_code, 1);
            dispatch_semaphore_signal(h->ready);
        } else if (state == nw_listener_state_cancelled) {
            atomic_store(&h->state_code, 2);
            dispatch_semaphore_signal(h->ready);
        } else if (state == nw_listener_state_failed) {
            atomic_store(&h->state_code, 3);
            dispatch_semaphore_signal(h->ready);
        }
    });
    nw_listener_set_new_connection_handler(listener, ^(nw_connection_t conn) {
        nw_retain(conn);
        if (h->pending) {
            nw_connection_cancel(h->pending);
            nw_release(h->pending);
        }
        h->pending = conn;
        dispatch_semaphore_signal(h->accept_sem);
    });
    nw_listener_start(listener);

    dispatch_time_t deadline = dispatch_time(DISPATCH_TIME_NOW, 10LL * NSEC_PER_SEC);
    if (dispatch_semaphore_wait(h->ready, deadline) != 0 || atomic_load(&h->state_code) != 1) {
        cancel_and_destroy_listener_handle(h);
        if (out_status) *out_status = NW_LISTEN_FAILED;
        return NULL;
    }

    if (out_status) *out_status = NW_OK;
    return h;
}

static nw_connection_group_handle *nw_shim_wrap_connection_group(nw_connection_group_t group, const char *label) {
    if (!group) {
        return NULL;
    }
    nw_connection_group_handle *h = (nw_connection_group_handle *)calloc(1, sizeof(nw_connection_group_handle));
    if (!h) {
        return NULL;
    }
    h->group = nw_retain(group);
    h->queue = dispatch_queue_create(label ? label : "networkframework-rs.connection-group.wrap", DISPATCH_QUEUE_SERIAL);
    h->state_sem = dispatch_semaphore_create(0);
    atomic_store(&h->state_code, (int)nw_connection_group_state_invalid);
    nw_connection_group_set_queue(h->group, h->queue);
    nw_connection_group_set_state_changed_handler(h->group, ^(nw_connection_group_state_t state, nw_error_t error) {
        (void)error;
        atomic_store(&h->state_code, (int)state);
        dispatch_semaphore_signal(h->state_sem);
        if (h->state_callback) {
            h->state_callback((int)state, h->state_user_info);
        }
    });
    return h;
}

void nw_shim_connection_release_without_cancel(void *handle) {
    nw_conn_handle *h = (nw_conn_handle *)handle;
    if (!h) {
        return;
    }
    destroy_handle(h);
}

void nw_shim_advertise_descriptor_set_txt_record_object(void *descriptor, void *txt_record) {
    if (!descriptor) {
        return;
    }
    nw_advertise_descriptor_set_txt_record_object((nw_advertise_descriptor_t)descriptor, (nw_txt_record_t)txt_record);
}

void *nw_shim_advertise_descriptor_copy_txt_record_object(void *descriptor) {
    if (!descriptor) {
        return NULL;
    }
    return nw_advertise_descriptor_copy_txt_record_object((nw_advertise_descriptor_t)descriptor);
}

void *nw_shim_browser_start_results_with_descriptor(
    void *descriptor,
    void *parameters,
    BrowseResultChangedCallback callback,
    void *user_info
) {
    if (!descriptor || !callback) {
        return NULL;
    }

    nw_browse_descriptor_t desc = nw_retain((nw_browse_descriptor_t)descriptor);
    nw_parameters_t params = parameters ? nw_parameters_copy((nw_parameters_t)parameters) : nw_parameters_create();
    if (!params) {
        nw_release(desc);
        return NULL;
    }

    nw_browser_t browser = nw_browser_create(desc, params);
    nw_release(desc);
    nw_release(params);
    if (!browser) {
        return NULL;
    }

    nw_browser_handle *h = (nw_browser_handle *)calloc(1, sizeof(nw_browser_handle));
    h->browser = browser;
    h->queue = dispatch_queue_create("networkframework-rs.browser.results", DISPATCH_QUEUE_SERIAL);
    h->user_info = user_info;

    nw_browser_set_queue(browser, h->queue);
    nw_browser_set_browse_results_changed_handler(browser, ^(nw_browse_result_t old_result, nw_browse_result_t new_result, bool batch_complete) {
        uint64_t changes = nw_browse_result_get_changes(old_result, new_result);
        void *old_handle = old_result ? nw_retain(old_result) : NULL;
        void *new_handle = new_result ? nw_retain(new_result) : NULL;
        callback(old_handle, new_handle, changes, batch_complete ? 1 : 0, user_info);
    });
    nw_browser_start(browser);
    return h;
}

void *nw_shim_browser_copy_browse_descriptor(void *handle) {
    nw_browser_handle *h = (nw_browser_handle *)handle;
    if (!h) {
        return NULL;
    }
    return nw_browser_copy_browse_descriptor(h->browser);
}

void *nw_shim_browser_copy_parameters(void *handle) {
    nw_browser_handle *h = (nw_browser_handle *)handle;
    if (!h) {
        return NULL;
    }
    return nw_browser_copy_parameters(h->browser);
}

void nw_shim_browser_set_state_changed_handler(void *handle, BrowserStateChangedCallback callback, void *user_info) {
    nw_browser_handle *h = (nw_browser_handle *)handle;
    if (!h) {
        return;
    }
    if (!callback) {
        nw_browser_set_state_changed_handler(h->browser, NULL);
        return;
    }
    nw_browser_set_state_changed_handler(h->browser, ^(nw_browser_state_t state, nw_error_t error) {
        void *retained_error = error ? nw_retain(error) : NULL;
        callback((int)state, retained_error, user_info);
    });
}

uint64_t nw_shim_browse_result_get_changes(void *old_result, void *new_result) {
    return nw_browse_result_get_changes((nw_browse_result_t)old_result, (nw_browse_result_t)new_result);
}

void *nw_shim_browse_result_copy_endpoint(void *result) {
    if (!result) {
        return NULL;
    }
    return nw_browse_result_copy_endpoint((nw_browse_result_t)result);
}

size_t nw_shim_browse_result_get_interfaces_count(void *result) {
    if (!result) {
        return 0;
    }
    return nw_browse_result_get_interfaces_count((nw_browse_result_t)result);
}

void *nw_shim_browse_result_copy_txt_record_object(void *result) {
    if (!result) {
        return NULL;
    }
    return nw_browse_result_copy_txt_record_object((nw_browse_result_t)result);
}

int nw_shim_browse_result_enumerate_interfaces(void *result, InterfaceEnumerationCallback callback, void *user_info) {
    if (!result || !callback) {
        return 0;
    }
    __block int count = 0;
    nw_browse_result_enumerate_interfaces((nw_browse_result_t)result, ^bool(nw_interface_t interface) {
        int keep_going = nw_shim_invoke_interface_callback(interface, callback, user_info);
        count += 1;
        return keep_going != 0;
    });
    return count;
}

void nw_shim_connection_set_viability_changed_handler(void *handle, ConnectionBooleanCallback callback, void *user_info) {
    nw_conn_handle *h = (nw_conn_handle *)handle;
    if (!h) {
        return;
    }
    if (!callback) {
        nw_connection_set_viability_changed_handler(h->conn, NULL);
        return;
    }
    nw_connection_set_viability_changed_handler(h->conn, ^(bool value) {
        callback(value ? 1 : 0, user_info);
    });
}

void nw_shim_connection_set_better_path_available_handler(void *handle, ConnectionBooleanCallback callback, void *user_info) {
    nw_conn_handle *h = (nw_conn_handle *)handle;
    if (!h) {
        return;
    }
    if (!callback) {
        nw_connection_set_better_path_available_handler(h->conn, NULL);
        return;
    }
    nw_connection_set_better_path_available_handler(h->conn, ^(bool value) {
        callback(value ? 1 : 0, user_info);
    });
}

void nw_shim_connection_set_path_changed_handler(void *handle, ConnectionPathCallback callback, void *user_info) {
    nw_conn_handle *h = (nw_conn_handle *)handle;
    if (!h) {
        return;
    }
    if (!callback) {
        nw_connection_set_path_changed_handler(h->conn, NULL);
        return;
    }
    nw_connection_set_path_changed_handler(h->conn, ^(nw_path_t path) {
        void *retained_path = path ? nw_retain(path) : NULL;
        callback(retained_path, user_info);
    });
}

void nw_shim_connection_restart(void *handle) {
    nw_conn_handle *h = (nw_conn_handle *)handle;
    if (h) {
        nw_connection_restart(h->conn);
    }
}

void nw_shim_connection_force_cancel(void *handle) {
    nw_conn_handle *h = (nw_conn_handle *)handle;
    if (h) {
        nw_connection_force_cancel(h->conn);
    }
}

void nw_shim_connection_cancel_current_endpoint(void *handle) {
    nw_conn_handle *h = (nw_conn_handle *)handle;
    if (h) {
        nw_connection_cancel_current_endpoint(h->conn);
    }
}

void nw_shim_connection_batch(void *handle, ConnectionBatchCallback callback, void *user_info) {
    nw_conn_handle *h = (nw_conn_handle *)handle;
    if (!h) {
        return;
    }
    if (!callback) {
        nw_connection_batch(h->conn, ^{});
        return;
    }
    nw_connection_batch(h->conn, ^{
        callback(user_info);
    });
}

char *nw_shim_connection_copy_description(void *handle) {
    nw_conn_handle *h = (nw_conn_handle *)handle;
    if (!h) {
        return NULL;
    }
    return nw_connection_copy_description(h->conn);
}

void *nw_shim_connection_copy_protocol_metadata(void *handle, void *definition) {
    nw_conn_handle *h = (nw_conn_handle *)handle;
    if (!h || !definition) {
        return NULL;
    }
    return nw_connection_copy_protocol_metadata(h->conn, (nw_protocol_definition_t)definition);
}

uint32_t nw_shim_connection_get_maximum_datagram_size(void *handle) {
    nw_conn_handle *h = (nw_conn_handle *)handle;
    if (!h) {
        return 0;
    }
    return nw_connection_get_maximum_datagram_size(h->conn);
}

void nw_shim_content_context_foreach_protocol_metadata(
    void *context,
    ProtocolMetadataEnumerationCallback callback,
    void *user_info
) {
    if (!context || !callback) {
        return;
    }
    nw_content_context_foreach_protocol_metadata((nw_content_context_t)context, ^(nw_protocol_definition_t definition, nw_protocol_metadata_t metadata) {
        void *retained_definition = definition ? nw_retain(definition) : NULL;
        void *retained_metadata = metadata ? nw_retain(metadata) : NULL;
        (void)callback(retained_definition, retained_metadata, user_info);
    });
}

void *nw_shim_framer_copy_remote_endpoint(void *framer) {
    if (!framer) {
        return NULL;
    }
    if (__builtin_available(macOS 10.15, *)) {
        return nw_framer_copy_remote_endpoint((nw_framer_t)framer);
    }
    return NULL;
}

void *nw_shim_framer_copy_local_endpoint(void *framer) {
    if (!framer) {
        return NULL;
    }
    if (__builtin_available(macOS 10.15, *)) {
        return nw_framer_copy_local_endpoint((nw_framer_t)framer);
    }
    return NULL;
}

void *nw_shim_framer_copy_parameters(void *framer) {
    if (!framer) {
        return NULL;
    }
    if (__builtin_available(macOS 10.15, *)) {
        return nw_framer_copy_parameters((nw_framer_t)framer);
    }
    return NULL;
}

void *nw_shim_framer_copy_options(void *framer) {
    if (!framer) {
        return NULL;
    }
    if (__builtin_available(macOS 12.3, *)) {
        return nw_framer_copy_options((nw_framer_t)framer);
    }
    return NULL;
}

int nw_shim_group_descriptor_enumerate_endpoints(void *descriptor, EndpointEnumerationCallback callback, void *user_info) {
    if (!descriptor || !callback) {
        return 0;
    }
    __block int count = 0;
    nw_group_descriptor_enumerate_endpoints((nw_group_descriptor_t)descriptor, ^bool(nw_endpoint_t endpoint) {
        int keep_going = nw_shim_invoke_endpoint_callback(endpoint, callback, user_info);
        count += 1;
        return keep_going != 0;
    });
    return count;
}

void nw_shim_multicast_group_descriptor_set_specific_source(void *descriptor, void *endpoint) {
    if (!descriptor || !endpoint) {
        return;
    }
    nw_multicast_group_descriptor_set_specific_source((nw_group_descriptor_t)descriptor, (nw_endpoint_t)endpoint);
}

int nw_shim_multicast_group_descriptor_get_disable_unicast_traffic(void *descriptor) {
    if (!descriptor) {
        return 0;
    }
    return nw_multicast_group_descriptor_get_disable_unicast_traffic((nw_group_descriptor_t)descriptor) ? 1 : 0;
}

void nw_shim_multicast_group_descriptor_set_disable_unicast_traffic(void *descriptor, int disable_unicast_traffic) {
    if (!descriptor) {
        return;
    }
    nw_multicast_group_descriptor_set_disable_unicast_traffic((nw_group_descriptor_t)descriptor, disable_unicast_traffic != 0);
}

void *nw_shim_connection_group_copy_descriptor(void *handle) {
    nw_connection_group_handle *h = (nw_connection_group_handle *)handle;
    if (!h) {
        return NULL;
    }
    return nw_connection_group_copy_descriptor(h->group);
}

void *nw_shim_connection_group_copy_parameters(void *handle) {
    nw_connection_group_handle *h = (nw_connection_group_handle *)handle;
    if (!h) {
        return NULL;
    }
    return nw_connection_group_copy_parameters(h->group);
}

void *nw_shim_connection_group_copy_remote_endpoint_for_message(void *handle, void *context) {
    nw_connection_group_handle *h = (nw_connection_group_handle *)handle;
    if (!h || !context) {
        return NULL;
    }
    return nw_connection_group_copy_remote_endpoint_for_message(h->group, (nw_content_context_t)context);
}

void *nw_shim_connection_group_copy_local_endpoint_for_message(void *handle, void *context) {
    nw_connection_group_handle *h = (nw_connection_group_handle *)handle;
    if (!h || !context) {
        return NULL;
    }
    return nw_connection_group_copy_local_endpoint_for_message(h->group, (nw_content_context_t)context);
}

void *nw_shim_connection_group_copy_path_for_message(void *handle, void *context) {
    nw_connection_group_handle *h = (nw_connection_group_handle *)handle;
    if (!h || !context) {
        return NULL;
    }
    return nw_connection_group_copy_path_for_message(h->group, (nw_content_context_t)context);
}

void *nw_shim_connection_group_copy_protocol_metadata_for_message(void *handle, void *context, void *definition) {
    nw_connection_group_handle *h = (nw_connection_group_handle *)handle;
    if (!h || !context || !definition) {
        return NULL;
    }
    if (__builtin_available(macOS 12.0, *)) {
        return nw_connection_group_copy_protocol_metadata_for_message(
            h->group,
            (nw_content_context_t)context,
            (nw_protocol_definition_t)definition);
    }
    return NULL;
}

void *nw_shim_connection_group_copy_protocol_metadata(void *handle, void *definition) {
    nw_connection_group_handle *h = (nw_connection_group_handle *)handle;
    if (!h || !definition) {
        return NULL;
    }
    if (__builtin_available(macOS 12.0, *)) {
        return nw_connection_group_copy_protocol_metadata(h->group, (nw_protocol_definition_t)definition);
    }
    return NULL;
}

void *nw_shim_connection_group_extract_connection_for_message(void *handle, void *context, int *out_status) {
    nw_connection_group_handle *h = (nw_connection_group_handle *)handle;
    if (!h || !context) {
        if (out_status) *out_status = NW_INVALID_ARG;
        return NULL;
    }
    nw_connection_t connection = nw_connection_group_extract_connection_for_message(h->group, (nw_content_context_t)context);
    if (!connection) {
        if (out_status) *out_status = NW_CONNECT_FAILED;
        return NULL;
    }
    return nw_shim_wrap_started_connection(connection, "networkframework-rs.connection-group.message", out_status);
}

void *nw_shim_connection_group_extract_connection(void *handle, void *endpoint, void *protocol_options, int *out_status) {
    nw_connection_group_handle *h = (nw_connection_group_handle *)handle;
    if (!h) {
        if (out_status) *out_status = NW_INVALID_ARG;
        return NULL;
    }
    if (__builtin_available(macOS 12.0, *)) {
        nw_connection_t connection = nw_connection_group_extract_connection(
            h->group,
            (nw_endpoint_t)endpoint,
            (nw_protocol_options_t)protocol_options);
        if (!connection) {
            if (out_status) *out_status = NW_CONNECT_FAILED;
            return NULL;
        }
        return nw_shim_wrap_started_connection(connection, "networkframework-rs.connection-group.extract", out_status);
    }
    if (out_status) *out_status = NW_INVALID_ARG;
    return NULL;
}

int nw_shim_connection_group_reinsert_extracted_connection(void *handle, void *connection_handle) {
    nw_connection_group_handle *h = (nw_connection_group_handle *)handle;
    nw_conn_handle *connection = (nw_conn_handle *)connection_handle;
    if (!h || !connection || !connection->conn) {
        return 0;
    }
    if (__builtin_available(macOS 12.0, *)) {
        return nw_connection_group_reinsert_extracted_connection(h->group, connection->conn) ? 1 : 0;
    }
    return 0;
}

int nw_shim_connection_group_reply(
    void *handle,
    void *inbound_message,
    void *outbound_message,
    const uint8_t *data,
    size_t len
) {
    nw_connection_group_handle *h = (nw_connection_group_handle *)handle;
    if (!h || !inbound_message || !outbound_message) {
        return NW_INVALID_ARG;
    }
    dispatch_data_t payload = NULL;
    if (data || len != 0) {
        payload = dispatch_data_create(data, len, h->queue, DISPATCH_DATA_DESTRUCTOR_DEFAULT);
    }
    nw_connection_group_reply(h->group, (nw_content_context_t)inbound_message, (nw_content_context_t)outbound_message, payload);
    if (payload) {
        dispatch_release(payload);
    }
    return NW_OK;
}

void nw_shim_connection_group_set_new_connection_handler(
    void *handle,
    ConnectionGroupNewConnectionCallback callback,
    void *user_info
) {
    nw_connection_group_handle *h = (nw_connection_group_handle *)handle;
    if (!h) {
        return;
    }
    if (!callback) {
        if (__builtin_available(macOS 12.0, *)) {
            nw_connection_group_set_new_connection_handler(h->group, NULL);
        }
        return;
    }
    if (__builtin_available(macOS 12.0, *)) {
        nw_connection_group_set_new_connection_handler(h->group, ^(nw_connection_t connection) {
            int status = NW_OK;
            nw_connection_t retained = nw_retain(connection);
            nw_conn_handle *wrapped = nw_shim_wrap_started_connection(retained, "networkframework-rs.connection-group.new-connection", &status);
            if (wrapped) {
                callback(wrapped, user_info);
            }
        });
    }
}

int nw_shim_error_get_domain(void *error) {
    if (!error) {
        return 0;
    }
    return (int)nw_error_get_error_domain((nw_error_t)error);
}

int nw_shim_error_get_code(void *error) {
    if (!error) {
        return 0;
    }
    return nw_error_get_error_code((nw_error_t)error);
}

void *nw_shim_error_copy_cf_error(void *error) {
    if (!error) {
        return NULL;
    }
    return (void *)nw_error_copy_cf_error((nw_error_t)error);
}

char *nw_shim_error_copy_cf_error_domain(void *error) {
    if (!error) {
        return NULL;
    }
    CFErrorRef cf_error = nw_error_copy_cf_error((nw_error_t)error);
    if (!cf_error) {
        return NULL;
    }
    char *result = nw_shim_copy_cfstring_value(CFErrorGetDomain(cf_error));
    CFRelease(cf_error);
    return result;
}

char *nw_shim_error_copy_cf_error_description(void *error) {
    if (!error) {
        return NULL;
    }
    CFErrorRef cf_error = nw_error_copy_cf_error((nw_error_t)error);
    if (!cf_error) {
        return NULL;
    }
    CFStringRef description = CFErrorCopyDescription(cf_error);
    CFRelease(cf_error);
    char *result = nw_shim_copy_cfstring_value(description);
    if (description) {
        CFRelease(description);
    }
    return result;
}

char *nw_shim_error_copy_posix_domain(void) {
    return nw_shim_copy_cfstring_value(kNWErrorDomainPOSIX);
}

char *nw_shim_error_copy_dns_domain(void) {
    return nw_shim_copy_cfstring_value(kNWErrorDomainDNS);
}

char *nw_shim_error_copy_tls_domain(void) {
    return nw_shim_copy_cfstring_value(kNWErrorDomainTLS);
}

char *nw_shim_error_copy_wifi_aware_domain(void) {
    if (__builtin_available(macOS 26.0, *)) {
        return nw_shim_copy_cfstring_value(kNWErrorDomainWiFiAware);
    }
    return NULL;
}

void *nw_shim_parameters_create_custom_ip(uint8_t protocol_number) {
    if (__builtin_available(macOS 10.15, *)) {
        return nw_parameters_create_custom_ip(protocol_number, NW_PARAMETERS_DEFAULT_CONFIGURATION);
    }
    return NULL;
}

void *nw_shim_listener_create_direct(void *parameters, int *out_status) {
    if (!parameters) {
        if (out_status) *out_status = NW_INVALID_ARG;
        return NULL;
    }
    nw_listener_t listener = nw_listener_create((nw_parameters_t)parameters);
    return nw_shim_wrap_listener_common(listener, "networkframework-rs.listener.direct", out_status);
}

void *nw_shim_listener_create_with_connection(void *connection_handle, void *parameters, int *out_status) {
    nw_conn_handle *connection = (nw_conn_handle *)connection_handle;
    if (!connection || !connection->conn || !parameters) {
        if (out_status) *out_status = NW_INVALID_ARG;
        return NULL;
    }
    if (__builtin_available(macOS 11.0, *)) {
        nw_listener_t listener = nw_listener_create_with_connection(connection->conn, (nw_parameters_t)parameters);
        return nw_shim_wrap_listener_common(listener, "networkframework-rs.listener.connection", out_status);
    }
    if (out_status) *out_status = NW_INVALID_ARG;
    return NULL;
}

void *nw_shim_listener_create_with_launchd_key(void *parameters, const char *launchd_key, int *out_status) {
    if (!parameters || !launchd_key) {
        if (out_status) *out_status = NW_INVALID_ARG;
        return NULL;
    }
    nw_listener_t listener = nw_listener_create_with_launchd_key((nw_parameters_t)parameters, launchd_key);
    return nw_shim_wrap_listener_common(listener, "networkframework-rs.listener.launchd", out_status);
}

uint32_t nw_shim_listener_get_new_connection_limit(void *handle) {
    nw_listener_handle *h = (nw_listener_handle *)handle;
    if (!h) {
        return 0;
    }
    return nw_listener_get_new_connection_limit(h->listener);
}

void nw_shim_listener_set_new_connection_limit(void *handle, uint32_t new_connection_limit) {
    nw_listener_handle *h = (nw_listener_handle *)handle;
    if (h) {
        nw_listener_set_new_connection_limit(h->listener, new_connection_limit);
    }
}

void nw_shim_listener_set_advertised_endpoint_changed_handler(
    void *handle,
    ListenerAdvertisedEndpointChangedCallback callback,
    void *user_info
) {
    nw_listener_handle *h = (nw_listener_handle *)handle;
    if (!h) {
        return;
    }
    if (!callback) {
        nw_listener_set_advertised_endpoint_changed_handler(h->listener, NULL);
        return;
    }
    nw_listener_set_advertised_endpoint_changed_handler(h->listener, ^(nw_endpoint_t endpoint, bool added) {
        void *retained_endpoint = endpoint ? nw_retain(endpoint) : NULL;
        callback(retained_endpoint, added ? 1 : 0, user_info);
    });
}

void nw_shim_listener_set_new_connection_group_handler(
    void *handle,
    ListenerNewConnectionGroupCallback callback,
    void *user_info
) {
    nw_listener_handle *h = (nw_listener_handle *)handle;
    if (!h) {
        return;
    }
    if (__builtin_available(macOS 12.0, *)) {
        if (!callback) {
            nw_listener_set_new_connection_group_handler(h->listener, NULL);
            return;
        }
        nw_listener_set_new_connection_group_handler(h->listener, ^(nw_connection_group_t group) {
            nw_connection_group_handle *wrapped = nw_shim_wrap_connection_group(group, "networkframework-rs.listener.group");
            if (wrapped) {
                callback(wrapped, user_info);
            }
        });
    }
}

int nw_shim_path_enumerate_gateways(void *path, EndpointEnumerationCallback callback, void *user_info) {
    if (!path || !callback) {
        return 0;
    }
    __block int count = 0;
    if (__builtin_available(macOS 11.0, *)) {
        nw_path_enumerate_gateways((nw_path_t)path, ^bool(nw_endpoint_t gateway) {
            int keep_going = nw_shim_invoke_endpoint_callback(gateway, callback, user_info);
            count += 1;
            return keep_going != 0;
        });
    }
    return count;
}

static void *nw_shim_start_path_monitor_with_monitor(
    nw_path_monitor_t monitor,
    PathMonitorCallback callback,
    void *user_info,
    const char *label
) {
    if (!monitor) {
        return NULL;
    }
    nw_path_handle *h = (nw_path_handle *)calloc(1, sizeof(nw_path_handle));
    h->monitor = monitor;
    h->queue = dispatch_queue_create(label, DISPATCH_QUEUE_SERIAL);
    h->callback = callback;
    h->user_info = user_info;
    nw_path_monitor_set_queue(h->monitor, h->queue);
    nw_path_monitor_set_update_handler(h->monitor, ^(nw_path_t path) {
        if (h->latest_path) {
            nw_release(h->latest_path);
            h->latest_path = NULL;
        }
        if (path) {
            h->latest_path = nw_retain(path);
        }
        if (!h->callback) return;
        int satisfied = (nw_path_get_status(path) == nw_path_status_satisfied) ? 1 : 0;
        int iface = 0;
        if (nw_path_uses_interface_type(path, nw_interface_type_wifi)) iface = 1;
        else if (nw_path_uses_interface_type(path, nw_interface_type_cellular)) iface = 2;
        else if (nw_path_uses_interface_type(path, nw_interface_type_wired)) iface = 3;
        else if (nw_path_uses_interface_type(path, nw_interface_type_loopback)) iface = 4;
        h->callback(satisfied, iface, h->user_info);
    });
    nw_path_monitor_start(h->monitor);
    return h;
}

void *nw_shim_path_monitor_start_with_type(int interface_type, PathMonitorCallback callback, void *user_info) {
    return nw_shim_start_path_monitor_with_monitor(
        nw_path_monitor_create_with_type((nw_interface_type_t)interface_type),
        callback,
        user_info,
        "networkframework-rs.path.type");
}

void *nw_shim_path_monitor_start_for_ethernet_channel(PathMonitorCallback callback, void *user_info) {
    if (__builtin_available(macOS 13.0, *)) {
        return nw_shim_start_path_monitor_with_monitor(
            nw_path_monitor_create_for_ethernet_channel(),
            callback,
            user_info,
            "networkframework-rs.path.ethernet");
    }
    return NULL;
}

void nw_shim_path_monitor_prohibit_interface_type(void *handle, int interface_type) {
    nw_path_handle *h = (nw_path_handle *)handle;
    if (!h) {
        return;
    }
    if (__builtin_available(macOS 11.0, *)) {
        nw_path_monitor_prohibit_interface_type(h->monitor, (nw_interface_type_t)interface_type);
    }
}

void nw_shim_path_monitor_set_cancel_handler(void *handle, PathMonitorCancelCallback callback, void *user_info) {
    nw_path_handle *h = (nw_path_handle *)handle;
    if (!h) {
        return;
    }
    if (!callback) {
        nw_path_monitor_set_cancel_handler(h->monitor, NULL);
        return;
    }
    nw_path_monitor_set_cancel_handler(h->monitor, ^{
        callback(user_info);
    });
}

void *nw_shim_protocol_create_ip_metadata(void) {
    return nw_ip_create_metadata();
}

void *nw_shim_protocol_create_udp_metadata(void) {
    return nw_udp_create_metadata();
}

void *nw_shim_protocol_create_ws_options_with_version(int version) {
    if (__builtin_available(macOS 10.15, *)) {
        return nw_ws_create_options((nw_ws_version_t)version);
    }
    return NULL;
}

void *nw_shim_protocol_create_ws_metadata(int opcode) {
    if (__builtin_available(macOS 10.15, *)) {
        return nw_ws_create_metadata((nw_ws_opcode_t)opcode);
    }
    return NULL;
}

int nw_shim_protocol_metadata_is_ip(void *metadata) {
    return metadata ? (nw_protocol_metadata_is_ip((nw_protocol_metadata_t)metadata) ? 1 : 0) : 0;
}

int nw_shim_protocol_metadata_is_tcp(void *metadata) {
    return metadata ? (nw_protocol_metadata_is_tcp((nw_protocol_metadata_t)metadata) ? 1 : 0) : 0;
}

int nw_shim_protocol_metadata_is_tls(void *metadata) {
    return metadata ? (nw_protocol_metadata_is_tls((nw_protocol_metadata_t)metadata) ? 1 : 0) : 0;
}

int nw_shim_protocol_metadata_is_udp(void *metadata) {
    return metadata ? (nw_protocol_metadata_is_udp((nw_protocol_metadata_t)metadata) ? 1 : 0) : 0;
}

int nw_shim_protocol_metadata_is_ws(void *metadata) {
    if (!metadata) {
        return 0;
    }
    if (__builtin_available(macOS 10.15, *)) {
        return nw_protocol_metadata_is_ws((nw_protocol_metadata_t)metadata) ? 1 : 0;
    }
    return 0;
}

#define NW_SHIM_BOOL_PROTOCOL_OPTION_SETTER(name, fn) \
    void name(void *options, int value) { if (options) { fn((nw_protocol_options_t)options, value != 0); } }
#define NW_SHIM_U32_PROTOCOL_OPTION_SETTER(name, fn) \
    void name(void *options, uint32_t value) { if (options) { fn((nw_protocol_options_t)options, value); } }
#define NW_SHIM_U8_PROTOCOL_OPTION_SETTER(name, fn) \
    void name(void *options, uint8_t value) { if (options) { fn((nw_protocol_options_t)options, value); } }

void nw_shim_ip_options_set_version(void *options, int version) {
    if (options) {
        nw_ip_options_set_version((nw_protocol_options_t)options, (nw_ip_version_t)version);
    }
}
NW_SHIM_U8_PROTOCOL_OPTION_SETTER(nw_shim_ip_options_set_hop_limit, nw_ip_options_set_hop_limit)
NW_SHIM_BOOL_PROTOCOL_OPTION_SETTER(nw_shim_ip_options_set_use_minimum_mtu, nw_ip_options_set_use_minimum_mtu)
NW_SHIM_BOOL_PROTOCOL_OPTION_SETTER(nw_shim_ip_options_set_disable_fragmentation, nw_ip_options_set_disable_fragmentation)
NW_SHIM_BOOL_PROTOCOL_OPTION_SETTER(nw_shim_ip_options_set_calculate_receive_time, nw_ip_options_set_calculate_receive_time)
void nw_shim_ip_options_set_local_address_preference(void *options, int preference) {
    if (options && __builtin_available(macOS 10.15, *)) {
        nw_ip_options_set_local_address_preference((nw_protocol_options_t)options, (nw_ip_local_address_preference_t)preference);
    }
}
NW_SHIM_BOOL_PROTOCOL_OPTION_SETTER(nw_shim_ip_options_set_disable_multicast_loopback, nw_ip_options_set_disable_multicast_loopback)
void nw_shim_ip_metadata_set_ecn_flag(void *metadata, int ecn_flag) {
    if (metadata) {
        nw_ip_metadata_set_ecn_flag((nw_protocol_metadata_t)metadata, (nw_ip_ecn_flag_t)ecn_flag);
    }
}
int nw_shim_ip_metadata_get_ecn_flag(void *metadata) {
    return metadata ? (int)nw_ip_metadata_get_ecn_flag((nw_protocol_metadata_t)metadata) : 0;
}
void nw_shim_ip_metadata_set_service_class(void *metadata, int service_class) {
    if (metadata) {
        nw_ip_metadata_set_service_class((nw_protocol_metadata_t)metadata, (nw_service_class_t)service_class);
    }
}
int nw_shim_ip_metadata_get_service_class(void *metadata) {
    return metadata ? (int)nw_ip_metadata_get_service_class((nw_protocol_metadata_t)metadata) : 0;
}
uint64_t nw_shim_ip_metadata_get_receive_time(void *metadata) {
    return metadata ? nw_ip_metadata_get_receive_time((nw_protocol_metadata_t)metadata) : 0;
}

NW_SHIM_BOOL_PROTOCOL_OPTION_SETTER(nw_shim_tcp_options_set_no_delay, nw_tcp_options_set_no_delay)
NW_SHIM_BOOL_PROTOCOL_OPTION_SETTER(nw_shim_tcp_options_set_no_push, nw_tcp_options_set_no_push)
NW_SHIM_BOOL_PROTOCOL_OPTION_SETTER(nw_shim_tcp_options_set_no_options, nw_tcp_options_set_no_options)
NW_SHIM_BOOL_PROTOCOL_OPTION_SETTER(nw_shim_tcp_options_set_enable_keepalive, nw_tcp_options_set_enable_keepalive)
NW_SHIM_U32_PROTOCOL_OPTION_SETTER(nw_shim_tcp_options_set_keepalive_count, nw_tcp_options_set_keepalive_count)
NW_SHIM_U32_PROTOCOL_OPTION_SETTER(nw_shim_tcp_options_set_keepalive_idle_time, nw_tcp_options_set_keepalive_idle_time)
NW_SHIM_U32_PROTOCOL_OPTION_SETTER(nw_shim_tcp_options_set_keepalive_interval, nw_tcp_options_set_keepalive_interval)
NW_SHIM_U32_PROTOCOL_OPTION_SETTER(nw_shim_tcp_options_set_maximum_segment_size, nw_tcp_options_set_maximum_segment_size)
NW_SHIM_U32_PROTOCOL_OPTION_SETTER(nw_shim_tcp_options_set_connection_timeout, nw_tcp_options_set_connection_timeout)
NW_SHIM_U32_PROTOCOL_OPTION_SETTER(nw_shim_tcp_options_set_persist_timeout, nw_tcp_options_set_persist_timeout)
NW_SHIM_U32_PROTOCOL_OPTION_SETTER(nw_shim_tcp_options_set_retransmit_connection_drop_time, nw_tcp_options_set_retransmit_connection_drop_time)
NW_SHIM_BOOL_PROTOCOL_OPTION_SETTER(nw_shim_tcp_options_set_retransmit_fin_drop, nw_tcp_options_set_retransmit_fin_drop)
NW_SHIM_BOOL_PROTOCOL_OPTION_SETTER(nw_shim_tcp_options_set_disable_ack_stretching, nw_tcp_options_set_disable_ack_stretching)
NW_SHIM_BOOL_PROTOCOL_OPTION_SETTER(nw_shim_tcp_options_set_enable_fast_open, nw_tcp_options_set_enable_fast_open)
NW_SHIM_BOOL_PROTOCOL_OPTION_SETTER(nw_shim_tcp_options_set_disable_ecn, nw_tcp_options_set_disable_ecn)
void nw_shim_tcp_options_set_multipath_force_version(void *options, int multipath_force_version) {
    if (options && __builtin_available(macOS 12.0, *)) {
        nw_tcp_options_set_multipath_force_version((nw_protocol_options_t)options, (nw_multipath_version_t)multipath_force_version);
    }
}
uint32_t nw_shim_tcp_get_available_receive_buffer(void *metadata) {
    return metadata ? nw_tcp_get_available_receive_buffer((nw_protocol_metadata_t)metadata) : 0;
}
uint32_t nw_shim_tcp_get_available_send_buffer(void *metadata) {
    return metadata ? nw_tcp_get_available_send_buffer((nw_protocol_metadata_t)metadata) : 0;
}

void *nw_shim_tls_copy_sec_protocol_options(void *options) {
    return options ? nw_tls_copy_sec_protocol_options((nw_protocol_options_t)options) : NULL;
}

void *nw_shim_tls_copy_sec_protocol_metadata(void *metadata) {
    return metadata ? nw_tls_copy_sec_protocol_metadata((nw_protocol_metadata_t)metadata) : NULL;
}

NW_SHIM_BOOL_PROTOCOL_OPTION_SETTER(nw_shim_udp_options_set_prefer_no_checksum, nw_udp_options_set_prefer_no_checksum)

void nw_shim_ws_options_add_additional_header(void *options, const char *name, const char *value) {
    if (options && name && value && __builtin_available(macOS 10.15, *)) {
        nw_ws_options_add_additional_header((nw_protocol_options_t)options, name, value);
    }
}

void nw_shim_ws_options_add_subprotocol(void *options, const char *subprotocol) {
    if (options && subprotocol && __builtin_available(macOS 10.15, *)) {
        nw_ws_options_add_subprotocol((nw_protocol_options_t)options, subprotocol);
    }
}

NW_SHIM_BOOL_PROTOCOL_OPTION_SETTER(nw_shim_ws_options_set_auto_reply_ping, nw_ws_options_set_auto_reply_ping)
NW_SHIM_BOOL_PROTOCOL_OPTION_SETTER(nw_shim_ws_options_set_skip_handshake, nw_ws_options_set_skip_handshake)
void nw_shim_ws_options_set_maximum_message_size(void *options, size_t maximum_message_size) {
    if (options && __builtin_available(macOS 10.15, *)) {
        nw_ws_options_set_maximum_message_size((nw_protocol_options_t)options, maximum_message_size);
    }
}

void nw_shim_ws_metadata_set_close_code(void *metadata, int close_code) {
    if (metadata && __builtin_available(macOS 10.15, *)) {
        nw_ws_metadata_set_close_code((nw_protocol_metadata_t)metadata, (nw_ws_close_code_t)close_code);
    }
}

int nw_shim_ws_metadata_get_close_code(void *metadata) {
    if (!metadata) {
        return 0;
    }
    if (__builtin_available(macOS 10.15, *)) {
        return (int)nw_ws_metadata_get_close_code((nw_protocol_metadata_t)metadata);
    }
    return 0;
}

void *nw_shim_ws_metadata_copy_server_response(void *metadata) {
    if (!metadata) {
        return NULL;
    }
    if (__builtin_available(macOS 10.15, *)) {
        return nw_ws_metadata_copy_server_response((nw_protocol_metadata_t)metadata);
    }
    return NULL;
}

void nw_shim_ws_metadata_set_pong_handler(void *metadata, WsPongCallback callback, void *user_info) {
    if (!metadata || !__builtin_available(macOS 10.15, *)) {
        return;
    }
    if (!callback) {
        nw_ws_metadata_set_pong_handler((nw_protocol_metadata_t)metadata, dispatch_get_main_queue(), NULL);
        return;
    }
    dispatch_queue_t queue = dispatch_queue_create("networkframework-rs.ws.pong", DISPATCH_QUEUE_SERIAL);
    nw_ws_metadata_set_pong_handler((nw_protocol_metadata_t)metadata, queue, ^(nw_error_t error) {
        void *retained_error = error ? nw_retain(error) : NULL;
        callback(retained_error, user_info);
    });
    dispatch_release(queue);
}

int nw_shim_ws_request_enumerate_subprotocols(void *request, StringEnumerationCallback callback, void *user_info) {
    if (!request || !callback) {
        return 0;
    }
    if (!__builtin_available(macOS 10.15, *)) {
        return 0;
    }
    __block int count = 0;
    bool completed = nw_ws_request_enumerate_subprotocols((nw_ws_request_t)request, ^bool(const char *subprotocol) {
        callback(subprotocol ? subprotocol : "", user_info);
        count += 1;
        return true;
    });
    return completed ? count : -count;
}

int nw_shim_ws_request_enumerate_additional_headers(void *request, HeaderEnumerationCallback callback, void *user_info) {
    if (!request || !callback) {
        return 0;
    }
    if (!__builtin_available(macOS 10.15, *)) {
        return 0;
    }
    __block int count = 0;
    bool completed = nw_ws_request_enumerate_additional_headers((nw_ws_request_t)request, ^bool(const char *name, const char *value) {
        int keep_going = callback(name ? name : "", value ? value : "", user_info);
        count += 1;
        return keep_going != 0;
    });
    return completed ? count : -count;
}

void *nw_shim_ws_response_create(int status, const char *selected_subprotocol) {
    if (__builtin_available(macOS 10.15, *)) {
        return nw_ws_response_create((nw_ws_response_status_t)status, selected_subprotocol);
    }
    return NULL;
}

int nw_shim_ws_response_get_status(void *response) {
    if (!__builtin_available(macOS 10.15, *)) {
        return 0;
    }
    return (int)nw_ws_response_get_status((nw_ws_response_t)response);
}

char *nw_shim_ws_response_get_selected_subprotocol(void *response) {
    if (!response) {
        return NULL;
    }
    if (!__builtin_available(macOS 10.15, *)) {
        return NULL;
    }
    const char *value = nw_ws_response_get_selected_subprotocol((nw_ws_response_t)response);
    return value ? strdup(value) : NULL;
}

void nw_shim_ws_response_add_additional_header(void *response, const char *name, const char *value) {
    if (response && name && value && __builtin_available(macOS 10.15, *)) {
        nw_ws_response_add_additional_header((nw_ws_response_t)response, name, value);
    }
}

int nw_shim_ws_response_enumerate_additional_headers(void *response, HeaderEnumerationCallback callback, void *user_info) {
    if (!response || !callback) {
        return 0;
    }
    if (!__builtin_available(macOS 10.15, *)) {
        return 0;
    }
    __block int count = 0;
    bool completed = nw_ws_response_enumerate_additional_headers((nw_ws_response_t)response, ^bool(const char *name, const char *value) {
        int keep_going = callback(name ? name : "", value ? value : "", user_info);
        count += 1;
        return keep_going != 0;
    });
    return completed ? count : -count;
}

// --- Async-stream helper shims ---

void nw_shim_connection_set_state_changed_handler(void *handle, ConnectionStateCallback callback, void *user_info) {
    nw_conn_handle *h = (nw_conn_handle *)handle;
    if (!h) {
        return;
    }
    if (!callback) {
        nw_connection_set_state_changed_handler(h->conn, NULL);
        return;
    }
    nw_connection_set_state_changed_handler(h->conn, ^(nw_connection_state_t state, nw_error_t error) {
        if (state == nw_connection_state_ready) {
            atomic_store(&h->state_code, 1);
            dispatch_semaphore_signal(h->ready);
        } else if (state == nw_connection_state_cancelled) {
            atomic_store(&h->state_code, 2);
            dispatch_semaphore_signal(h->ready);
        } else if (state == nw_connection_state_failed) {
            atomic_store(&h->state_code, 3);
            dispatch_semaphore_signal(h->ready);
        }
        void *retained_error = error ? nw_retain(error) : NULL;
        callback((int)state, retained_error, user_info);
    });
}

void nw_shim_connection_drain_queue(void *handle) {
    nw_conn_handle *h = (nw_conn_handle *)handle;
    if (!h || !h->queue) {
        return;
    }
    dispatch_sync(h->queue, ^{});
}

void nw_shim_listener_set_state_changed_handler(void *handle, ListenerStateCallback callback, void *user_info) {
    nw_listener_handle *h = (nw_listener_handle *)handle;
    if (!h) {
        return;
    }
    if (!callback) {
        nw_listener_set_state_changed_handler(h->listener, NULL);
        return;
    }
    nw_listener_set_state_changed_handler(h->listener, ^(nw_listener_state_t state, nw_error_t error) {
        if (state == nw_listener_state_ready) {
            atomic_store(&h->bound_port, nw_listener_get_port(h->listener));
            atomic_store(&h->state_code, 1);
            dispatch_semaphore_signal(h->ready);
        } else if (state == nw_listener_state_cancelled) {
            atomic_store(&h->state_code, 2);
            dispatch_semaphore_signal(h->ready);
        } else if (state == nw_listener_state_failed) {
            atomic_store(&h->state_code, 3);
            dispatch_semaphore_signal(h->ready);
        }
        void *retained_error = error ? nw_retain(error) : NULL;
        callback((int)state, retained_error, user_info);
    });
}

void nw_shim_listener_set_new_connection_handler(void *handle, ListenerNewConnectionCallback callback, void *user_info) {
    nw_listener_handle *h = (nw_listener_handle *)handle;
    if (!h) {
        return;
    }
    if (!callback) {
        nw_listener_set_new_connection_handler(h->listener, NULL);
        return;
    }
    nw_listener_set_new_connection_handler(h->listener, ^(nw_connection_t conn) {
        if (!conn) {
            return;
        }
        int status = NW_OK;
        nw_connection_t retained = nw_retain(conn);
        nw_conn_handle *wrapped = nw_shim_wrap_started_connection(retained, "networkframework-rs.async-accepted", &status);
        if (wrapped) {
            callback(wrapped, user_info);
        }
    });
}

void nw_shim_listener_drain_queue(void *handle) {
    nw_listener_handle *h = (nw_listener_handle *)handle;
    if (!h || !h->queue) {
        return;
    }
    dispatch_sync(h->queue, ^{});
}

void nw_shim_browser_set_browse_results_changed_handler(void *handle, BrowseResultChangedCallback callback, void *user_info) {
    nw_browser_handle *h = (nw_browser_handle *)handle;
    if (!h) {
        return;
    }
    if (!callback) {
        nw_browser_set_browse_results_changed_handler(h->browser, NULL);
        return;
    }
    nw_browser_set_browse_results_changed_handler(h->browser, ^(nw_browse_result_t old_result, nw_browse_result_t new_result, bool batch_complete) {
        uint64_t changes = nw_browse_result_get_changes(old_result, new_result);
        void *old_handle = old_result ? nw_retain(old_result) : NULL;
        void *new_handle = new_result ? nw_retain(new_result) : NULL;
        callback(old_handle, new_handle, changes, batch_complete ? 1 : 0, user_info);
    });
}

void nw_shim_browser_drain_queue(void *handle) {
    nw_browser_handle *h = (nw_browser_handle *)handle;
    if (!h || !h->queue) {
        return;
    }
    dispatch_sync(h->queue, ^{});
}

void nw_shim_path_monitor_set_update_handler(void *handle, ConnectionPathCallback callback, void *user_info) {
    nw_path_handle *h = (nw_path_handle *)handle;
    if (!h) {
        return;
    }
    if (!callback) {
        nw_path_monitor_set_update_handler(h->monitor, NULL);
        return;
    }
    nw_path_monitor_set_update_handler(h->monitor, ^(nw_path_t path) {
        if (h->latest_path) {
            nw_release(h->latest_path);
            h->latest_path = NULL;
        }
        if (path) {
            h->latest_path = nw_retain(path);
        }
        void *retained_path = path ? nw_retain(path) : NULL;
        callback(retained_path, user_info);
    });
}

void nw_shim_path_monitor_drain_queue(void *handle) {
    nw_path_handle *h = (nw_path_handle *)handle;
    if (!h || !h->queue) {
        return;
    }
    dispatch_sync(h->queue, ^{});
}

