// C helpers behind std.network.
//
// Socket addresses cross the boundary as a `kestrel_address` (below), never
// as a `sockaddr`, so the per-target layouts of `sockaddr_in`,
// `sockaddr_in6`, `sockaddr_un` and `addrinfo` stay in C. Every socket is
// created close-on-exec, and no send can raise SIGPIPE: Linux passes
// MSG_NOSIGNAL, macOS sets SO_NOSIGPIPE on the socket.

#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <netdb.h>
#include <netinet/in.h>
#include <netinet/tcp.h>
#include <poll.h>
#include <stdint.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/time.h>
#include <sys/un.h>
#include <unistd.h>

#ifdef MSG_NOSIGNAL
#define KESTREL_SEND_FLAGS MSG_NOSIGNAL
#else
#define KESTREL_SEND_FLAGS 0
#endif

// The address as std.network reads it (address.ks `readAddress` /
// `writeAddress`): family 4 or 6, port and scope id in host order, then the
// address bytes in network order (4 used for IPv4, 16 for IPv6).
typedef struct {
    int64_t family;
    int64_t port;
    int64_t scope;
    uint8_t ip[16];
} kestrel_address;

// Socket families and kinds, as network.ks passes them.
enum { KESTREL_UNIX = 1, KESTREL_V4 = 4, KESTREL_V6 = 6 };
enum { KESTREL_STREAM = 1, KESTREL_DATAGRAM = 2 };

static socklen_t to_sockaddr(const kestrel_address *in, struct sockaddr_storage *out) {
    memset(out, 0, sizeof *out);
    if (in->family == KESTREL_V4) {
        struct sockaddr_in *s = (struct sockaddr_in *)out;
        s->sin_family = AF_INET;
        s->sin_port = htons((uint16_t)in->port);
        memcpy(&s->sin_addr, in->ip, 4);
        return sizeof *s;
    }
    struct sockaddr_in6 *s = (struct sockaddr_in6 *)out;
    s->sin6_family = AF_INET6;
    s->sin6_port = htons((uint16_t)in->port);
    s->sin6_scope_id = (uint32_t)in->scope;
    memcpy(&s->sin6_addr, in->ip, 16);
    return sizeof *s;
}

// Returns 0, or -1 with errno = EAFNOSUPPORT for a non-IP address.
static int from_sockaddr(const struct sockaddr *in, kestrel_address *out) {
    memset(out, 0, sizeof *out);
    if (in->sa_family == AF_INET) {
        const struct sockaddr_in *s = (const struct sockaddr_in *)in;
        out->family = KESTREL_V4;
        out->port = ntohs(s->sin_port);
        memcpy(out->ip, &s->sin_addr, 4);
        return 0;
    }
    if (in->sa_family == AF_INET6) {
        const struct sockaddr_in6 *s = (const struct sockaddr_in6 *)in;
        out->family = KESTREL_V6;
        out->port = ntohs(s->sin6_port);
        out->scope = s->sin6_scope_id;
        memcpy(out->ip, &s->sin6_addr, 16);
        return 0;
    }
    errno = EAFNOSUPPORT;
    return -1;
}

__attribute__((unused)) static int set_cloexec(int fd) {
    int flags = fcntl(fd, F_GETFD);
    return flags < 0 ? -1 : fcntl(fd, F_SETFD, flags | FD_CLOEXEC);
}

// Per-socket setup every socket gets: close-on-exec (where the type flag
// isn't available) and no SIGPIPE on macOS.
static int prepare(int fd) {
    (void)fd;
#ifndef SOCK_CLOEXEC
    if (set_cloexec(fd) < 0) return -1;
#endif
#ifdef SO_NOSIGPIPE
    int one = 1;
    if (setsockopt(fd, SOL_SOCKET, SO_NOSIGPIPE, &one, sizeof one) < 0) return -1;
#endif
    return 0;
}

// socket(2). Returns the fd, or -1 with errno set.
int32_t kestrel_socket(int32_t family, int32_t kind) {
    int domain = family == KESTREL_UNIX ? AF_UNIX : family == KESTREL_V4 ? AF_INET : AF_INET6;
    int type = kind == KESTREL_STREAM ? SOCK_STREAM : SOCK_DGRAM;
#ifdef SOCK_CLOEXEC
    type |= SOCK_CLOEXEC;
#endif
    int fd = socket(domain, type, 0);
    if (fd < 0) return -1;
    if (prepare(fd) < 0) {
        int saved = errno;
        close(fd);
        errno = saved;
        return -1;
    }
    return fd;
}

