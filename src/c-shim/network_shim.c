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
#include <CommonCrypto/CommonDigest.h>
#include <Security/Security.h>
#include <pthread.h>
#include <time.h>

#include "network_shim.h"

// ---------------------------------------------------------------------
// Connection (outbound TCP)
// ---------------------------------------------------------------------

#define NW_SHIM_CONNECT_TIMEOUT_NS (30LL * (int64_t)NSEC_PER_SEC)
#define NW_SHIM_START_TIMEOUT_NS (10LL * (int64_t)NSEC_PER_SEC)
#define NW_SHIM_ACCEPT_TIMEOUT_NS (10LL * (int64_t)NSEC_PER_SEC)
#define NW_SHIM_ACCEPT_BACKLOG 128
#define NW_SHIM_INLINE_SNAPSHOT 4

typedef void (*nw_shim_fn)(void);

typedef struct nw_shim_callback {
    nw_shim_fn fn;
    void *context;
    NwShimContextCallback retain;
    NwShimContextCallback release;
} nw_shim_callback;

typedef struct nw_shim_subscription {
    struct nw_shim_subscription *next;
    uint64_t token;
    int kind;
    nw_shim_callback callback;
} nw_shim_subscription;

typedef struct nw_shim_subscriptions {
    pthread_mutex_t lock;
    nw_shim_subscription *head;
    uint64_t next_token;
} nw_shim_subscriptions;

typedef struct nw_shim_snapshot {
    size_t count;
    nw_shim_callback *items;
    nw_shim_callback inline_items[NW_SHIM_INLINE_SNAPSHOT];
} nw_shim_snapshot;

static nw_shim_callback nw_shim_make_callback(
    nw_shim_fn fn,
    void *context,
    NwShimContextCallback retain,
    NwShimContextCallback release
) {
    nw_shim_callback callback = { fn, context, retain, release };
    return callback;
}

static void nw_shim_callback_release(const nw_shim_callback *callback) {
    if (callback->release && callback->context) {
        callback->release(callback->context);
    }
}

static void nw_shim_subscriptions_init(nw_shim_subscriptions *subs) {
    pthread_mutex_init(&subs->lock, NULL);
    subs->head = NULL;
    subs->next_token = 1;
}

static uint64_t nw_shim_subscriptions_add(nw_shim_subscriptions *subs, int kind, nw_shim_callback callback) {
    if (!callback.fn) {
        nw_shim_callback_release(&callback);
        return 0;
    }
    nw_shim_subscription *entry = (nw_shim_subscription *)calloc(1, sizeof(nw_shim_subscription));
    if (!entry) {
        nw_shim_callback_release(&callback);
        return 0;
    }
    entry->kind = kind;
    entry->callback = callback;
    pthread_mutex_lock(&subs->lock);
    entry->token = subs->next_token++;
    nw_shim_subscription **link = &subs->head;
    while (*link) {
        link = &(*link)->next;
    }
    *link = entry;
    uint64_t token = entry->token;
    pthread_mutex_unlock(&subs->lock);
    return token;
}

static void nw_shim_subscriptions_remove(nw_shim_subscriptions *subs, uint64_t token) {
    if (token == 0) {
        return;
    }
    nw_shim_subscription *found = NULL;
    pthread_mutex_lock(&subs->lock);
    for (nw_shim_subscription **link = &subs->head; *link; link = &(*link)->next) {
        if ((*link)->token == token) {
            found = *link;
            *link = found->next;
            break;
        }
    }
    pthread_mutex_unlock(&subs->lock);
    if (found) {
        nw_shim_callback_release(&found->callback);
        free(found);
    }
}

static void nw_shim_subscriptions_snapshot(nw_shim_subscriptions *subs, int kind, nw_shim_snapshot *snapshot) {
    snapshot->count = 0;
    snapshot->items = snapshot->inline_items;
    pthread_mutex_lock(&subs->lock);
    size_t wanted = 0;
    for (nw_shim_subscription *entry = subs->head; entry; entry = entry->next) {
        if (entry->kind == kind) {
            wanted += 1;
        }
    }
    size_t capacity = NW_SHIM_INLINE_SNAPSHOT;
    if (wanted > capacity) {
        nw_shim_callback *items = (nw_shim_callback *)calloc(wanted, sizeof(nw_shim_callback));
        if (items) {
            snapshot->items = items;
            capacity = wanted;
        }
    }
    for (nw_shim_subscription *entry = subs->head; entry && snapshot->count < capacity; entry = entry->next) {
        if (entry->kind != kind) {
            continue;
        }
        nw_shim_callback item = entry->callback;
        if (item.retain && item.context) {
            item.retain(item.context);
        } else {
            item.release = NULL;
        }
        snapshot->items[snapshot->count++] = item;
    }
    pthread_mutex_unlock(&subs->lock);
}

static void nw_shim_snapshot_release(nw_shim_snapshot *snapshot) {
    for (size_t index = 0; index < snapshot->count; index++) {
        nw_shim_callback_release(&snapshot->items[index]);
    }
    if (snapshot->items != snapshot->inline_items) {
        free(snapshot->items);
    }
    snapshot->items = snapshot->inline_items;
    snapshot->count = 0;
}

static void nw_shim_subscriptions_destroy(nw_shim_subscriptions *subs) {
    pthread_mutex_lock(&subs->lock);
    nw_shim_subscription *entry = subs->head;
    subs->head = NULL;
    pthread_mutex_unlock(&subs->lock);
    while (entry) {
        nw_shim_subscription *next = entry->next;
        nw_shim_callback_release(&entry->callback);
        free(entry);
        entry = next;
    }
    pthread_mutex_destroy(&subs->lock);
}

static uint64_t nw_shim_now_ns(void) {
    return clock_gettime_nsec_np(CLOCK_UPTIME_RAW);
}

static uint64_t nw_shim_deadline_after(int64_t timeout_ns) {
    return nw_shim_now_ns() + (uint64_t)timeout_ns;
}

static bool nw_shim_cond_wait_until(pthread_cond_t *cond, pthread_mutex_t *lock, uint64_t deadline) {
    uint64_t now = nw_shim_now_ns();
    if (now >= deadline) {
        return false;
    }
    uint64_t remaining = deadline - now;
    struct timespec relative;
    relative.tv_sec = (time_t)(remaining / NSEC_PER_SEC);
    relative.tv_nsec = (long)(remaining % NSEC_PER_SEC);
    pthread_cond_timedwait_relative_np(cond, lock, &relative);
    return true;
}

typedef struct nw_shim_waiter {
    _Atomic long refs;
    pthread_mutex_t lock;
    pthread_cond_t cond;
    bool done;
    void *value;
} nw_shim_waiter;

static nw_shim_waiter *nw_shim_waiter_create(long refs) {
    nw_shim_waiter *waiter = (nw_shim_waiter *)calloc(1, sizeof(nw_shim_waiter));
    if (!waiter) {
        return NULL;
    }
    atomic_init(&waiter->refs, refs);
    pthread_mutex_init(&waiter->lock, NULL);
    pthread_cond_init(&waiter->cond, NULL);
    return waiter;
}

static void nw_shim_waiter_release(nw_shim_waiter *waiter) {
    if (atomic_fetch_sub_explicit(&waiter->refs, 1, memory_order_acq_rel) != 1) {
        return;
    }
    if (waiter->value) {
        nw_release(waiter->value);
    }
    pthread_cond_destroy(&waiter->cond);
    pthread_mutex_destroy(&waiter->lock);
    free(waiter);
}

static void nw_shim_waiter_complete(nw_shim_waiter *waiter, void *value) {
    pthread_mutex_lock(&waiter->lock);
    bool keep = !waiter->done;
    if (keep) {
        waiter->done = true;
        waiter->value = value;
        pthread_cond_broadcast(&waiter->cond);
    }
    pthread_mutex_unlock(&waiter->lock);
    if (!keep && value) {
        nw_release(value);
    }
}

static void *nw_shim_waiter_wait_take(nw_shim_waiter *waiter, int64_t timeout_ns, bool *out_done) {
    uint64_t deadline = nw_shim_deadline_after(timeout_ns);
    pthread_mutex_lock(&waiter->lock);
    while (!waiter->done) {
        if (!nw_shim_cond_wait_until(&waiter->cond, &waiter->lock, deadline)) {
            break;
        }
    }
    bool done = waiter->done;
    void *value = waiter->value;
    waiter->value = NULL;
    waiter->done = true;
    pthread_mutex_unlock(&waiter->lock);
    if (out_done) {
        *out_done = done;
    }
    return value;
}

static ssize_t nw_shim_copy_received(dispatch_data_t content, uint8_t *out_buf, size_t max_len, size_t *out_size) {
    size_t size = content ? dispatch_data_get_size(content) : 0;
    if (out_size) {
        *out_size = size;
    }
    if (size > max_len) {
        return NW_MESSAGE_TOO_LARGE;
    }
    if (size == 0) {
        return 0;
    }
    __block size_t copied = 0;
    dispatch_data_apply(content, ^bool(dispatch_data_t region, size_t offset, const void *buffer, size_t length) {
        (void)region;
        (void)offset;
        memcpy(out_buf + copied, buffer, length);
        copied += length;
        return true;
    });
    return (ssize_t)copied;
}

static uint32_t nw_shim_clamp_receive_length(size_t max_len) {
    return max_len > UINT32_MAX ? UINT32_MAX : (uint32_t)max_len;
}

typedef struct nw_shim_acceptor nw_shim_acceptor;

enum {
    NW_SHIM_SLOT_NONE = 0,
    NW_SHIM_SLOT_PENDING,
    NW_SHIM_SLOT_READY,
};

typedef struct nw_conn_handle {
    _Atomic long refs;
    nw_connection_t conn;
    dispatch_queue_t queue;
    pthread_mutex_t lock;
    pthread_cond_t cond;
    int state;
    bool waiting_error;
    bool cancelled;
    bool cancel_requested;
    bool handlers_cleared;
    nw_shim_subscriptions subs;
    nw_shim_acceptor *acceptor;
    int slot;
    struct nw_conn_handle *prev;
    struct nw_conn_handle *next;
} nw_conn_handle;

struct nw_shim_acceptor {
    pthread_mutex_t lock;
    pthread_cond_t cond;
    bool closed;
    bool terminal;
    bool keep_ready;
    nw_conn_handle *pending_head;
    nw_conn_handle *pending_tail;
    size_t pending_count;
    nw_conn_handle *ready_head;
    nw_conn_handle *ready_tail;
    size_t ready_count;
    nw_shim_subscriptions *subs;
    void *owner;
    void (*retain_owner)(void *owner);
    void (*release_owner)(void *owner);
};

static void nw_shim_conn_retain(nw_conn_handle *h) {
    atomic_fetch_add_explicit(&h->refs, 1, memory_order_relaxed);
}

void nw_shim_conn_release(void *handle) {
    nw_conn_handle *h = (nw_conn_handle *)handle;
    if (atomic_fetch_sub_explicit(&h->refs, 1, memory_order_acq_rel) != 1) {
        return;
    }
    nw_shim_subscriptions_destroy(&h->subs);
    nw_release(h->conn);
    dispatch_release(h->queue);
    pthread_cond_destroy(&h->cond);
    pthread_mutex_destroy(&h->lock);
    free(h);
}

static void nw_shim_conn_clear_handlers(nw_conn_handle *h) {
    pthread_mutex_lock(&h->lock);
    bool clear = !h->handlers_cleared;
    h->handlers_cleared = true;
    pthread_mutex_unlock(&h->lock);
    if (clear) {
        nw_connection_set_state_changed_handler(h->conn, NULL);
        nw_connection_set_viability_changed_handler(h->conn, NULL);
        nw_connection_set_better_path_available_handler(h->conn, NULL);
        nw_connection_set_path_changed_handler(h->conn, NULL);
    }
}

static void nw_shim_conn_clear_handlers_async(void *context) {
    nw_conn_handle *h = (nw_conn_handle *)context;
    nw_shim_conn_clear_handlers(h);
    nw_shim_conn_release(h);
}

static void nw_shim_conn_cancel(nw_conn_handle *h) {
    pthread_mutex_lock(&h->lock);
    bool should_cancel = !h->cancel_requested && !h->cancelled;
    h->cancel_requested = true;
    pthread_mutex_unlock(&h->lock);
    if (should_cancel) {
        nw_connection_cancel(h->conn);
    }
}

static void nw_shim_conn_close(nw_conn_handle *h) {
    nw_shim_conn_cancel(h);
    nw_shim_conn_release(h);
}

static void nw_shim_acceptor_on_state(nw_conn_handle *h, nw_connection_state_t state, bool has_error);

