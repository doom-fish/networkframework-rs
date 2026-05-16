// Tiny C shim that hides Network.framework's heavy use of Objective-C
// blocks and dispatch_data behind a synchronous C API. Bridged 1:1 by
// src/ffi/mod.rs on the Rust side.

#include <Network/Network.h>
#include <dispatch/dispatch.h>
#include <arpa/inet.h>
#include <netinet/in.h>
#include <stdatomic.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
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
        nw_listener_cancel(h->listener);
        dispatch_time_t deadline = dispatch_time(DISPATCH_TIME_NOW, 5LL * NSEC_PER_SEC);
        while (atomic_load(&h->state_code) != 2 && atomic_load(&h->state_code) != 3) {
            if (dispatch_semaphore_wait(h->ready, deadline) != 0) {
                break;
            }
        }
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