// socketpair(2) of Unix stream sockets into fds[0] and fds[1].
int32_t kestrel_socket_pair(int32_t *fds) {
    int type = SOCK_STREAM;
#ifdef SOCK_CLOEXEC
    type |= SOCK_CLOEXEC;
#endif
    int pair[2];
    if (socketpair(AF_UNIX, type, 0, pair) < 0) return -1;
    if (prepare(pair[0]) < 0 || prepare(pair[1]) < 0) {
        int saved = errno;
        close(pair[0]);
        close(pair[1]);
        errno = saved;
        return -1;
    }
    fds[0] = pair[0];
    fds[1] = pair[1];
    return 0;
}

static int wait_connected(int fd, int64_t timeout_ms) {
    struct pollfd p = { .fd = fd, .events = POLLOUT };
    int r;
    do {
        r = poll(&p, 1, timeout_ms > INT32_MAX ? INT32_MAX : (int)timeout_ms);
    } while (r < 0 && errno == EINTR);
    if (r < 0) return -1;
    if (r == 0) {
        errno = ETIMEDOUT;
        return -1;
    }
    int err = 0;
    socklen_t len = sizeof err;
    if (getsockopt(fd, SOL_SOCKET, SO_ERROR, &err, &len) < 0) return -1;
    if (err != 0) {
        errno = err;
        return -1;
    }
    return 0;
}

// connect(2) with an optional timeout in milliseconds (negative = block).
// A timed connect runs nonblocking and waits with poll(2), then restores
// blocking mode.
static int32_t connect_with_timeout(int fd, const struct sockaddr *sa, socklen_t len, int64_t timeout_ms) {
    if (timeout_ms < 0) {
        int r = connect(fd, sa, len);
        // An interrupted connect keeps going in the background.
        if (r < 0 && errno == EINTR) return wait_connected(fd, -1);
        return r;
    }
    int flags = fcntl(fd, F_GETFL);
    if (flags < 0 || fcntl(fd, F_SETFL, flags | O_NONBLOCK) < 0) return -1;
    int r = connect(fd, sa, len);
    if (r < 0 && (errno == EINPROGRESS || errno == EINTR)) r = wait_connected(fd, timeout_ms);
    int saved = errno;
    fcntl(fd, F_SETFL, flags);
    errno = saved;
    return r;
}

int32_t kestrel_socket_connect(int32_t fd, const kestrel_address *address, int64_t timeout_ms) {
    struct sockaddr_storage sa;
    socklen_t len = to_sockaddr(address, &sa);
    return connect_with_timeout(fd, (struct sockaddr *)&sa, len, timeout_ms);
}

int32_t kestrel_socket_bind(int32_t fd, const kestrel_address *address) {
    struct sockaddr_storage sa;
    socklen_t len = to_sockaddr(address, &sa);
    return bind(fd, (struct sockaddr *)&sa, len);
}

int32_t kestrel_socket_listen(int32_t fd, int32_t backlog) {
    return listen(fd, backlog);
}

// accept(2), retrying on EINTR. Writes the peer to `peer` when it's an IP
// address (a Unix peer leaves `peer` zeroed). The new fd is close-on-exec.
int32_t kestrel_socket_accept(int32_t fd, kestrel_address *peer) {
    struct sockaddr_storage sa;
    socklen_t len;
    int client;
    do {
        len = sizeof sa;
#if defined(SOCK_CLOEXEC) && defined(__linux__)
        client = accept4(fd, (struct sockaddr *)&sa, &len, SOCK_CLOEXEC);
#else
        client = accept(fd, (struct sockaddr *)&sa, &len);
#endif
    } while (client < 0 && errno == EINTR);
    if (client < 0) return -1;
#if !(defined(SOCK_CLOEXEC) && defined(__linux__))
    set_cloexec(client);
#endif
#ifdef SO_NOSIGPIPE
    int one = 1;
    setsockopt(client, SOL_SOCKET, SO_NOSIGPIPE, &one, sizeof one);
#endif
    if (from_sockaddr((struct sockaddr *)&sa, peer) < 0) memset(peer, 0, sizeof *peer);
    return client;
}