void nw_shim_conn_on_state(void *handle, int raw_state, void *raw_error) {
    nw_conn_handle *h = (nw_conn_handle *)handle;
    nw_connection_state_t state = (nw_connection_state_t)raw_state;
    nw_error_t error = (nw_error_t)raw_error;
    pthread_mutex_lock(&h->lock);
    if (h->cancelled) {
        pthread_mutex_unlock(&h->lock);
        return;
    }
    h->state = (int)state;
    h->waiting_error = state == nw_connection_state_waiting && error != NULL;
    bool final_event = state == nw_connection_state_cancelled;
    if (final_event) {
        h->cancelled = true;
    }
    pthread_cond_broadcast(&h->cond);
    pthread_mutex_unlock(&h->lock);

    nw_shim_acceptor_on_state(h, state, error != NULL);

    nw_shim_snapshot snapshot;
    nw_shim_subscriptions_snapshot(&h->subs, NW_SHIM_EVENT_STATE, &snapshot);
    for (size_t index = 0; index < snapshot.count; index++) {
        ((ConnectionStateCallback)snapshot.items[index].fn)(
            (int)state,
            error ? nw_retain(error) : NULL,
            snapshot.items[index].context);
    }
    nw_shim_snapshot_release(&snapshot);

    if (final_event) {
        nw_shim_conn_retain(h);
        dispatch_async_f(h->queue, h, nw_shim_conn_clear_handlers_async);
    }
}

void nw_shim_conn_on_boolean(void *handle, int kind, int value) {
    nw_conn_handle *h = (nw_conn_handle *)handle;
    nw_shim_snapshot snapshot;
    nw_shim_subscriptions_snapshot(&h->subs, kind, &snapshot);
    for (size_t index = 0; index < snapshot.count; index++) {
        ((ConnectionBooleanCallback)snapshot.items[index].fn)(value ? 1 : 0, snapshot.items[index].context);
    }
    nw_shim_snapshot_release(&snapshot);
}

void nw_shim_conn_on_path(void *handle, void *raw_path) {
    nw_conn_handle *h = (nw_conn_handle *)handle;
    nw_path_t path = (nw_path_t)raw_path;
    nw_shim_snapshot snapshot;
    nw_shim_subscriptions_snapshot(&h->subs, NW_SHIM_EVENT_PATH, &snapshot);
    for (size_t index = 0; index < snapshot.count; index++) {
        ((ConnectionPathCallback)snapshot.items[index].fn)(
            path ? nw_retain(path) : NULL,
            snapshot.items[index].context);
    }
    nw_shim_snapshot_release(&snapshot);
}

static nw_conn_handle *nw_shim_conn_create(nw_connection_t conn, const char *label) {
    if (!conn) {
        return NULL;
    }
    nw_conn_handle *h = (nw_conn_handle *)calloc(1, sizeof(nw_conn_handle));
    if (!h) {
        nw_connection_cancel(conn);
        nw_release(conn);
        return NULL;
    }
    atomic_init(&h->refs, 2);
    h->conn = conn;
    h->queue = dispatch_queue_create(label ? label : "networkframework-rs.connection", DISPATCH_QUEUE_SERIAL);
    pthread_mutex_init(&h->lock, NULL);
    pthread_cond_init(&h->cond, NULL);
    nw_shim_subscriptions_init(&h->subs);

    nw_connection_set_queue(conn, h->queue);
    if (!nw_shim_conn_install_handlers(conn, h)) {
        nw_connection_cancel(conn);
        nw_shim_conn_release(h);
        nw_shim_conn_release(h);
        return NULL;
    }
    return h;
}

static int nw_shim_conn_wait_ready(nw_conn_handle *h, int64_t timeout_ns) {
    uint64_t deadline = nw_shim_deadline_after(timeout_ns);
    pthread_mutex_lock(&h->lock);
    while (h->state != nw_connection_state_ready
           && h->state != nw_connection_state_failed
           && !h->waiting_error
           && !h->cancelled) {
        if (!nw_shim_cond_wait_until(&h->cond, &h->lock, deadline)) {
            break;
        }
    }
    int state = h->state;
    bool failed = h->cancelled || h->waiting_error || state == nw_connection_state_failed;
    pthread_mutex_unlock(&h->lock);
    if (failed) {
        return NW_CONNECT_FAILED;
    }
    return state == nw_connection_state_ready ? NW_OK : NW_TIMEOUT;
}

static void *nw_shim_conn_start_and_wait(nw_connection_t conn, const char *label, int64_t timeout_ns, int *out_status) {
    nw_conn_handle *h = nw_shim_conn_create(conn, label);
    if (!h) {
        if (out_status) *out_status = NW_CONNECT_FAILED;
        return NULL;
    }
    nw_connection_start(h->conn);
    int status = nw_shim_conn_wait_ready(h, timeout_ns);
    if (status != NW_OK) {
        nw_shim_conn_close(h);
        if (out_status) *out_status = status;
        return NULL;
    }
    if (out_status) *out_status = NW_OK;
    return h;
}

static void nw_shim_acceptor_init(
    nw_shim_acceptor *acceptor,
    nw_shim_subscriptions *subs,
    void *owner,
    void (*retain_owner)(void *owner),
    void (*release_owner)(void *owner),
    bool keep_ready
) {
    pthread_mutex_init(&acceptor->lock, NULL);
    pthread_cond_init(&acceptor->cond, NULL);
    acceptor->subs = subs;
    acceptor->owner = owner;
    acceptor->retain_owner = retain_owner;
    acceptor->release_owner = release_owner;
    acceptor->keep_ready = keep_ready;
}

static void nw_shim_acceptor_destroy(nw_shim_acceptor *acceptor) {
    pthread_cond_destroy(&acceptor->cond);
    pthread_mutex_destroy(&acceptor->lock);
}

static void nw_shim_conn_list_append(nw_conn_handle **head, nw_conn_handle **tail, nw_conn_handle *h) {
    h->prev = *tail;
    h->next = NULL;
    if (*tail) {
        (*tail)->next = h;
    } else {
        *head = h;
    }
    *tail = h;
}

static void nw_shim_conn_list_remove(nw_conn_handle **head, nw_conn_handle **tail, nw_conn_handle *h) {
    if (h->prev) {
        h->prev->next = h->next;
    } else {
        *head = h->next;
    }
    if (h->next) {
        h->next->prev = h->prev;
    } else {
        *tail = h->prev;
    }
    h->prev = NULL;
    h->next = NULL;
}

static nw_shim_acceptor *nw_shim_conn_copy_acceptor(nw_conn_handle *h) {
    pthread_mutex_lock(&h->lock);
    nw_shim_acceptor *acceptor = h->acceptor;
    if (acceptor) {
        acceptor->retain_owner(acceptor->owner);
    }
    pthread_mutex_unlock(&h->lock);
    return acceptor;
}

static void nw_shim_conn_detach(nw_conn_handle *h) {
    pthread_mutex_lock(&h->lock);
    nw_shim_acceptor *acceptor = h->acceptor;
    h->acceptor = NULL;
    pthread_mutex_unlock(&h->lock);
    if (acceptor) {
        acceptor->release_owner(acceptor->owner);
    }
}

static bool nw_shim_acceptor_unlink(nw_shim_acceptor *acceptor, nw_conn_handle *h, bool pending_only) {
    if (h->slot == NW_SHIM_SLOT_PENDING) {
        nw_shim_conn_list_remove(&acceptor->pending_head, &acceptor->pending_tail, h);
        acceptor->pending_count -= 1;
    } else if (h->slot == NW_SHIM_SLOT_READY && !pending_only) {
        nw_shim_conn_list_remove(&acceptor->ready_head, &acceptor->ready_tail, h);
        acceptor->ready_count -= 1;
    } else {
        return false;
    }
    h->slot = NW_SHIM_SLOT_NONE;
    return true;
}

static void nw_shim_acceptor_evict(nw_conn_handle *h, bool pending_only) {
    nw_shim_acceptor *acceptor = nw_shim_conn_copy_acceptor(h);
    if (!acceptor) {
        return;
    }
    pthread_mutex_lock(&acceptor->lock);
    bool evicted = nw_shim_acceptor_unlink(acceptor, h, pending_only);
    pthread_mutex_unlock(&acceptor->lock);
    if (evicted) {
        nw_shim_conn_detach(h);
        nw_shim_conn_close(h);
    }
    acceptor->release_owner(acceptor->owner);
}

static void nw_shim_acceptor_timeout(void *context) {
    nw_conn_handle *h = (nw_conn_handle *)context;
    nw_shim_acceptor_evict(h, true);
    nw_shim_conn_release(h);
}

static void nw_shim_acceptor_dispatch(nw_shim_acceptor *acceptor) {
    for (;;) {
        nw_shim_snapshot snapshot;
        nw_shim_subscriptions_snapshot(acceptor->subs, NW_SHIM_EVENT_NEW_CONNECTION, &snapshot);
        if (snapshot.count == 0 && acceptor->keep_ready) {
            nw_shim_snapshot_release(&snapshot);
            return;
        }
        pthread_mutex_lock(&acceptor->lock);
        nw_conn_handle *h = acceptor->ready_head;
        if (h) {
            nw_shim_acceptor_unlink(acceptor, h, false);
        }
        pthread_mutex_unlock(&acceptor->lock);
        if (!h) {
            nw_shim_snapshot_release(&snapshot);
            return;
        }
        nw_shim_conn_detach(h);
        if (snapshot.count > 0) {
            ((ListenerNewConnectionCallback)snapshot.items[0].fn)(h, snapshot.items[0].context);
        } else {
            nw_shim_conn_close(h);
        }
        nw_shim_snapshot_release(&snapshot);
    }
}

static void nw_shim_acceptor_on_state(nw_conn_handle *h, nw_connection_state_t state, bool has_error) {
    if (state == nw_connection_state_ready) {
        nw_shim_acceptor *acceptor = nw_shim_conn_copy_acceptor(h);
        if (!acceptor) {
            return;
        }
        bool promoted = false;
        pthread_mutex_lock(&acceptor->lock);
        if (h->slot == NW_SHIM_SLOT_PENDING && !acceptor->closed) {
            nw_shim_acceptor_unlink(acceptor, h, true);
            nw_shim_conn_list_append(&acceptor->ready_head, &acceptor->ready_tail, h);
            acceptor->ready_count += 1;
            h->slot = NW_SHIM_SLOT_READY;
            promoted = true;
            pthread_cond_broadcast(&acceptor->cond);
        }
        pthread_mutex_unlock(&acceptor->lock);
        if (promoted) {
            nw_shim_acceptor_dispatch(acceptor);
        }
        acceptor->release_owner(acceptor->owner);
    } else if (state == nw_connection_state_failed
               || state == nw_connection_state_cancelled
               || (state == nw_connection_state_waiting && has_error)) {
        nw_shim_acceptor_evict(h, false);
    }
}

static void nw_shim_acceptor_offer(nw_shim_acceptor *acceptor, nw_connection_t connection, const char *label) {
    if (!connection) {
        return;
    }
    pthread_mutex_lock(&acceptor->lock);
    bool admit = !acceptor->closed
        && acceptor->pending_count + acceptor->ready_count < NW_SHIM_ACCEPT_BACKLOG;
    pthread_mutex_unlock(&acceptor->lock);
    if (!admit) {
        nw_connection_cancel(connection);
        return;
    }
    nw_conn_handle *h = nw_shim_conn_create(nw_retain(connection), label);
    if (!h) {
        return;
    }
    acceptor->retain_owner(acceptor->owner);
    pthread_mutex_lock(&h->lock);
    h->acceptor = acceptor;
    pthread_mutex_unlock(&h->lock);

    pthread_mutex_lock(&acceptor->lock);
    bool closed = acceptor->closed;
    if (!closed) {
        nw_shim_conn_list_append(&acceptor->pending_head, &acceptor->pending_tail, h);
        acceptor->pending_count += 1;
        h->slot = NW_SHIM_SLOT_PENDING;
    }
    pthread_mutex_unlock(&acceptor->lock);
    if (closed) {
        nw_shim_conn_detach(h);
        nw_shim_conn_close(h);
        return;
    }
    nw_shim_conn_retain(h);
    dispatch_after_f(dispatch_time(DISPATCH_TIME_NOW, NW_SHIM_ACCEPT_TIMEOUT_NS), h->queue, h, nw_shim_acceptor_timeout);
    nw_connection_start(h->conn);
}

static void *nw_shim_acceptor_accept(nw_shim_acceptor *acceptor, int *out_status) {
    pthread_mutex_lock(&acceptor->lock);
    while (!acceptor->ready_head && !acceptor->closed && !acceptor->terminal) {
        pthread_cond_wait(&acceptor->cond, &acceptor->lock);
    }
    nw_conn_handle *h = acceptor->ready_head;
    if (h) {
        nw_shim_acceptor_unlink(acceptor, h, false);
    }
    pthread_mutex_unlock(&acceptor->lock);
    if (!h) {
        if (out_status) *out_status = NW_CANCELLED;
        return NULL;
    }
    nw_shim_conn_detach(h);
    if (out_status) *out_status = NW_OK;
    return h;
}

