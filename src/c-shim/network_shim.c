// Tiny C shim that hides Network.framework's heavy use of Objective-C
// blocks and dispatch_data behind a synchronous C API. Bridged 1:1 by
// src/ffi/mod.rs on the Rust side.

#include <Network/Network.h>
#include <dispatch/dispatch.h>
#include <stdatomic.h>
#include <stdlib.h>
#include <string.h>

// ---------------------------------------------------------------------
// Status codes (mirrored in Rust)
// ---------------------------------------------------------------------

#define NW_OK              0
#define NW_INVALID_ARG    -1
#define NW_CONNECT_FAILED -2
#define NW_SEND_FAILED    -3
#define NW_RECV_FAILED    -4
#define NW_LISTEN_FAILED  -5
#define NW_CANCELLED      -6
#define NW_TIMEOUT        -7

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
        nw_connection_cancel(conn);
        destroy_handle(h);
        if (out_status) *out_status = NW_TIMEOUT;
        return NULL;
    }

    int code = atomic_load(&h->state_code);
    if (code != 1) {
        nw_connection_cancel(conn);
        destroy_handle(h);
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
    nw_connection_cancel(h->conn);
    destroy_handle(h);
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
        nw_listener_cancel(listener);
        nw_release(h->listener);
        dispatch_release(h->queue);
        dispatch_release(h->ready);
        dispatch_release(h->accept_sem);
        free(h);
        if (out_status) *out_status = NW_LISTEN_FAILED;
        return NULL;
    }

    if (atomic_load(&h->state_code) != 1) {
        nw_listener_cancel(listener);
        nw_release(h->listener);
        dispatch_release(h->queue);
        dispatch_release(h->ready);
        dispatch_release(h->accept_sem);
        free(h);
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
        nw_connection_cancel(conn);
        destroy_handle(ch);
        if (out_status) *out_status = NW_CONNECT_FAILED;
        return NULL;
    }

    if (out_status) *out_status = NW_OK;
    return ch;
}

void nw_shim_listener_close(void *handle) {
    nw_listener_handle *h = (nw_listener_handle *)handle;
    if (!h) return;
    nw_listener_cancel(h->listener);
    if (h->pending) {
        nw_connection_cancel(h->pending);
        nw_release(h->pending);
    }
    nw_release(h->listener);
    dispatch_release(h->queue);
    dispatch_release(h->ready);
    dispatch_release(h->accept_sem);
    free(h);
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
        nw_connection_cancel(conn);
        destroy_handle(h);
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
        nw_connection_cancel(conn);
        destroy_handle(h);
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
        nw_connection_cancel(conn);
        destroy_handle(h);
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
    nw_release(h->listener);
    dispatch_release(h->queue);
    dispatch_release(h->ready);
    free(h);
}