// The local (`peer` = 0) or remote (`peer` = 1) address of `fd`.
int32_t kestrel_socket_address(int32_t fd, int32_t peer, kestrel_address *out) {
    struct sockaddr_storage sa;
    socklen_t len = sizeof sa;
    int r = peer ? getpeername(fd, (struct sockaddr *)&sa, &len)
                 : getsockname(fd, (struct sockaddr *)&sa, &len);
    if (r < 0) return -1;
    return from_sockaddr((struct sockaddr *)&sa, out);
}

int64_t kestrel_socket_send(int32_t fd, const void *buf, int64_t len) {
    return send(fd, buf, (size_t)len, KESTREL_SEND_FLAGS);
}

int64_t kestrel_socket_receive(int32_t fd, void *buf, int64_t len) {
    return recv(fd, buf, (size_t)len, 0);
}

int64_t kestrel_socket_send_to(int32_t fd, const void *buf, int64_t len, const kestrel_address *to) {
    struct sockaddr_storage sa;
    socklen_t salen = to_sockaddr(to, &sa);
    return sendto(fd, buf, (size_t)len, KESTREL_SEND_FLAGS, (struct sockaddr *)&sa, salen);
}

int64_t kestrel_socket_receive_from(int32_t fd, void *buf, int64_t len, kestrel_address *from) {
    struct sockaddr_storage sa;
    socklen_t salen = sizeof sa;
    ssize_t n = recvfrom(fd, buf, (size_t)len, 0, (struct sockaddr *)&sa, &salen);
    if (n < 0) return -1;
    if (from_sockaddr((struct sockaddr *)&sa, from) < 0) memset(from, 0, sizeof *from);
    return n;
}

// shutdown(2): how is 0 read, 1 write, 2 both.
int32_t kestrel_socket_shutdown(int32_t fd, int32_t how) {
    return shutdown(fd, how == 0 ? SHUT_RD : how == 1 ? SHUT_WR : SHUT_RDWR);
}

// Boolean options, numbered as network.ks passes them.
enum {
    KESTREL_NO_DELAY = 1,
    KESTREL_KEEP_ALIVE = 2,
    KESTREL_BROADCAST = 3,
    KESTREL_REUSE_ADDRESS = 4,
    KESTREL_V6_ONLY = 5,
};

int32_t kestrel_socket_set_flag(int32_t fd, int32_t option, int32_t on) {
    int value = on ? 1 : 0;
    switch (option) {
    case KESTREL_NO_DELAY: return setsockopt(fd, IPPROTO_TCP, TCP_NODELAY, &value, sizeof value);
    case KESTREL_KEEP_ALIVE: return setsockopt(fd, SOL_SOCKET, SO_KEEPALIVE, &value, sizeof value);
    case KESTREL_BROADCAST: return setsockopt(fd, SOL_SOCKET, SO_BROADCAST, &value, sizeof value);
    case KESTREL_REUSE_ADDRESS: return setsockopt(fd, SOL_SOCKET, SO_REUSEADDR, &value, sizeof value);
    case KESTREL_V6_ONLY: return setsockopt(fd, IPPROTO_IPV6, IPV6_V6ONLY, &value, sizeof value);
    default: errno = EINVAL; return -1;
    }
}

// SO_RCVTIMEO (`write` = 0) or SO_SNDTIMEO (`write` = 1). A negative
// timeout clears it, so calls block.
int32_t kestrel_socket_set_timeout(int32_t fd, int32_t write, int64_t timeout_ms) {
    struct timeval tv = { 0, 0 };
    if (timeout_ms > 0) {
        tv.tv_sec = timeout_ms / 1000;
        tv.tv_usec = (timeout_ms % 1000) * 1000;
    } else if (timeout_ms == 0) {
        // A zero timeval means "never time out", so round up to 1 µs.
        tv.tv_usec = 1;
    }
    return setsockopt(fd, SOL_SOCKET, write ? SO_SNDTIMEO : SO_RCVTIMEO, &tv, sizeof tv);
}

