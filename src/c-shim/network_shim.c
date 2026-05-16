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

void *nw_shim_tcp_connect(const char *host, uint16_t port, int *out_status) {
    if (!host) { if (out_status) *out_status = NW_INVALID_ARG; return NULL; }

    char port_str[8];
    snprintf(port_str, sizeof(port_str), "%u", (unsigned)port);
    nw_endpoint_t endpoint = nw_endpoint_create_host(host, port_str);
    if (!endpoint) { if (out_status) *out_status = NW_INVALID_ARG; return NULL; }

    nw_parameters_t params = nw_parameters_create_secure_tcp(
        NW_PARAMETERS_DISABLE_PROTOCOL,
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

void *nw_shim_listener_create(uint16_t port, int *out_status) {
    char port_str[8];
    snprintf(port_str, sizeof(port_str), "%u", (unsigned)port);

    nw_parameters_t params = nw_parameters_create_secure_tcp(
        NW_PARAMETERS_DISABLE_PROTOCOL,
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