static void nw_shim_acceptor_mark_terminal(nw_shim_acceptor *acceptor) {
    pthread_mutex_lock(&acceptor->lock);
    acceptor->terminal = true;
    pthread_cond_broadcast(&acceptor->cond);
    pthread_mutex_unlock(&acceptor->lock);
}

static void nw_shim_acceptor_close_list(nw_conn_handle *h) {
    while (h) {
        nw_conn_handle *next = h->next;
        h->prev = NULL;
        h->next = NULL;
        nw_shim_conn_detach(h);
        nw_shim_conn_close(h);
        h = next;
    }
}

static void nw_shim_acceptor_close(nw_shim_acceptor *acceptor) {
    pthread_mutex_lock(&acceptor->lock);
    acceptor->closed = true;
    nw_conn_handle *pending = acceptor->pending_head;
    nw_conn_handle *ready = acceptor->ready_head;
    acceptor->pending_head = NULL;
    acceptor->pending_tail = NULL;
    acceptor->ready_head = NULL;
    acceptor->ready_tail = NULL;
    acceptor->pending_count = 0;
    acceptor->ready_count = 0;
    for (nw_conn_handle *h = pending; h; h = h->next) {
        h->slot = NW_SHIM_SLOT_NONE;
    }
    for (nw_conn_handle *h = ready; h; h = h->next) {
        h->slot = NW_SHIM_SLOT_NONE;
    }
    pthread_cond_broadcast(&acceptor->cond);
    pthread_mutex_unlock(&acceptor->lock);
    nw_shim_acceptor_close_list(pending);
    nw_shim_acceptor_close_list(ready);
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
    if (!params) {
        nw_release(endpoint);
        if (out_status) *out_status = NW_CONNECT_FAILED;
        return NULL;
    }

    nw_connection_t conn = nw_connection_create(endpoint, params);
    nw_release(endpoint);
    nw_release(params);
    if (!conn) { if (out_status) *out_status = NW_CONNECT_FAILED; return NULL; }

    return nw_shim_conn_start_and_wait(conn, "networkframework-rs.conn", NW_SHIM_CONNECT_TIMEOUT_NS, out_status);
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
ssize_t nw_shim_tcp_receive(void *handle, uint8_t *out_buf, size_t max_len, size_t *out_size) {
    nw_conn_handle *h = (nw_conn_handle *)handle;
    if (out_size) *out_size = 0;
    if (!h || !out_buf || max_len == 0) return NW_INVALID_ARG;

    __block ssize_t result = 0;
    __block size_t size = 0;
    dispatch_semaphore_t done = dispatch_semaphore_create(0);

    nw_connection_receive(h->conn, 1, nw_shim_clamp_receive_length(max_len),
        ^(dispatch_data_t content, nw_content_context_t ctx, bool is_complete, nw_error_t error) {
            (void)ctx; (void)is_complete;
            if (error) {
                result = NW_RECV_FAILED;
            } else {
                result = nw_shim_copy_received(content, out_buf, max_len, &size);
            }
            dispatch_semaphore_signal(done);
        });

    dispatch_semaphore_wait(done, DISPATCH_TIME_FOREVER);
    dispatch_release(done);
    if (out_size) *out_size = size;
    return result;
}

void nw_shim_tcp_close(void *handle) {
    nw_conn_handle *h = (nw_conn_handle *)handle;
    if (!h) return;
    nw_shim_conn_close(h);
}

// ---------------------------------------------------------------------
// Listener (inbound TCP)
// ---------------------------------------------------------------------

typedef struct nw_listener_handle {
    _Atomic long refs;
    nw_listener_t listener;
    dispatch_queue_t queue;
    pthread_mutex_t lock;
    pthread_cond_t cond;
    int state;
    bool cancelled;
    bool cancel_requested;
    bool advertised_installed;
    bool group_mode;
    _Atomic uint16_t bound_port;
    nw_shim_subscriptions subs;
    nw_shim_acceptor acceptor;
} nw_listener_handle;

static void nw_shim_listener_retain_owner(void *owner) {
    atomic_fetch_add_explicit(&((nw_listener_handle *)owner)->refs, 1, memory_order_relaxed);
}

static void nw_shim_listener_release_owner(void *owner) {
    nw_listener_handle *h = (nw_listener_handle *)owner;
    if (atomic_fetch_sub_explicit(&h->refs, 1, memory_order_acq_rel) != 1) {
        return;
    }
    nw_shim_acceptor_destroy(&h->acceptor);
    nw_shim_subscriptions_destroy(&h->subs);
    nw_release(h->listener);
    dispatch_release(h->queue);
    pthread_cond_destroy(&h->cond);
    pthread_mutex_destroy(&h->lock);
    free(h);
}

static void nw_shim_listener_release_async(void *context) {
    nw_shim_listener_release_owner(context);
}

static void nw_shim_listener_on_state(nw_listener_handle *h, nw_listener_state_t state, nw_error_t error) {
    pthread_mutex_lock(&h->lock);
    if (h->cancelled) {
        pthread_mutex_unlock(&h->lock);
        return;
    }
    h->state = (int)state;
    if (state == nw_listener_state_ready) {
        atomic_store(&h->bound_port, nw_listener_get_port(h->listener));
    }
    bool final_event = state == nw_listener_state_cancelled;
    if (final_event) {
        h->cancelled = true;
    }
    pthread_cond_broadcast(&h->cond);
    pthread_mutex_unlock(&h->lock);

    if (state == nw_listener_state_failed || final_event) {
        nw_shim_acceptor_mark_terminal(&h->acceptor);
    }

    nw_shim_snapshot snapshot;
    nw_shim_subscriptions_snapshot(&h->subs, NW_SHIM_EVENT_STATE, &snapshot);
    for (size_t index = 0; index < snapshot.count; index++) {
        ((ListenerStateCallback)snapshot.items[index].fn)(
            (int)state,
            error ? nw_retain(error) : NULL,
            snapshot.items[index].context);
    }
    nw_shim_snapshot_release(&snapshot);

    if (final_event) {
        dispatch_async_f(h->queue, h, nw_shim_listener_release_async);
    }
}

static void nw_shim_listener_cancel(nw_listener_handle *h) {
    pthread_mutex_lock(&h->lock);
    bool should_cancel = !h->cancel_requested && !h->cancelled;
    h->cancel_requested = true;
    pthread_mutex_unlock(&h->lock);
    if (should_cancel) {
        nw_listener_cancel(h->listener);
    }
}

static void nw_shim_listener_close_handle(nw_listener_handle *h) {
    nw_shim_acceptor_close(&h->acceptor);
    nw_shim_listener_cancel(h);
    nw_shim_listener_release_owner(h);
}

static void nw_shim_listener_on_group(nw_listener_handle *h, nw_connection_group_t group);

static void *nw_shim_listener_start(
    nw_listener_t listener,
    const char *label,
    nw_shim_callback *group_entry,
    int *out_status
) {
    if (!listener) {
        if (group_entry) nw_shim_callback_release(group_entry);
        if (out_status) *out_status = NW_LISTEN_FAILED;
        return NULL;
    }
    nw_listener_handle *h = (nw_listener_handle *)calloc(1, sizeof(nw_listener_handle));
    if (!h) {
        nw_release(listener);
        if (group_entry) nw_shim_callback_release(group_entry);
        if (out_status) *out_status = NW_LISTEN_FAILED;
        return NULL;
    }
    atomic_init(&h->refs, 2);
    h->listener = listener;
    h->queue = dispatch_queue_create(label ? label : "networkframework-rs.listener", DISPATCH_QUEUE_SERIAL);
    pthread_mutex_init(&h->lock, NULL);
    pthread_cond_init(&h->cond, NULL);
    nw_shim_subscriptions_init(&h->subs);
    nw_shim_acceptor_init(&h->acceptor, &h->subs, h, nw_shim_listener_retain_owner, nw_shim_listener_release_owner, true);

    nw_listener_set_queue(listener, h->queue);
    nw_listener_set_state_changed_handler(listener, ^(nw_listener_state_t state, nw_error_t error) {
        nw_shim_listener_on_state(h, state, error);
    });
    if (group_entry) {
        h->group_mode = true;
        nw_shim_subscriptions_add(&h->subs, NW_SHIM_EVENT_NEW_GROUP, *group_entry);
        nw_listener_set_new_connection_group_handler(listener, ^(nw_connection_group_t group) {
            nw_shim_listener_on_group(h, group);
        });
    } else {
        nw_listener_set_new_connection_handler(listener, ^(nw_connection_t connection) {
            nw_shim_acceptor_offer(&h->acceptor, connection, "networkframework-rs.accepted");
        });
    }
    nw_listener_start(listener);

    uint64_t deadline = nw_shim_deadline_after(NW_SHIM_START_TIMEOUT_NS);
    pthread_mutex_lock(&h->lock);
    while (h->state != nw_listener_state_ready && h->state != nw_listener_state_failed && !h->cancelled) {
        if (!nw_shim_cond_wait_until(&h->cond, &h->lock, deadline)) {
            break;
        }
    }
    bool ready = h->state == nw_listener_state_ready && !h->cancelled;
    pthread_mutex_unlock(&h->lock);
    if (!ready) {
        nw_shim_listener_close_handle(h);
        if (out_status) *out_status = NW_LISTEN_FAILED;
        return NULL;
    }
    if (out_status) *out_status = NW_OK;
    return h;
}

void *nw_shim_listener_create(uint16_t port, int use_tls, int *out_status) {
    char port_str[8];
    snprintf(port_str, sizeof(port_str), "%u", (unsigned)port);

    nw_parameters_t params = nw_parameters_create_secure_tcp(
        use_tls ? NW_PARAMETERS_DEFAULT_CONFIGURATION : NW_PARAMETERS_DISABLE_PROTOCOL,
        NW_PARAMETERS_DEFAULT_CONFIGURATION);
    if (!params) { if (out_status) *out_status = NW_LISTEN_FAILED; return NULL; }

    nw_listener_t listener = nw_listener_create_with_port(port_str, params);
    nw_release(params);
    return nw_shim_listener_start(listener, "networkframework-rs.listener", NULL, out_status);
}

uint16_t nw_shim_listener_port(void *handle) {
    nw_listener_handle *h = (nw_listener_handle *)handle;
    if (!h) return 0;
    return atomic_load(&h->bound_port);
}

// Blocking accept. Returns a `nw_conn_handle*` cast to void*, or NULL.
void *nw_shim_listener_accept(void *handle, int *out_status) {
    nw_listener_handle *h = (nw_listener_handle *)handle;
    if (!h || h->group_mode) { if (out_status) *out_status = NW_INVALID_ARG; return NULL; }
    return nw_shim_acceptor_accept(&h->acceptor, out_status);
}

void nw_shim_listener_close(void *handle) {
    nw_listener_handle *h = (nw_listener_handle *)handle;
    if (!h) return;
    nw_shim_listener_close_handle(h);
}

uint64_t nw_shim_listener_subscribe_state(
    void *handle,
    ListenerStateCallback callback,
    void *context,
    NwShimContextCallback retain,
    NwShimContextCallback release
) {
    nw_listener_handle *h = (nw_listener_handle *)handle;
    nw_shim_callback entry = nw_shim_make_callback((nw_shim_fn)callback, context, retain, release);
    if (!h) {
        nw_shim_callback_release(&entry);
        return 0;
    }
    return nw_shim_subscriptions_add(&h->subs, NW_SHIM_EVENT_STATE, entry);
}

uint64_t nw_shim_listener_subscribe_new_connection(
    void *handle,
    ListenerNewConnectionCallback callback,
    void *context,
    NwShimContextCallback retain,
    NwShimContextCallback release
) {
    nw_listener_handle *h = (nw_listener_handle *)handle;
    nw_shim_callback entry = nw_shim_make_callback((nw_shim_fn)callback, context, retain, release);
    if (!h) {
        nw_shim_callback_release(&entry);
        return 0;
    }
    uint64_t token = nw_shim_subscriptions_add(&h->subs, NW_SHIM_EVENT_NEW_CONNECTION, entry);
    if (token) {
        nw_shim_acceptor_dispatch(&h->acceptor);
    }
    return token;
}

void nw_shim_listener_unsubscribe(void *handle, uint64_t token) {
    nw_listener_handle *h = (nw_listener_handle *)handle;
    if (!h) return;
    nw_shim_subscriptions_remove(&h->subs, token);
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
    if (!params) {
        nw_release(endpoint);
        if (out_status) *out_status = NW_CONNECT_FAILED;
        return NULL;
    }

    nw_connection_t conn = nw_connection_create(endpoint, params);
    nw_release(endpoint);
    nw_release(params);
    if (!conn) { if (out_status) *out_status = NW_CONNECT_FAILED; return NULL; }

    return nw_shim_conn_start_and_wait(conn, "networkframework-rs.udp", NW_SHIM_START_TIMEOUT_NS, out_status);
}

// UDP send and close use the TCP shim entrypoints (nw_shim_tcp_send /
// nw_shim_tcp_close). Receives go through nw_shim_connection_receive_message
// so that each call returns one whole datagram.

// ---------------------------------------------------------------------
// Path monitor (network reachability / interface changes)
// ---------------------------------------------------------------------

typedef struct nw_path_handle {
    _Atomic long refs;
    nw_path_monitor_t monitor;
    dispatch_queue_t queue;
    pthread_mutex_t lock;
    nw_path_t latest_path;
    bool cancelled;
    bool cancel_requested;
    nw_shim_subscriptions subs;
} nw_path_handle;

static void nw_shim_path_release(nw_path_handle *h) {
    if (atomic_fetch_sub_explicit(&h->refs, 1, memory_order_acq_rel) != 1) {
        return;
    }
    nw_shim_subscriptions_destroy(&h->subs);
    if (h->latest_path) {
        nw_release(h->latest_path);
    }
    nw_release(h->monitor);
    dispatch_release(h->queue);
    pthread_mutex_destroy(&h->lock);
    free(h);
}

static void nw_shim_path_release_async(void *context) {
    nw_shim_path_release((nw_path_handle *)context);
}

static int nw_shim_path_interface_summary(nw_path_t path) {
    if (nw_path_uses_interface_type(path, nw_interface_type_wifi)) return 1;
    if (nw_path_uses_interface_type(path, nw_interface_type_cellular)) return 2;
    if (nw_path_uses_interface_type(path, nw_interface_type_wired)) return 3;
    if (nw_path_uses_interface_type(path, nw_interface_type_loopback)) return 4;
    return 0;
}

static void nw_shim_path_on_update(nw_path_handle *h, nw_path_t path) {
    pthread_mutex_lock(&h->lock);
    nw_path_t previous = h->latest_path;
    h->latest_path = path ? nw_retain(path) : NULL;
    pthread_mutex_unlock(&h->lock);
    if (previous) {
        nw_release(previous);
    }
    if (!path) {
        return;
    }

    nw_shim_snapshot snapshot;
    nw_shim_subscriptions_snapshot(&h->subs, NW_SHIM_EVENT_SUMMARY, &snapshot);
    if (snapshot.count > 0) {
        int satisfied = nw_path_get_status(path) == nw_path_status_satisfied ? 1 : 0;
        int interface_type = nw_shim_path_interface_summary(path);
        for (size_t index = 0; index < snapshot.count; index++) {
            ((PathMonitorCallback)snapshot.items[index].fn)(satisfied, interface_type, snapshot.items[index].context);
        }
    }
    nw_shim_snapshot_release(&snapshot);

    nw_shim_subscriptions_snapshot(&h->subs, NW_SHIM_EVENT_PATH, &snapshot);
    for (size_t index = 0; index < snapshot.count; index++) {
        ((ConnectionPathCallback)snapshot.items[index].fn)(nw_retain(path), snapshot.items[index].context);
    }
    nw_shim_snapshot_release(&snapshot);
}

static void nw_shim_path_on_cancel(nw_path_handle *h) {
    pthread_mutex_lock(&h->lock);
    bool already = h->cancelled;
    h->cancelled = true;
    pthread_mutex_unlock(&h->lock);
    if (already) {
        return;
    }
    nw_shim_snapshot snapshot;
    nw_shim_subscriptions_snapshot(&h->subs, NW_SHIM_EVENT_CANCEL, &snapshot);
    for (size_t index = 0; index < snapshot.count; index++) {
        ((PathMonitorCancelCallback)snapshot.items[index].fn)(snapshot.items[index].context);
    }
    nw_shim_snapshot_release(&snapshot);
    dispatch_async_f(h->queue, h, nw_shim_path_release_async);
}

// interface_type matches nw_interface_type_t:
//   0=other, 1=wifi, 2=cellular, 3=wired, 4=loopback
void *nw_shim_path_monitor_start(
    int scope,
    int interface_type,
    const int *prohibited_types,
    size_t prohibited_count,
    PathMonitorCallback callback,
    void *context,
    NwShimContextCallback retain,
    NwShimContextCallback release
) {
    nw_shim_callback entry = nw_shim_make_callback((nw_shim_fn)callback, context, retain, release);
    if (prohibited_count > 0 && !prohibited_types) {
        nw_shim_callback_release(&entry);
        return NULL;
    }
    nw_path_monitor_t monitor = NULL;
    const char *label = "networkframework-rs.path";
    if (scope == NW_SHIM_PATH_SCOPE_ALL) {
        monitor = nw_path_monitor_create();
    } else if (scope == NW_SHIM_PATH_SCOPE_INTERFACE_TYPE) {
        monitor = nw_path_monitor_create_with_type((nw_interface_type_t)interface_type);
        label = "networkframework-rs.path.type";
    } else if (scope == NW_SHIM_PATH_SCOPE_ETHERNET_CHANNEL) {
        monitor = nw_path_monitor_create_for_ethernet_channel();
        label = "networkframework-rs.path.ethernet";
    }
    if (!monitor) {
        nw_shim_callback_release(&entry);
        return NULL;
    }
    nw_path_handle *h = (nw_path_handle *)calloc(1, sizeof(nw_path_handle));
    if (!h) {
        nw_release(monitor);
        nw_shim_callback_release(&entry);
        return NULL;
    }
    atomic_init(&h->refs, 2);
    h->monitor = monitor;
    h->queue = dispatch_queue_create(label, DISPATCH_QUEUE_SERIAL);
    pthread_mutex_init(&h->lock, NULL);
    nw_shim_subscriptions_init(&h->subs);
    nw_shim_subscriptions_add(&h->subs, NW_SHIM_EVENT_SUMMARY, entry);

    for (size_t index = 0; index < prohibited_count; index++) {
        nw_path_monitor_prohibit_interface_type(monitor, (nw_interface_type_t)prohibited_types[index]);
    }
    nw_path_monitor_set_queue(monitor, h->queue);
    nw_path_monitor_set_update_handler(monitor, ^(nw_path_t path) {
        nw_shim_path_on_update(h, path);
    });
    nw_path_monitor_set_cancel_handler(monitor, ^{
        nw_shim_path_on_cancel(h);
    });
    nw_path_monitor_start(monitor);
    return h;
}

void nw_shim_path_monitor_stop(void *handle) {
    nw_path_handle *h = (nw_path_handle *)handle;
    if (!h) return;
    pthread_mutex_lock(&h->lock);
    bool should_cancel = !h->cancel_requested && !h->cancelled;
    h->cancel_requested = true;
    pthread_mutex_unlock(&h->lock);
    if (should_cancel) {
        nw_path_monitor_cancel(h->monitor);
    }
    nw_shim_path_release(h);
}

void *nw_shim_path_monitor_copy_latest_path(void *handle) {
    nw_path_handle *h = (nw_path_handle *)handle;
    if (!h) {
        return NULL;
    }
    pthread_mutex_lock(&h->lock);
    nw_path_t snapshot = h->latest_path ? nw_retain(h->latest_path) : NULL;
    pthread_mutex_unlock(&h->lock);
    return snapshot;
}

uint64_t nw_shim_path_monitor_subscribe_update(
    void *handle,
    ConnectionPathCallback callback,
    void *context,
    NwShimContextCallback retain,
    NwShimContextCallback release
) {
    nw_path_handle *h = (nw_path_handle *)handle;
    nw_shim_callback entry = nw_shim_make_callback((nw_shim_fn)callback, context, retain, release);
    if (!h) {
        nw_shim_callback_release(&entry);
        return 0;
    }
    return nw_shim_subscriptions_add(&h->subs, NW_SHIM_EVENT_PATH, entry);
}

uint64_t nw_shim_path_monitor_subscribe_cancel(
    void *handle,
    PathMonitorCancelCallback callback,
    void *context,
    NwShimContextCallback retain,
    NwShimContextCallback release
) {
    nw_path_handle *h = (nw_path_handle *)handle;
    nw_shim_callback entry = nw_shim_make_callback((nw_shim_fn)callback, context, retain, release);
    if (!h) {
        nw_shim_callback_release(&entry);
        return 0;
    }
    return nw_shim_subscriptions_add(&h->subs, NW_SHIM_EVENT_CANCEL, entry);
}

void nw_shim_path_monitor_unsubscribe(void *handle, uint64_t token) {
    nw_path_handle *h = (nw_path_handle *)handle;
    if (!h) return;
    nw_shim_subscriptions_remove(&h->subs, token);
}

// ---------------------------------------------------------------------
// Bonjour browser (nw_browser)
// ---------------------------------------------------------------------

typedef struct nw_browser_handle {
    _Atomic long refs;
    nw_browser_t browser;
    dispatch_queue_t queue;
    pthread_mutex_t lock;
    bool cancelled;
    bool cancel_requested;
    nw_shim_subscriptions subs;
} nw_browser_handle;

static void nw_shim_browser_release(nw_browser_handle *h) {
    if (atomic_fetch_sub_explicit(&h->refs, 1, memory_order_acq_rel) != 1) {
        return;
    }
    nw_shim_subscriptions_destroy(&h->subs);
    nw_release(h->browser);
    dispatch_release(h->queue);
    pthread_mutex_destroy(&h->lock);
    free(h);
}

static void nw_shim_browser_release_async(void *context) {
    nw_shim_browser_release((nw_browser_handle *)context);
}

static void nw_shim_browser_on_state(nw_browser_handle *h, nw_browser_state_t state, nw_error_t error) {
    pthread_mutex_lock(&h->lock);
    if (h->cancelled) {
        pthread_mutex_unlock(&h->lock);
        return;
    }
    bool final_event = state == nw_browser_state_cancelled;
    if (final_event) {
        h->cancelled = true;
    }
    pthread_mutex_unlock(&h->lock);

    nw_shim_snapshot snapshot;
    nw_shim_subscriptions_snapshot(&h->subs, NW_SHIM_EVENT_STATE, &snapshot);
    for (size_t index = 0; index < snapshot.count; index++) {
        ((BrowserStateChangedCallback)snapshot.items[index].fn)(
            (int)state,
            error ? nw_retain(error) : NULL,
            snapshot.items[index].context);
    }
    nw_shim_snapshot_release(&snapshot);

    if (final_event) {
        dispatch_async_f(h->queue, h, nw_shim_browser_release_async);
    }
}

static void nw_shim_browser_on_results(
    nw_browser_handle *h,
    nw_browse_result_t old_result,
    nw_browse_result_t new_result,
    bool batch_complete
) {
    nw_shim_snapshot snapshot;
    nw_shim_subscriptions_snapshot(&h->subs, NW_SHIM_EVENT_RESULTS, &snapshot);
    if (snapshot.count > 0) {
        uint64_t changes = nw_browse_result_get_changes(old_result, new_result);
        for (size_t index = 0; index < snapshot.count; index++) {
            ((BrowseResultChangedCallback)snapshot.items[index].fn)(
                old_result ? nw_retain(old_result) : NULL,
                new_result ? nw_retain(new_result) : NULL,
                changes,
                batch_complete ? 1 : 0,
                snapshot.items[index].context);
        }
    }
    nw_shim_snapshot_release(&snapshot);

    if ((old_result == NULL) == (new_result == NULL)) {
        return;
    }
    nw_shim_subscriptions_snapshot(&h->subs, NW_SHIM_EVENT_SERVICE, &snapshot);
    if (snapshot.count > 0) {
        nw_endpoint_t ep = nw_browse_result_copy_endpoint(new_result ? new_result : old_result);
        if (ep) {
            const char *name = nw_endpoint_get_bonjour_service_name(ep);
            const char *type = nw_endpoint_get_bonjour_service_type(ep);
            const char *dom = nw_endpoint_get_bonjour_service_domain(ep);
            if (!name || !name[0]) {
                name = nw_endpoint_get_hostname(ep);
            }
            for (size_t index = 0; index < snapshot.count; index++) {
                ((BrowserServiceEventCallback)snapshot.items[index].fn)(
                    new_result ? 1 : 0,
                    name ? name : "",
                    type ? type : "",
                    dom ? dom : "",
                    snapshot.items[index].context);
            }
            nw_release(ep);
        }
    }
    nw_shim_snapshot_release(&snapshot);
}

static void *nw_shim_browser_start_common(void *descriptor, void *parameters, int kind, nw_shim_callback entry, const char *label) {
    if (!descriptor) {
        nw_shim_callback_release(&entry);
        return NULL;
    }
    nw_parameters_t params = parameters ? nw_parameters_copy((nw_parameters_t)parameters) : nw_parameters_create();
    if (!params) {
        nw_shim_callback_release(&entry);
        return NULL;
    }
    nw_browser_t browser = nw_browser_create((nw_browse_descriptor_t)descriptor, params);
    nw_release(params);
    if (!browser) {
        nw_shim_callback_release(&entry);
        return NULL;
    }
    nw_browser_handle *h = (nw_browser_handle *)calloc(1, sizeof(nw_browser_handle));
    if (!h) {
        nw_release(browser);
        nw_shim_callback_release(&entry);
        return NULL;
    }
    atomic_init(&h->refs, 2);
    h->browser = browser;
    h->queue = dispatch_queue_create(label, DISPATCH_QUEUE_SERIAL);
    pthread_mutex_init(&h->lock, NULL);
    nw_shim_subscriptions_init(&h->subs);
    nw_shim_subscriptions_add(&h->subs, kind, entry);

    nw_browser_set_queue(browser, h->queue);
    nw_browser_set_state_changed_handler(browser, ^(nw_browser_state_t state, nw_error_t error) {
        nw_shim_browser_on_state(h, state, error);
    });
    nw_browser_set_browse_results_changed_handler(browser,
        ^(nw_browse_result_t old_result, nw_browse_result_t new_result, bool batch_complete) {
            nw_shim_browser_on_results(h, old_result, new_result, batch_complete);
        });
    nw_browser_start(browser);
    return h;
}

void *nw_shim_browser_start_with_descriptor(
    void *descriptor,
    void *parameters,
    BrowserServiceEventCallback callback,
    void *context,
    NwShimContextCallback retain,
    NwShimContextCallback release
) {
    return nw_shim_browser_start_common(
        descriptor,
        parameters,
        NW_SHIM_EVENT_SERVICE,
        nw_shim_make_callback((nw_shim_fn)callback, context, retain, release),
        "networkframework-rs.browser");
}

void *nw_shim_browser_start_results_with_descriptor(
    void *descriptor,
    void *parameters,
    BrowseResultChangedCallback callback,
    void *context,
    NwShimContextCallback retain,
    NwShimContextCallback release
) {
    return nw_shim_browser_start_common(
        descriptor,
        parameters,
        NW_SHIM_EVENT_RESULTS,
        nw_shim_make_callback((nw_shim_fn)callback, context, retain, release),
        "networkframework-rs.browser.results");
}

uint64_t nw_shim_browser_subscribe_state(
    void *handle,
    BrowserStateChangedCallback callback,
    void *context,
    NwShimContextCallback retain,
    NwShimContextCallback release
) {
    nw_browser_handle *h = (nw_browser_handle *)handle;
    nw_shim_callback entry = nw_shim_make_callback((nw_shim_fn)callback, context, retain, release);
    if (!h) {
        nw_shim_callback_release(&entry);
        return 0;
    }
    return nw_shim_subscriptions_add(&h->subs, NW_SHIM_EVENT_STATE, entry);
}

uint64_t nw_shim_browser_subscribe_results(
    void *handle,
    BrowseResultChangedCallback callback,
    void *context,
    NwShimContextCallback retain,
    NwShimContextCallback release
) {
    nw_browser_handle *h = (nw_browser_handle *)handle;
    nw_shim_callback entry = nw_shim_make_callback((nw_shim_fn)callback, context, retain, release);
    if (!h) {
        nw_shim_callback_release(&entry);
        return 0;
    }
    return nw_shim_subscriptions_add(&h->subs, NW_SHIM_EVENT_RESULTS, entry);
}

void nw_shim_browser_unsubscribe(void *handle, uint64_t token) {
    nw_browser_handle *h = (nw_browser_handle *)handle;
    if (!h) return;
    nw_shim_subscriptions_remove(&h->subs, token);
}

void nw_shim_browser_stop(void *handle) {
    nw_browser_handle *h = (nw_browser_handle *)handle;
    if (!h) return;
    pthread_mutex_lock(&h->lock);
    bool should_cancel = !h->cancel_requested && !h->cancelled;
    h->cancel_requested = true;
    pthread_mutex_unlock(&h->lock);
    if (should_cancel) {
        nw_browser_cancel(h->browser);
    }
    nw_shim_browser_release(h);
}

// ---------------------------------------------------------------------
// WebSocket (nw_ws_*) connection — v0.5
// ---------------------------------------------------------------------
// Builds a websocket connection by chaining ws -> tcp / tls protocols
// into a custom nw_parameters_t.

void *nw_shim_ws_connect(const char *url, int use_tls, int *out_status) {
    if (!url) { if (out_status) *out_status = NW_INVALID_ARG; return NULL; }

    nw_parameters_t params = nw_parameters_create_secure_tcp(
        use_tls ? NW_PARAMETERS_DEFAULT_CONFIGURATION : NW_PARAMETERS_DISABLE_PROTOCOL,
        NW_PARAMETERS_DEFAULT_CONFIGURATION);
    if (!params) { if (out_status) *out_status = NW_CONNECT_FAILED; return NULL; }

    // Insert WebSocket framing on top of (TLS)TCP.
    nw_protocol_options_t ws_opts = nw_ws_create_options(nw_ws_version_13);
    nw_protocol_stack_t stack = nw_parameters_copy_default_protocol_stack(params);
    if (stack && ws_opts) {
        nw_protocol_stack_prepend_application_protocol(stack, ws_opts);
    }
    if (ws_opts) nw_release(ws_opts);
    if (stack) nw_release(stack);

    // URL endpoint encodes ws:// or wss:// + host + port + path.
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

    return nw_shim_conn_start_and_wait(conn, "networkframework-rs.ws", NW_SHIM_CONNECT_TIMEOUT_NS, out_status);
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

ssize_t nw_shim_ws_receive(void *handle, uint8_t *out_buf, size_t max_len, int *out_opcode, size_t *out_size) {
    nw_conn_handle *h = (nw_conn_handle *)handle;
    if (out_size) *out_size = 0;
    if (out_opcode) *out_opcode = 0;
    if (!h || !out_buf || max_len == 0) return NW_INVALID_ARG;

    __block ssize_t result = 0;
    __block int op = 0;
    __block size_t size = 0;
    nw_protocol_definition_t definition = nw_protocol_copy_ws_definition();
    dispatch_semaphore_t done = dispatch_semaphore_create(0);

    nw_connection_receive_message(h->conn,
        ^(dispatch_data_t content, nw_content_context_t ctx, bool is_complete, nw_error_t error) {
            (void)is_complete;
            if (error) {
                result = NW_RECV_FAILED;
            } else {
                if (ctx && definition) {
                    nw_protocol_metadata_t md = nw_content_context_copy_protocol_metadata(ctx, definition);
                    if (md) {
                        op = (int)nw_ws_metadata_get_opcode(md);
                        nw_release(md);
                    }
                }
                result = nw_shim_copy_received(content, out_buf, max_len, &size);
            }
            dispatch_semaphore_signal(done);
        });

    dispatch_semaphore_wait(done, DISPATCH_TIME_FOREVER);
    dispatch_release(done);
    if (definition) nw_release(definition);
    if (out_opcode) *out_opcode = op;
    if (out_size) *out_size = size;
    return result;
}

// ---------------------------------------------------------------------
// QUIC (single-stream) — v0.6
// ---------------------------------------------------------------------
// Builds a QUIC connection with one bidirectional stream from
// nw_parameters_create_quic. send/receive on the returned handle work the
// same as the TCP variant.

static nw_endpoint_t nw_shim_create_host_endpoint(const char *host, uint16_t port);
static nw_parameters_t nw_shim_create_quic_parameters(const char *alpn);

void *nw_shim_quic_connect(const char *host, uint16_t port, const char *alpn, int *out_status) {
    if (!host) { if (out_status) *out_status = NW_INVALID_ARG; return NULL; }

    nw_endpoint_t endpoint = nw_shim_create_host_endpoint(host, port);
    if (!endpoint) { if (out_status) *out_status = NW_INVALID_ARG; return NULL; }

    nw_parameters_t params = nw_shim_create_quic_parameters(alpn);
    if (!params) {
        nw_release(endpoint);
        if (out_status) *out_status = NW_CONNECT_FAILED;
        return NULL;
    }

    nw_connection_t conn = nw_connection_create(endpoint, params);
    nw_release(endpoint);
    nw_release(params);
    if (!conn) { if (out_status) *out_status = NW_CONNECT_FAILED; return NULL; }

    return nw_shim_conn_start_and_wait(conn, "networkframework-rs.quic", NW_SHIM_CONNECT_TIMEOUT_NS, out_status);
}

// ---------------------------------------------------------------------
// Bonjour advertisement (nw_listener with a bonjour service endpoint)
// ---------------------------------------------------------------------

typedef struct nw_bonjour_advertise_handle {
    _Atomic long refs;
    nw_listener_t listener;
    dispatch_queue_t queue;
    pthread_mutex_t lock;
    pthread_cond_t cond;
    int state;
    bool cancelled;
    bool cancel_requested;
} nw_bonjour_advertise_handle;

static void nw_shim_advertise_release(nw_bonjour_advertise_handle *h) {
    if (atomic_fetch_sub_explicit(&h->refs, 1, memory_order_acq_rel) != 1) {
        return;
    }
    nw_release(h->listener);
    dispatch_release(h->queue);
    pthread_cond_destroy(&h->cond);
    pthread_mutex_destroy(&h->lock);
    free(h);
}

static void nw_shim_advertise_release_async(void *context) {
    nw_shim_advertise_release((nw_bonjour_advertise_handle *)context);
}

static void nw_shim_advertise_on_state(nw_bonjour_advertise_handle *h, nw_listener_state_t state) {
    pthread_mutex_lock(&h->lock);
    if (h->cancelled) {
        pthread_mutex_unlock(&h->lock);
        return;
    }
    h->state = (int)state;
    bool final_event = state == nw_listener_state_cancelled;
    if (final_event) {
        h->cancelled = true;
    }
    pthread_cond_broadcast(&h->cond);
    pthread_mutex_unlock(&h->lock);
    if (final_event) {
        dispatch_async_f(h->queue, h, nw_shim_advertise_release_async);
    }
}

static void nw_shim_advertise_close(nw_bonjour_advertise_handle *h) {
    pthread_mutex_lock(&h->lock);
    bool should_cancel = !h->cancel_requested && !h->cancelled;
    h->cancel_requested = true;
    pthread_mutex_unlock(&h->lock);
    if (should_cancel) {
        nw_listener_cancel(h->listener);
    }
    nw_shim_advertise_release(h);
}

static void *nw_shim_advertise_start(nw_listener_t listener, const char *label, int *out_status) {
    if (!listener) {
        if (out_status) *out_status = NW_LISTEN_FAILED;
        return NULL;
    }
    nw_bonjour_advertise_handle *h = (nw_bonjour_advertise_handle *)calloc(1, sizeof(nw_bonjour_advertise_handle));
    if (!h) {
        nw_release(listener);
        if (out_status) *out_status = NW_LISTEN_FAILED;
        return NULL;
    }
    atomic_init(&h->refs, 2);
    h->listener = listener;
    h->queue = dispatch_queue_create(label, DISPATCH_QUEUE_SERIAL);
    pthread_mutex_init(&h->lock, NULL);
    pthread_cond_init(&h->cond, NULL);

    nw_listener_set_queue(listener, h->queue);
    nw_listener_set_state_changed_handler(listener, ^(nw_listener_state_t state, nw_error_t error) {
        (void)error;
        nw_shim_advertise_on_state(h, state);
    });
    nw_listener_set_new_connection_handler(listener, ^(nw_connection_t conn) {
        // Drop inbound connections — we're advertising only.
        nw_connection_cancel(conn);
    });
    nw_listener_start(listener);

    uint64_t deadline = nw_shim_deadline_after(NW_SHIM_START_TIMEOUT_NS);
    pthread_mutex_lock(&h->lock);
    while (h->state != nw_listener_state_ready && h->state != nw_listener_state_failed && !h->cancelled) {
        if (!nw_shim_cond_wait_until(&h->cond, &h->lock, deadline)) {
            break;
        }
    }
    bool ready = h->state == nw_listener_state_ready && !h->cancelled;
    pthread_mutex_unlock(&h->lock);
    if (!ready) {
        nw_shim_advertise_close(h);
        if (out_status) *out_status = NW_LISTEN_FAILED;
        return NULL;
    }
    if (out_status) *out_status = NW_OK;
    return h;
}

static nw_listener_t nw_shim_advertise_listener(uint16_t port, nw_advertise_descriptor_t descriptor) {
    nw_parameters_t params = nw_parameters_create_secure_tcp(
        NW_PARAMETERS_DISABLE_PROTOCOL,
        NW_PARAMETERS_DEFAULT_CONFIGURATION);
    if (!params) {
        return NULL;
    }
    char port_str[8];
    snprintf(port_str, sizeof(port_str), "%u", (unsigned)port);
    nw_listener_t listener = nw_listener_create_with_port(port_str, params);
    nw_release(params);
    if (listener) {
        nw_listener_set_advertise_descriptor(listener, descriptor);
    }
    return listener;
}

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
    nw_advertise_descriptor_t adv = nw_advertise_descriptor_create_bonjour_service(
        service_name,
        service_type,
        (domain && domain[0]) ? domain : NULL);
    if (!adv) {
        if (out_status) *out_status = NW_INVALID_ARG;
        return NULL;
    }
    nw_listener_t listener = nw_shim_advertise_listener(port, adv);
    nw_release(adv);
    return nw_shim_advertise_start(listener, "networkframework-rs.bonjour-adv", out_status);
}

void *nw_shim_bonjour_advertise_start_with_descriptor(void *descriptor, uint16_t port, int *out_status) {
    if (!descriptor) {
        if (out_status) *out_status = NW_INVALID_ARG;
        return NULL;
    }
    nw_listener_t listener = nw_shim_advertise_listener(port, (nw_advertise_descriptor_t)descriptor);
    return nw_shim_advertise_start(listener, "networkframework-rs.advertise-descriptor", out_status);
}

void nw_shim_bonjour_advertise_stop(void *handle) {
    nw_bonjour_advertise_handle *h = (nw_bonjour_advertise_handle *)handle;
    if (!h) return;
    nw_shim_advertise_close(h);
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
    return nw_parameters_create_quic(^(nw_protocol_options_t options) {
        if (alpn && alpn[0]) {
            nw_quic_add_tls_application_protocol(options, alpn);
        }
    });
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

    return nw_shim_conn_start_and_wait(conn, "networkframework-rs.conn.params", NW_SHIM_CONNECT_TIMEOUT_NS, out_status);
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

    nw_shim_waiter *waiter = nw_shim_waiter_create(2);
    if (!waiter) {
        nw_release(connection);
        return NULL;
    }

    dispatch_queue_t queue = dispatch_queue_create("networkframework-rs.test.error", DISPATCH_QUEUE_SERIAL);
    nw_connection_set_queue(connection, queue);
    nw_connection_set_state_changed_handler(connection, ^(nw_connection_state_t state, nw_error_t error) {
        if (error
            || state == nw_connection_state_ready
            || state == nw_connection_state_failed
            || state == nw_connection_state_waiting
            || state == nw_connection_state_cancelled) {
            nw_shim_waiter_complete(waiter, error ? nw_retain(error) : NULL);
        }
        if (state == nw_connection_state_cancelled) {
            nw_shim_waiter_release(waiter);
        }
    });
    nw_connection_start(connection);

    void *retained_error = nw_shim_waiter_wait_take(waiter, 10LL * (int64_t)NSEC_PER_SEC, NULL);
    nw_connection_cancel(connection);
    nw_release(connection);
    dispatch_release(queue);
    nw_shim_waiter_release(waiter);
    return retained_error;
}

void *nw_shim_listener_create_for_groups(
    void *parameters,
    uint16_t port,
    ListenerNewConnectionGroupCallback callback,
    void *context,
    NwShimContextCallback retain,
    NwShimContextCallback release,
    int *out_status
) {
    nw_shim_callback entry = nw_shim_make_callback((nw_shim_fn)callback, context, retain, release);
    if (!parameters || !callback) {
        nw_shim_callback_release(&entry);
        if (out_status) *out_status = NW_INVALID_ARG;
        return NULL;
    }

    char port_str[8];
    snprintf(port_str, sizeof(port_str), "%u", (unsigned)port);

    nw_listener_t listener = nw_listener_create_with_port(port_str, (nw_parameters_t)parameters);
    return nw_shim_listener_start(listener, "networkframework-rs.listener.groups", &entry, out_status);
}

void *nw_shim_listener_create_with_parameters(void *parameters, uint16_t port, int *out_status) {
    if (!parameters) {
        if (out_status) *out_status = NW_INVALID_ARG;
        return NULL;
    }

    char port_str[8];
    snprintf(port_str, sizeof(port_str), "%u", (unsigned)port);

    nw_listener_t listener = nw_listener_create_with_port(port_str, (nw_parameters_t)parameters);
    return nw_shim_listener_start(listener, "networkframework-rs.listener.params", NULL, out_status);
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

static ssize_t nw_shim_connection_receive_common(
    nw_conn_handle *h,
    bool whole_message,
    uint8_t *out_buf,
    size_t max_len,
    size_t *out_size,
    void **out_context,
    int *out_is_complete
) {
    if (out_size) *out_size = 0;
    if (out_context) *out_context = NULL;
    if (out_is_complete) *out_is_complete = 0;
    if (!h || !out_buf || max_len == 0) {
        return NW_INVALID_ARG;
    }

    __block ssize_t result = 0;
    __block size_t size = 0;
    __block void *retained_context = NULL;
    __block int is_complete_result = 0;
    dispatch_semaphore_t done = dispatch_semaphore_create(0);

    nw_connection_receive_completion_t completion =
        ^(dispatch_data_t content, nw_content_context_t ctx, bool is_complete, nw_error_t error) {
            if (error) {
                result = NW_RECV_FAILED;
            } else {
                is_complete_result = is_complete ? 1 : 0;
                if (ctx) {
                    retained_context = nw_retain(ctx);
                }
                result = nw_shim_copy_received(content, out_buf, max_len, &size);
            }
            dispatch_semaphore_signal(done);
        };
    if (whole_message) {
        nw_connection_receive_message(h->conn, completion);
    } else {
        nw_connection_receive(h->conn, 1, nw_shim_clamp_receive_length(max_len), completion);
    }

    dispatch_semaphore_wait(done, DISPATCH_TIME_FOREVER);
    dispatch_release(done);

    if (out_size) *out_size = size;
    if (result >= 0) {
        if (out_context) {
            *out_context = retained_context;
            retained_context = NULL;
        }
        if (out_is_complete) {
            *out_is_complete = is_complete_result;
        }
    }
    if (retained_context) {
        nw_release((nw_content_context_t)retained_context);
    }
    return result;
}

ssize_t nw_shim_connection_receive_with_context(
    void *handle,
    uint8_t *out_buf,
    size_t max_len,
    size_t *out_size,
    void **out_context,
    int *out_is_complete
) {
    return nw_shim_connection_receive_common(
        (nw_conn_handle *)handle, false, out_buf, max_len, out_size, out_context, out_is_complete);
}

ssize_t nw_shim_connection_receive_message(
    void *handle,
    uint8_t *out_buf,
    size_t max_len,
    size_t *out_size,
    void **out_context,
    int *out_is_complete
) {
    return nw_shim_connection_receive_common(
        (nw_conn_handle *)handle, true, out_buf, max_len, out_size, out_context, out_is_complete);
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
    if (!handle || !callback) {
        return 0;
    }
    nw_path_t snapshot = (nw_path_t)nw_shim_path_monitor_copy_latest_path(handle);
    int count = nw_shim_enumerate_path_interfaces(snapshot, callback, user_info);
    if (snapshot) {
        nw_release(snapshot);
    }
    return count;
}

static nw_path_t nw_shim_copy_current_path(const char *label) {
    nw_shim_waiter *waiter = nw_shim_waiter_create(2);
    if (!waiter) {
        return NULL;
    }
    nw_path_monitor_t monitor = nw_path_monitor_create();
    if (!monitor) {
        nw_shim_waiter_release(waiter);
        nw_shim_waiter_release(waiter);
        return NULL;
    }
    dispatch_queue_t queue = dispatch_queue_create(label, DISPATCH_QUEUE_SERIAL);
    nw_path_monitor_set_queue(monitor, queue);
    nw_path_monitor_set_update_handler(monitor, ^(nw_path_t path) {
        if (path) {
            nw_shim_waiter_complete(waiter, nw_retain(path));
        }
    });
    nw_path_monitor_set_cancel_handler(monitor, ^{
        nw_shim_waiter_release(waiter);
    });
    nw_path_monitor_start(monitor);

    nw_path_t path = (nw_path_t)nw_shim_waiter_wait_take(waiter, 5LL * (int64_t)NSEC_PER_SEC, NULL);
    nw_path_monitor_cancel(monitor);
    nw_release(monitor);
    dispatch_release(queue);
    nw_shim_waiter_release(waiter);
    return path;
}

int nw_shim_list_interfaces(
    int (*callback)(const char *name, int interface_type, uint32_t index, void *user_info),
    void *user_info
) {
    if (!callback) {
        return 0;
    }
    nw_path_t path = nw_shim_copy_current_path("networkframework-rs.interface-list");
    if (!path) {
        return 0;
    }
    int count = nw_shim_enumerate_path_interfaces(path, callback, user_info);
    nw_release(path);
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

typedef size_t (*nw_shim_framer_parse_fn)(
    const uint8_t *buffer,
    size_t buffer_length,
    int is_complete,
    void *user_info);
typedef void (*nw_shim_framer_async_fn)(void *framer, void *user_info);

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
        return nw_framer_deliver_input_no_copy(
            (nw_framer_t)framer,
            input_length,
            (nw_framer_message_t)message,
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
        if (input_length == 0) {
            nw_framer_deliver_input_no_copy((nw_framer_t)framer, 0, (nw_framer_message_t)message, is_complete != 0);
            return;
        }
        nw_framer_deliver_input(
            (nw_framer_t)framer,
            input_buffer,
            input_length,
            (nw_framer_message_t)message,
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
typedef struct nw_connection_group_handle {
    _Atomic long refs;
    nw_connection_group_t group;
    dispatch_queue_t queue;
    pthread_mutex_t lock;
    pthread_cond_t cond;
    int state;
    bool cancelled;
    bool cancel_requested;
    bool started;
    bool new_connection_installed;
    nw_shim_subscriptions subs;
    nw_shim_acceptor acceptor;
} nw_connection_group_handle;

static void nw_shim_group_retain_owner(void *owner) {
    atomic_fetch_add_explicit(&((nw_connection_group_handle *)owner)->refs, 1, memory_order_relaxed);
}

static void nw_shim_group_release_owner(void *owner) {
    nw_connection_group_handle *h = (nw_connection_group_handle *)owner;
    if (atomic_fetch_sub_explicit(&h->refs, 1, memory_order_acq_rel) != 1) {
        return;
    }
    nw_shim_acceptor_destroy(&h->acceptor);
    nw_shim_subscriptions_destroy(&h->subs);
    nw_release(h->group);
    dispatch_release(h->queue);
    pthread_cond_destroy(&h->cond);
    pthread_mutex_destroy(&h->lock);
    free(h);
}

static void nw_shim_group_release_async(void *context) {
    nw_shim_group_release_owner(context);
}

static void nw_shim_group_on_state(nw_connection_group_handle *h, nw_connection_group_state_t state) {
    pthread_mutex_lock(&h->lock);
    if (h->cancelled) {
        pthread_mutex_unlock(&h->lock);
        return;
    }
    h->state = (int)state;
    bool final_event = state == nw_connection_group_state_cancelled;
    if (final_event) {
        h->cancelled = true;
    }
    pthread_cond_broadcast(&h->cond);
    pthread_mutex_unlock(&h->lock);

    if (state == nw_connection_group_state_failed || final_event) {
        nw_shim_acceptor_mark_terminal(&h->acceptor);
    }

    nw_shim_snapshot snapshot;
    nw_shim_subscriptions_snapshot(&h->subs, NW_SHIM_EVENT_STATE, &snapshot);
    for (size_t index = 0; index < snapshot.count; index++) {
        ((ConnectionGroupStateCallback)snapshot.items[index].fn)((int)state, snapshot.items[index].context);
    }
    nw_shim_snapshot_release(&snapshot);

    if (final_event) {
        dispatch_async_f(h->queue, h, nw_shim_group_release_async);
    }
}

static void nw_shim_group_on_receive(
    nw_connection_group_handle *h,
    dispatch_data_t content,
    nw_content_context_t context,
    bool is_complete
) {
    nw_shim_snapshot snapshot;
    nw_shim_subscriptions_snapshot(&h->subs, NW_SHIM_EVENT_RECEIVE, &snapshot);
    if (snapshot.count > 0) {
        uint8_t *bytes = NULL;
        size_t length = nw_shim_copy_dispatch_data(content, &bytes);
        for (size_t index = 0; index < snapshot.count; index++) {
            ((ConnectionGroupReceiveCallback)snapshot.items[index].fn)(
                bytes,
                length,
                context ? nw_retain(context) : NULL,
                is_complete ? 1 : 0,
                snapshot.items[index].context);
        }
        free(bytes);
    }
    nw_shim_snapshot_release(&snapshot);
}

static nw_connection_group_handle *nw_shim_group_create_handle(nw_connection_group_t group, const char *label) {
    if (!group) {
        return NULL;
    }
    nw_connection_group_handle *h = (nw_connection_group_handle *)calloc(1, sizeof(nw_connection_group_handle));
    if (!h) {
        nw_connection_group_cancel(group);
        nw_release(group);
        return NULL;
    }
    atomic_init(&h->refs, 2);
    h->group = group;
    h->queue = dispatch_queue_create(label, DISPATCH_QUEUE_SERIAL);
    pthread_mutex_init(&h->lock, NULL);
    pthread_cond_init(&h->cond, NULL);
    nw_shim_subscriptions_init(&h->subs);
    nw_shim_acceptor_init(&h->acceptor, &h->subs, h, nw_shim_group_retain_owner, nw_shim_group_release_owner, false);

    nw_connection_group_set_queue(group, h->queue);
    nw_connection_group_set_state_changed_handler(group, ^(nw_connection_group_state_t state, nw_error_t error) {
        (void)error;
        nw_shim_group_on_state(h, state);
    });
    return h;
}

static void nw_shim_group_cancel(nw_connection_group_handle *h) {
    pthread_mutex_lock(&h->lock);
    bool should_cancel = !h->cancel_requested && !h->cancelled;
    h->cancel_requested = true;
    pthread_mutex_unlock(&h->lock);
    if (should_cancel) {
        nw_connection_group_cancel(h->group);
    }
}

static bool nw_shim_group_started(nw_connection_group_handle *h) {
    pthread_mutex_lock(&h->lock);
    bool started = h->started;
    pthread_mutex_unlock(&h->lock);
    return started;
}

void *nw_shim_group_descriptor_create_multiplex(const char *host, uint16_t port) {
    if (!host) {
        return NULL;
    }
    nw_endpoint_t endpoint = nw_shim_create_host_endpoint(host, port);
    if (!endpoint) {
        return NULL;
    }
    nw_group_descriptor_t descriptor = nw_group_descriptor_create_multiplex(endpoint);
    nw_release(endpoint);
    return descriptor;
}

void *nw_shim_group_descriptor_create_multicast(const char *group_address, uint16_t port) {
    if (!group_address) {
        return NULL;
    }
    nw_endpoint_t endpoint = nw_shim_create_address_endpoint(group_address, port);
    if (!endpoint) {
        return NULL;
    }
    nw_group_descriptor_t descriptor = nw_group_descriptor_create_multicast(endpoint);
    nw_release(endpoint);
    return descriptor;
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
    nw_connection_group_t group = nw_connection_group_create(
        (nw_group_descriptor_t)descriptor,
        (nw_parameters_t)parameters);
    return nw_shim_group_create_handle(group, "networkframework-rs.connection-group");
}

uint64_t nw_shim_connection_group_subscribe_state(
    void *handle,
    ConnectionGroupStateCallback callback,
    void *context,
    NwShimContextCallback retain,
    NwShimContextCallback release
) {
    nw_connection_group_handle *h = (nw_connection_group_handle *)handle;
    nw_shim_callback entry = nw_shim_make_callback((nw_shim_fn)callback, context, retain, release);
    if (!h) {
        nw_shim_callback_release(&entry);
        return 0;
    }
    return nw_shim_subscriptions_add(&h->subs, NW_SHIM_EVENT_STATE, entry);
}

uint64_t nw_shim_connection_group_subscribe_receive(
    void *handle,
    uint32_t maximum_message_size,
    int reject_oversized_messages,
    ConnectionGroupReceiveCallback callback,
    void *context,
    NwShimContextCallback retain,
    NwShimContextCallback release
) {
    nw_connection_group_handle *h = (nw_connection_group_handle *)handle;
    nw_shim_callback entry = nw_shim_make_callback((nw_shim_fn)callback, context, retain, release);
    if (!h || nw_shim_group_started(h)) {
        nw_shim_callback_release(&entry);
        return 0;
    }
    uint64_t token = nw_shim_subscriptions_add(&h->subs, NW_SHIM_EVENT_RECEIVE, entry);
    if (token) {
        nw_connection_group_set_receive_handler(
            h->group,
            maximum_message_size,
            reject_oversized_messages != 0,
            ^(dispatch_data_t content, nw_content_context_t message_context, bool is_complete) {
                nw_shim_group_on_receive(h, content, message_context, is_complete);
            });
    }
    return token;
}

uint64_t nw_shim_connection_group_subscribe_new_connection(
    void *handle,
    ConnectionGroupNewConnectionCallback callback,
    void *context,
    NwShimContextCallback retain,
    NwShimContextCallback release
) {
    nw_connection_group_handle *h = (nw_connection_group_handle *)handle;
    nw_shim_callback entry = nw_shim_make_callback((nw_shim_fn)callback, context, retain, release);
    if (!h || nw_shim_group_started(h)) {
        nw_shim_callback_release(&entry);
        return 0;
    }
    uint64_t token = nw_shim_subscriptions_add(&h->subs, NW_SHIM_EVENT_NEW_CONNECTION, entry);
    if (token) {
        pthread_mutex_lock(&h->lock);
        bool install = !h->new_connection_installed;
        h->new_connection_installed = true;
        pthread_mutex_unlock(&h->lock);
        if (install) {
            nw_connection_group_set_new_connection_handler(h->group, ^(nw_connection_t connection) {
                nw_shim_acceptor_offer(&h->acceptor, connection, "networkframework-rs.connection-group.connection");
            });
        }
    }
    return token;
}

void nw_shim_connection_group_unsubscribe(void *handle, uint64_t token) {
    nw_connection_group_handle *h = (nw_connection_group_handle *)handle;
    if (!h) return;
    nw_shim_subscriptions_remove(&h->subs, token);
}

int nw_shim_connection_group_start(void *handle) {
    nw_connection_group_handle *h = (nw_connection_group_handle *)handle;
    if (!h) {
        return NW_INVALID_ARG;
    }
    pthread_mutex_lock(&h->lock);
    bool already = h->started;
    h->started = true;
    pthread_mutex_unlock(&h->lock);
    if (already) {
        return NW_INVALID_ARG;
    }

    nw_connection_group_start(h->group);

    uint64_t deadline = nw_shim_deadline_after(NW_SHIM_START_TIMEOUT_NS);
    pthread_mutex_lock(&h->lock);
    while (h->state == (int)nw_connection_group_state_invalid && !h->cancelled) {
        if (!nw_shim_cond_wait_until(&h->cond, &h->lock, deadline)) {
            break;
        }
    }
    int state = h->state;
    bool cancelled = h->cancelled;
    pthread_mutex_unlock(&h->lock);

    if (cancelled || state == (int)nw_connection_group_state_failed || state == (int)nw_connection_group_state_cancelled) {
        return NW_CONNECT_FAILED;
    }
    if (state == (int)nw_connection_group_state_invalid) {
        return NW_TIMEOUT;
    }
    return NW_OK;
}

void nw_shim_connection_group_cancel(void *handle) {
    nw_connection_group_handle *h = (nw_connection_group_handle *)handle;
    if (!h) {
        return;
    }
    nw_shim_group_cancel(h);
}

void nw_shim_connection_group_release(void *handle) {
    nw_connection_group_handle *h = (nw_connection_group_handle *)handle;
    if (!h) {
        return;
    }
    nw_shim_acceptor_close(&h->acceptor);
    nw_shim_group_cancel(h);
    nw_shim_group_release_owner(h);
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
    nw_path_t path = nw_shim_copy_current_path("networkframework-rs.interface-lookup");
    if (!path) {
        return NULL;
    }
    nw_interface_t interface = nw_shim_copy_matching_interface_from_path(path, name, interface_type, index);
    nw_release(path);
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
    nw_shim_waiter *waiter = nw_shim_waiter_create(2);
    if (!waiter) {
        return NULL;
    }
    nw_connection_access_establishment_report(h->conn, h->queue, ^(nw_establishment_report_t accessed_report) {
        nw_shim_waiter_complete(waiter, accessed_report ? nw_retain(accessed_report) : NULL);
        nw_shim_waiter_release(waiter);
    });
    void *report = nw_shim_waiter_wait_take(waiter, NW_SHIM_CONNECT_TIMEOUT_NS, NULL);
    nw_shim_waiter_release(waiter);
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
    nw_shim_waiter *waiter = nw_shim_waiter_create(2);
    if (!waiter) {
        return NW_INVALID_ARG;
    }
    dispatch_queue_t queue = dispatch_queue_create("networkframework-rs.data-transfer-report", DISPATCH_QUEUE_SERIAL);
    nw_data_transfer_report_collect((nw_data_transfer_report_t)report, queue, ^(nw_data_transfer_report_t collected_report) {
        (void)collected_report;
        nw_shim_waiter_complete(waiter, NULL);
        nw_shim_waiter_release(waiter);
    });
    bool done = false;
    (void)nw_shim_waiter_wait_take(waiter, NW_SHIM_CONNECT_TIMEOUT_NS, &done);
    dispatch_release(queue);
    nw_shim_waiter_release(waiter);
    return done ? NW_OK : NW_TIMEOUT;
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
    _Atomic long refs;
    nw_ethernet_channel_t channel;
    dispatch_queue_t queue;
    pthread_mutex_t lock;
    bool cancelled;
    bool cancel_requested;
    nw_shim_subscriptions subs;
} nw_ethernet_channel_handle;

static void nw_shim_ethernet_release(nw_ethernet_channel_handle *h) {
    if (atomic_fetch_sub_explicit(&h->refs, 1, memory_order_acq_rel) != 1) {
        return;
    }
    nw_shim_subscriptions_destroy(&h->subs);
    nw_release(h->channel);
    dispatch_release(h->queue);
    pthread_mutex_destroy(&h->lock);
    free(h);
}

static void nw_shim_ethernet_release_async(void *context) {
    nw_shim_ethernet_release((nw_ethernet_channel_handle *)context);
}

static void nw_shim_ethernet_on_state(nw_ethernet_channel_handle *h, nw_ethernet_channel_state_t state) {
    pthread_mutex_lock(&h->lock);
    if (h->cancelled) {
        pthread_mutex_unlock(&h->lock);
        return;
    }
    bool final_event = state == nw_ethernet_channel_state_cancelled;
    if (final_event) {
        h->cancelled = true;
    }
    pthread_mutex_unlock(&h->lock);

    nw_shim_snapshot snapshot;
    nw_shim_subscriptions_snapshot(&h->subs, NW_SHIM_EVENT_STATE, &snapshot);
    for (size_t index = 0; index < snapshot.count; index++) {
        ((EthernetChannelStateCallback)snapshot.items[index].fn)((int)state, snapshot.items[index].context);
    }
    nw_shim_snapshot_release(&snapshot);

    if (final_event) {
        dispatch_async_f(h->queue, h, nw_shim_ethernet_release_async);
    }
}

static void nw_shim_ethernet_on_receive(
    nw_ethernet_channel_handle *h,
    dispatch_data_t content,
    uint16_t vlan_tag,
    const uint8_t *local_address,
    const uint8_t *remote_address
) {
    nw_shim_snapshot snapshot;
    nw_shim_subscriptions_snapshot(&h->subs, NW_SHIM_EVENT_RECEIVE, &snapshot);
    if (snapshot.count > 0) {
        uint8_t *bytes = NULL;
        size_t length = nw_shim_copy_dispatch_data(content, &bytes);
        for (size_t index = 0; index < snapshot.count; index++) {
            ((EthernetChannelReceiveCallback)snapshot.items[index].fn)(
                bytes, length, vlan_tag, local_address, remote_address, snapshot.items[index].context);
        }
        free(bytes);
    }
    nw_shim_snapshot_release(&snapshot);
}

static nw_ethernet_channel_handle *nw_shim_create_ethernet_channel_handle(nw_ethernet_channel_t channel) {
    if (!channel) {
        return NULL;
    }
    nw_ethernet_channel_handle *h = (nw_ethernet_channel_handle *)calloc(1, sizeof(nw_ethernet_channel_handle));
    if (!h) {
        nw_release(channel);
        return NULL;
    }
    atomic_init(&h->refs, 2);
    h->channel = channel;
    h->queue = dispatch_queue_create("networkframework-rs.ethernet-channel", DISPATCH_QUEUE_SERIAL);
    pthread_mutex_init(&h->lock, NULL);
    nw_shim_subscriptions_init(&h->subs);

    nw_ethernet_channel_set_queue(channel, h->queue);
    nw_ethernet_channel_set_state_changed_handler(channel, ^(nw_ethernet_channel_state_t state, nw_error_t error) {
        (void)error;
        nw_shim_ethernet_on_state(h, state);
    });
    nw_ethernet_channel_set_receive_handler(channel,
        ^(dispatch_data_t content, uint16_t vlan_tag, nw_ethernet_address_t local_address, nw_ethernet_address_t remote_address) {
            nw_shim_ethernet_on_receive(h, content, vlan_tag, local_address, remote_address);
        });
    return h;
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

uint64_t nw_shim_ethernet_channel_subscribe_state(
    void *handle,
    EthernetChannelStateCallback callback,
    void *context,
    NwShimContextCallback retain,
    NwShimContextCallback release
) {
    nw_ethernet_channel_handle *h = (nw_ethernet_channel_handle *)handle;
    nw_shim_callback entry = nw_shim_make_callback((nw_shim_fn)callback, context, retain, release);
    if (!h) {
        nw_shim_callback_release(&entry);
        return 0;
    }
    return nw_shim_subscriptions_add(&h->subs, NW_SHIM_EVENT_STATE, entry);
}

uint64_t nw_shim_ethernet_channel_subscribe_receive(
    void *handle,
    EthernetChannelReceiveCallback callback,
    void *context,
    NwShimContextCallback retain,
    NwShimContextCallback release
) {
    nw_ethernet_channel_handle *h = (nw_ethernet_channel_handle *)handle;
    nw_shim_callback entry = nw_shim_make_callback((nw_shim_fn)callback, context, retain, release);
    if (!h) {
        nw_shim_callback_release(&entry);
        return 0;
    }
    return nw_shim_subscriptions_add(&h->subs, NW_SHIM_EVENT_RECEIVE, entry);
}

void nw_shim_ethernet_channel_unsubscribe(void *handle, uint64_t token) {
    nw_ethernet_channel_handle *h = (nw_ethernet_channel_handle *)handle;
    if (!h) return;
    nw_shim_subscriptions_remove(&h->subs, token);
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
    pthread_mutex_lock(&h->lock);
    bool should_cancel = !h->cancel_requested && !h->cancelled;
    h->cancel_requested = true;
    pthread_mutex_unlock(&h->lock);
    if (should_cancel) {
        nw_ethernet_channel_cancel(h->channel);
    }
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
    nw_shim_ethernet_channel_cancel(h);
    nw_shim_ethernet_release(h);
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

void nw_shim_connection_release_without_cancel(void *handle) {
    nw_conn_handle *h = (nw_conn_handle *)handle;
    if (!h) {
        return;
    }
    nw_shim_conn_release(h);
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

static uint64_t nw_shim_connection_subscribe(void *handle, int kind, nw_shim_callback entry) {
    nw_conn_handle *h = (nw_conn_handle *)handle;
    if (!h) {
        nw_shim_callback_release(&entry);
        return 0;
    }
    return nw_shim_subscriptions_add(&h->subs, kind, entry);
}

uint64_t nw_shim_connection_subscribe_state(
    void *handle,
    ConnectionStateCallback callback,
    void *context,
    NwShimContextCallback retain,
    NwShimContextCallback release
) {
    return nw_shim_connection_subscribe(
        handle, NW_SHIM_EVENT_STATE, nw_shim_make_callback((nw_shim_fn)callback, context, retain, release));
}

uint64_t nw_shim_connection_subscribe_viability(
    void *handle,
    ConnectionBooleanCallback callback,
    void *context,
    NwShimContextCallback retain,
    NwShimContextCallback release
) {
    return nw_shim_connection_subscribe(
        handle, NW_SHIM_EVENT_VIABILITY, nw_shim_make_callback((nw_shim_fn)callback, context, retain, release));
}

uint64_t nw_shim_connection_subscribe_better_path(
    void *handle,
    ConnectionBooleanCallback callback,
    void *context,
    NwShimContextCallback retain,
    NwShimContextCallback release
) {
    return nw_shim_connection_subscribe(
        handle, NW_SHIM_EVENT_BETTER_PATH, nw_shim_make_callback((nw_shim_fn)callback, context, retain, release));
}

uint64_t nw_shim_connection_subscribe_path(
    void *handle,
    ConnectionPathCallback callback,
    void *context,
    NwShimContextCallback retain,
    NwShimContextCallback release
) {
    return nw_shim_connection_subscribe(
        handle, NW_SHIM_EVENT_PATH, nw_shim_make_callback((nw_shim_fn)callback, context, retain, release));
}

void nw_shim_connection_unsubscribe(void *handle, uint64_t token) {
    nw_conn_handle *h = (nw_conn_handle *)handle;
    if (!h) {
        return;
    }
    nw_shim_subscriptions_remove(&h->subs, token);
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
    return nw_shim_conn_start_and_wait(
        connection, "networkframework-rs.connection-group.message", NW_SHIM_CONNECT_TIMEOUT_NS, out_status);
}

void *nw_shim_connection_group_extract_connection(void *handle, void *endpoint, void *protocol_options, int *out_status) {
    nw_connection_group_handle *h = (nw_connection_group_handle *)handle;
    if (!h) {
        if (out_status) *out_status = NW_INVALID_ARG;
        return NULL;
    }
    nw_connection_t connection = nw_connection_group_extract_connection(
        h->group,
        (nw_endpoint_t)endpoint,
        (nw_protocol_options_t)protocol_options);
    if (!connection) {
        if (out_status) *out_status = NW_CONNECT_FAILED;
        return NULL;
    }
    return nw_shim_conn_start_and_wait(
        connection, "networkframework-rs.connection-group.extract", NW_SHIM_CONNECT_TIMEOUT_NS, out_status);
}

int nw_shim_connection_group_reinsert_extracted_connection(void *handle, void *connection_handle) {
    nw_connection_group_handle *h = (nw_connection_group_handle *)handle;
    nw_conn_handle *connection = (nw_conn_handle *)connection_handle;
    if (!h || !connection || !connection->conn) {
        return NW_INVALID_ARG;
    }
    nw_shim_conn_clear_handlers(connection);
    return nw_connection_group_reinsert_extracted_connection(h->group, connection->conn) ? NW_OK : NW_INVALID_ARG;
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
    return nw_shim_listener_start(listener, "networkframework-rs.listener.direct", NULL, out_status);
}

void *nw_shim_listener_create_with_connection(void *connection_handle, void *parameters, int *out_status) {
    nw_conn_handle *connection = (nw_conn_handle *)connection_handle;
    if (!connection || !connection->conn || !parameters) {
        if (out_status) *out_status = NW_INVALID_ARG;
        return NULL;
    }
    nw_listener_t listener = nw_listener_create_with_connection(connection->conn, (nw_parameters_t)parameters);
    return nw_shim_listener_start(listener, "networkframework-rs.listener.connection", NULL, out_status);
}

void *nw_shim_listener_create_with_launchd_key(void *parameters, const char *launchd_key, int *out_status) {
    if (!parameters || !launchd_key) {
        if (out_status) *out_status = NW_INVALID_ARG;
        return NULL;
    }
    nw_listener_t listener = nw_listener_create_with_launchd_key((nw_parameters_t)parameters, launchd_key);
    return nw_shim_listener_start(listener, "networkframework-rs.listener.launchd", NULL, out_status);
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

static void nw_shim_listener_on_advertised(nw_listener_handle *h, nw_endpoint_t endpoint, bool added) {
    nw_shim_snapshot snapshot;
    nw_shim_subscriptions_snapshot(&h->subs, NW_SHIM_EVENT_ADVERTISED_ENDPOINT, &snapshot);
    for (size_t index = 0; index < snapshot.count; index++) {
        ((ListenerAdvertisedEndpointChangedCallback)snapshot.items[index].fn)(
            endpoint ? nw_retain(endpoint) : NULL,
            added ? 1 : 0,
            snapshot.items[index].context);
    }
    nw_shim_snapshot_release(&snapshot);
}

static void nw_shim_listener_on_group(nw_listener_handle *h, nw_connection_group_t group) {
    if (!group) {
        return;
    }
    nw_connection_group_handle *wrapped = nw_shim_group_create_handle(nw_retain(group), "networkframework-rs.listener.group");
    if (!wrapped) {
        return;
    }
    nw_shim_snapshot snapshot;
    nw_shim_subscriptions_snapshot(&h->subs, NW_SHIM_EVENT_NEW_GROUP, &snapshot);
    if (snapshot.count > 0) {
        ((ListenerNewConnectionGroupCallback)snapshot.items[0].fn)(wrapped, snapshot.items[0].context);
    } else {
        nw_shim_connection_group_release(wrapped);
    }
    nw_shim_snapshot_release(&snapshot);
}

uint64_t nw_shim_listener_subscribe_advertised_endpoint(
    void *handle,
    ListenerAdvertisedEndpointChangedCallback callback,
    void *context,
    NwShimContextCallback retain,
    NwShimContextCallback release
) {
    nw_listener_handle *h = (nw_listener_handle *)handle;
    nw_shim_callback entry = nw_shim_make_callback((nw_shim_fn)callback, context, retain, release);
    if (!h) {
        nw_shim_callback_release(&entry);
        return 0;
    }
    uint64_t token = nw_shim_subscriptions_add(&h->subs, NW_SHIM_EVENT_ADVERTISED_ENDPOINT, entry);
    if (token) {
        pthread_mutex_lock(&h->lock);
        bool install = !h->advertised_installed;
        h->advertised_installed = true;
        pthread_mutex_unlock(&h->lock);
        if (install) {
            nw_listener_set_advertised_endpoint_changed_handler(h->listener, ^(nw_endpoint_t endpoint, bool added) {
                nw_shim_listener_on_advertised(h, endpoint, added);
            });
        }
    }
    return token;
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


void *nw_shim_identity_create(void *sec_identity_ref) {
    if (!sec_identity_ref) {
        return NULL;
    }
    return sec_identity_create((SecIdentityRef)sec_identity_ref);
}

void nw_shim_sec_options_set_local_identity(void *options, void *identity) {
    if (!options || !identity) {
        return;
    }
    sec_protocol_options_set_local_identity((sec_protocol_options_t)options, (sec_identity_t)identity);
}

void nw_shim_sec_options_set_min_tls_version(void *options, uint16_t version) {
    if (!options) {
        return;
    }
    sec_protocol_options_set_min_tls_protocol_version((sec_protocol_options_t)options, (tls_protocol_version_t)version);
}

void nw_shim_sec_options_set_max_tls_version(void *options, uint16_t version) {
    if (!options) {
        return;
    }
    sec_protocol_options_set_max_tls_protocol_version((sec_protocol_options_t)options, (tls_protocol_version_t)version);
}

void nw_shim_sec_options_add_application_protocol(void *options, const char *application_protocol) {
    if (!options || !application_protocol) {
        return;
    }
    sec_protocol_options_add_tls_application_protocol((sec_protocol_options_t)options, application_protocol);
}

void nw_shim_sec_options_set_server_name(void *options, const char *server_name) {
    if (!options || !server_name) {
        return;
    }
    sec_protocol_options_set_tls_server_name((sec_protocol_options_t)options, server_name);
}

void nw_shim_sec_options_set_peer_authentication_required(void *options, int required) {
    if (!options) {
        return;
    }
    sec_protocol_options_set_peer_authentication_required((sec_protocol_options_t)options, required != 0);
}

uint16_t nw_shim_sec_metadata_get_negotiated_tls_version(void *metadata) {
    if (!metadata) {
        return 0;
    }
    return (uint16_t)sec_protocol_metadata_get_negotiated_tls_protocol_version((sec_protocol_metadata_t)metadata);
}

char *nw_shim_sec_metadata_copy_negotiated_protocol(void *metadata) {
    if (!metadata) {
        return NULL;
    }
    if (__builtin_available(macOS 15.5, *)) {
        const char *value = sec_protocol_metadata_copy_negotiated_protocol((sec_protocol_metadata_t)metadata);
        return (char *)value;
    }
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wdeprecated-declarations"
    const char *value = sec_protocol_metadata_get_negotiated_protocol((sec_protocol_metadata_t)metadata);
#pragma clang diagnostic pop
    return value ? strdup(value) : NULL;
}

void nw_shim_sha256(const uint8_t *data, size_t length, uint8_t *out_digest) {
    if (!out_digest) {
        return;
    }
    CC_SHA256_CTX context;
    CC_SHA256_Init(&context);
    while (data && length > 0) {
        CC_LONG chunk = length > UINT32_MAX ? UINT32_MAX : (CC_LONG)length;
        CC_SHA256_Update(&context, data, chunk);
        data += chunk;
        length -= chunk;
    }
    CC_SHA256_Final(out_digest, &context);
}