int32_t kestrel_socket_set_nonblocking(int32_t fd, int32_t on) {
    int flags = fcntl(fd, F_GETFL);
    if (flags < 0) return -1;
    return fcntl(fd, F_SETFL, on ? (flags | O_NONBLOCK) : (flags & ~O_NONBLOCK));
}

// dup(2) with close-on-exec.
int32_t kestrel_socket_duplicate(int32_t fd) {
    return fcntl(fd, F_DUPFD_CLOEXEC, 0);
}

// IoErrorKind numbers for resolver failures (see streams_shims.c).
enum { KIND_OTHER = 0, KIND_HOST_NOT_FOUND = 24, KIND_OUT_OF_MEMORY = 11 };

// getaddrinfo(3) for TCP addresses of `host`, written to `out` (at most
// `capacity`), IPv6 and IPv4 interleaved starting with the family of the
// first answer (RFC 8305 §4). Returns the count, or -1 with `*error_kind`
// set to an IoErrorKind number, or to -1 when errno holds the error.
int64_t kestrel_lookup(const char *host, int64_t port, kestrel_address *out, int64_t capacity, int32_t *error_kind) {
    struct addrinfo hints;
    memset(&hints, 0, sizeof hints);
    hints.ai_family = AF_UNSPEC;
    hints.ai_socktype = SOCK_STREAM;
    hints.ai_flags = AI_ADDRCONFIG;
    struct addrinfo *list = NULL;
    int r = getaddrinfo(host, NULL, &hints, &list);
    if (r != 0) {
        switch (r) {
        case EAI_SYSTEM: *error_kind = -1; break;
        case EAI_MEMORY: *error_kind = KIND_OUT_OF_MEMORY; break;
        case EAI_NONAME:
#if defined(EAI_NODATA) && EAI_NODATA != EAI_NONAME
        case EAI_NODATA:
#endif
        case EAI_AGAIN:
        case EAI_FAIL:
            *error_kind = KIND_HOST_NOT_FOUND; break;
        default: *error_kind = KIND_OTHER; break;
        }
        return -1;
    }
    // Split by family, keeping each family's order, then interleave.
    kestrel_address v6[64], v4[64];
    int64_t n6 = 0, n4 = 0;
    int first_v6 = -1;
    for (struct addrinfo *a = list; a; a = a->ai_next) {
        kestrel_address k;
        if (from_sockaddr(a->ai_addr, &k) < 0) continue;
        k.port = port;
        if (first_v6 < 0) first_v6 = k.family == KESTREL_V6;
        if (k.family == KESTREL_V6 && n6 < 64) v6[n6++] = k;
        if (k.family == KESTREL_V4 && n4 < 64) v4[n4++] = k;
    }
    freeaddrinfo(list);
    kestrel_address *a = first_v6 ? v6 : v4, *b = first_v6 ? v4 : v6;
    int64_t na = first_v6 ? n6 : n4, nb = first_v6 ? n4 : n6;
    int64_t count = 0;
    for (int64_t i = 0; (i < na || i < nb) && count < capacity; i++) {
        if (i < na && count < capacity) out[count++] = a[i];
        if (i < nb && count < capacity) out[count++] = b[i];
    }
    if (count == 0) {
        *error_kind = KIND_HOST_NOT_FOUND;
        return -1;
    }
    return count;
}

static int unix_address(const char *path, struct sockaddr_un *sa) {
    memset(sa, 0, sizeof *sa);
    sa->sun_family = AF_UNIX;
    if (strlen(path) >= sizeof sa->sun_path) {
        errno = ENAMETOOLONG;
        return -1;
    }
    strcpy(sa->sun_path, path);
    return 0;
}

int32_t kestrel_unix_connect(int32_t fd, const char *path) {
    struct sockaddr_un sa;
    if (unix_address(path, &sa) < 0) return -1;
    return connect_with_timeout(fd, (struct sockaddr *)&sa, sizeof sa, -1);
}

int32_t kestrel_unix_bind(int32_t fd, const char *path) {
    struct sockaddr_un sa;
    if (unix_address(path, &sa) < 0) return -1;
    return bind(fd, (struct sockaddr *)&sa, sizeof sa);
}
