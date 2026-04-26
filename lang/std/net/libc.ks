// POSIX socket syscall bindings
//
// This module provides raw bindings to POSIX socket functions via @extern(.C).
// Constants use macOS values.

module std.net.libc

import std.num.(Int64, Int32, UInt8, UInt16)
import std.memory.(Pointer)

// ============================================================================
// SOCKET CONSTANTS (macOS)
// ============================================================================

/// `AF_INET` — the IPv4 address family.
///
/// Pass to `socket()` as the `domain` argument when opening an IPv4
/// socket. The numeric value is `2` on every POSIX platform we
/// target.
public func AF_INET() -> Int32 { 2 }

/// `SOCK_STREAM` — connection-oriented byte stream (TCP).
///
/// Pass to `socket()` as the `type_` argument. Pair with
/// `IPPROTO_TCP` for explicit TCP, or `0` to let the kernel pick the
/// default protocol for the family/type pair.
public func SOCK_STREAM() -> Int32 { 1 }

/// `IPPROTO_TCP` — the TCP transport protocol number (`6`).
///
/// Pass to `socket()` as the `proto` argument when opening a TCP
/// socket.
public func IPPROTO_TCP() -> Int32 { 6 }

// errno access
@platform(.darwin)
@extern(.C, mangleName: "__error")
func __errno_ptr() -> lang.ptr[lang.i32]

@platform(.linux)
@extern(.C, mangleName: "__errno_location")
func __errno_ptr() -> lang.ptr[lang.i32]

// Socket constants (platform-specific values)

/// `SOL_SOCKET` — the socket-level option layer.
///
/// Pass as the `level` argument of `setsockopt()` to address options
/// that apply to the socket itself (rather than a specific protocol).
/// Numeric value differs across platforms — `0xFFFF` on darwin, `1`
/// on linux — so always go through this accessor instead of
/// hard-coding.
@platform(.darwin)
public func SOL_SOCKET() -> Int32 { 0xFFFF }

/// Linux-specific definition of `SOL_SOCKET` (`1`).
@platform(.linux)
public func SOL_SOCKET() -> Int32 { 1 }

/// `SO_REUSEADDR` — allow rebinding to an address still in `TIME_WAIT`.
///
/// Pass as the `optname` argument of `setsockopt()` with `SOL_SOCKET`
/// to let a server bind to a recently-used port without waiting for
/// the kernel's grace period. Numeric value differs across
/// platforms.
@platform(.darwin)
public func SO_REUSEADDR() -> Int32 { 0x0004 }

/// Linux-specific definition of `SO_REUSEADDR` (`2`).
@platform(.linux)
public func SO_REUSEADDR() -> Int32 { 2 }

/// `INADDR_ANY` — wildcard IPv4 address (`0.0.0.0`).
///
/// Use as the `sin_addr` field of a `sockaddr_in` to bind a server
/// socket to every local interface.
public func INADDR_ANY() -> Int32 { 0 }

/// Size in bytes of `struct sockaddr_in` (`16`).
///
/// Pass as the `addrlen` argument to `bind()` / `connect()` /
/// `accept()` when working with IPv4 addresses.
public func SOCKADDR_IN_SIZE() -> Int32 { 16 }

// ============================================================================
// RAW LIBC BINDINGS
// ============================================================================

@extern(.C, mangleName: "socket")
func libc_socket(domain: lang.i32, type_: lang.i32, proto: lang.i32) -> lang.i32

@extern(.C, mangleName: "bind")
func libc_bind(sockfd: lang.i32, addr: lang.ptr[lang.i8], addrlen: lang.i32) -> lang.i32

@extern(.C, mangleName: "listen")
func libc_listen(sockfd: lang.i32, backlog: lang.i32) -> lang.i32

@extern(.C, mangleName: "accept")
func libc_accept(sockfd: lang.i32, addr: lang.ptr[lang.i8], addrlen: lang.ptr[lang.i32]) -> lang.i32

@extern(.C, mangleName: "recv")
func libc_recv(sockfd: lang.i32, buf: lang.ptr[lang.i8], len: lang.i64, flags: lang.i32) -> lang.i64

@extern(.C, mangleName: "send")
func libc_send(sockfd: lang.i32, buf: lang.ptr[lang.i8], len: lang.i64, flags: lang.i32) -> lang.i64

@extern(.C, mangleName: "close")
func libc_close(fd: lang.i32) -> lang.i32

@extern(.C, mangleName: "setsockopt")
func libc_setsockopt(sockfd: lang.i32, level: lang.i32, optname: lang.i32, optval: lang.ptr[lang.i8], optlen: lang.i32) -> lang.i32

@extern(.C, mangleName: "htons")
func libc_htons(hostshort: lang.i16) -> lang.i16

@extern(.C, mangleName: "connect")
func libc_connect(sockfd: lang.i32, addr: lang.ptr[lang.i8], addrlen: lang.i32) -> lang.i32

@extern(.C, mangleName: "getaddrinfo")
func libc_getaddrinfo(node: lang.ptr[lang.i8], service: lang.ptr[lang.i8], hints: lang.ptr[lang.i8], res: lang.ptr[lang.ptr[lang.i8]]) -> lang.i32

@extern(.C, mangleName: "freeaddrinfo")
func libc_freeaddrinfo(res: lang.ptr[lang.i8])


// ============================================================================
// PUBLIC WRAPPERS
// ============================================================================

/// Wraps `socket(2)` — creates a new socket fd.
///
/// Returns the new file descriptor on success, or `-1` on error
/// (call `errno()` for the cause). The caller owns the fd and is
/// responsible for closing it via `close`.
///
/// # Examples
///
/// ```
/// let fd = socket(domain: AF_INET(), type_: SOCK_STREAM(), proto: IPPROTO_TCP());
/// if fd < 0 { /* errno() */ }
/// ```
public func socket(domain: Int32, type_: Int32, proto: Int32) -> Int32 {
    Int32(raw: libc_socket(domain.raw, type_.raw, proto.raw))
}

/// Wraps `bind(2)` — assigns a local address to `sockfd`.
///
/// `addr` points at a packed `sockaddr_in` (or other family-
/// appropriate layout) of length `addrlen`. Returns `0` on success,
/// `-1` on error.
public func bind(sockfd: Int32, addr: Pointer[UInt8], addrlen: Int32) -> Int32 {
    Int32(raw: libc_bind(sockfd.raw, lang.cast_ptr[_, lang.i8](addr.raw), addrlen.raw))
}

/// Wraps `listen(2)` — marks `sockfd` as accepting incoming connections.
///
/// `backlog` is the maximum length of the kernel's pending-
/// connection queue; once full, further connect attempts are
/// refused. Returns `0` on success, `-1` on error.
public func listen(sockfd: Int32, backlog: Int32) -> Int32 {
    Int32(raw: libc_listen(sockfd.raw, backlog.raw))
}

/// Wraps `accept(2)` — pulls the next connection off `sockfd`'s queue.
///
/// Blocks until a connection arrives. Returns the new connected fd
/// on success, `-1` on error. Pass null pointers for `addr` and
/// `addrlen` if you don't need the client's address.
public func accept(sockfd: Int32, addr: Pointer[UInt8], addrlen: Pointer[Int32]) -> Int32 {
    Int32(raw: libc_accept(sockfd.raw, lang.cast_ptr[_, lang.i8](addr.raw), lang.cast_ptr[_, lang.i32](addrlen.raw)))
}

/// Wraps `recv(2)` — reads up to `len` bytes from `sockfd` into `buf`.
///
/// Returns the byte count on success (which may be less than `len`),
/// `0` on orderly shutdown by the peer, or `-1` on error. `flags`
/// is a bitmask of `MSG_*` modifiers (`0` for the default).
public func recv(sockfd: Int32, buf: Pointer[UInt8], len: Int64, flags: Int32) -> Int64 {
    Int64(raw: libc_recv(sockfd.raw, lang.cast_ptr[_, lang.i8](buf.raw), len.raw, flags.raw))
}

/// Wraps `send(2)` — writes up to `len` bytes from `buf` to `sockfd`.
///
/// May write fewer bytes than requested under back-pressure; the
/// caller must loop until the buffer is drained. Returns the byte
/// count on success or `-1` on error.
public func send(sockfd: Int32, buf: Pointer[UInt8], len: Int64, flags: Int32) -> Int64 {
    Int64(raw: libc_send(sockfd.raw, lang.cast_ptr[_, lang.i8](buf.raw), len.raw, flags.raw))
}

/// Wraps `close(2)` — releases a file descriptor.
///
/// Safe to call on any fd (sockets, files, pipes). Returns `0` on
/// success, `-1` on error. After `close`, `fd` becomes available for
/// reuse by the kernel — do not use the value again.
public func close(fd: Int32) -> Int32 {
    Int32(raw: libc_close(fd.raw))
}

/// Wraps `setsockopt(2)` — configures one option on `sockfd`.
///
/// `level` selects the option layer (e.g. `SOL_SOCKET`); `optname`
/// is the per-layer option code (e.g. `SO_REUSEADDR`); `optval` /
/// `optlen` describe the value. Returns `0` on success, `-1` on
/// error.
public func setsockopt(sockfd: Int32, level: Int32, optname: Int32, optval: Pointer[UInt8], optlen: Int32) -> Int32 {
    Int32(raw: libc_setsockopt(sockfd.raw, level.raw, optname.raw, lang.cast_ptr[_, lang.i8](optval.raw), optlen.raw))
}

/// Wraps `htons(3)` — host-to-network byte order for 16-bit values.
///
/// On little-endian hosts this swaps the byte order; on big-endian
/// hosts it is the identity. Use to convert a port number before
/// writing it into a `sockaddr_in.sin_port` field.
public func htons(hostshort: UInt16) -> UInt16 {
    UInt16(raw: libc_htons(hostshort.raw))
}

/// Wraps `connect(2)` — initiates a connection on `sockfd` to the address at `addr`.
///
/// Blocks until the handshake completes (for connection-oriented
/// sockets). Returns `0` on success, `-1` on error.
public func connect(sockfd: Int32, addr: Pointer[UInt8], addrlen: Int32) -> Int32 {
    Int32(raw: libc_connect(sockfd.raw, lang.cast_ptr[_, lang.i8](addr.raw), addrlen.raw))
}

/// Wraps `getaddrinfo(3)` — DNS / service-name resolution.
///
/// Resolves `node` (hostname or numeric address) and `service`
/// (service name or port string) to a linked list of `addrinfo`
/// records, written through `res`. `hints` constrains the result
/// (family, socket type, protocol). Returns `0` on success or a
/// non-zero `EAI_*` code on failure (note: not an `errno`). The
/// caller must free the list with `freeaddrinfo`.
///
/// # `addrinfo` struct layout (macOS, 48 bytes)
///
/// ```
///   offset 0:  ai_flags    (i32)
///   offset 4:  ai_family   (i32)
///   offset 8:  ai_socktype (i32)
///   offset 12: ai_protocol (i32)
///   offset 16: ai_addrlen  (u32)
///   offset 20: padding     (4 bytes on macOS, differs from Linux)
///   offset 24: ai_canonname (ptr)
///   offset 32: ai_addr     (ptr)
///   offset 40: ai_next     (ptr)
/// ```
public func getaddrinfo(node: Pointer[UInt8], service: Pointer[UInt8], hints: Pointer[UInt8], res: Pointer[Pointer[UInt8]]) -> Int32 {
    Int32(raw: libc_getaddrinfo(
        lang.cast_ptr[_, lang.i8](node.raw),
        lang.cast_ptr[_, lang.i8](service.raw),
        lang.cast_ptr[_, lang.i8](hints.raw),
        lang.cast_ptr[_, lang.ptr[lang.i8]](res.raw)
    ))
}

/// Wraps `freeaddrinfo(3)` — frees the linked list returned by `getaddrinfo`.
///
/// Walks the `ai_next` chain and frees each node along with its
/// embedded `sockaddr` and (if present) `ai_canonname` buffer.
/// Always pair every successful `getaddrinfo` with one
/// `freeaddrinfo` to avoid leaking the resolver's allocation.
public func freeaddrinfo(res: Pointer[UInt8]) {
    libc_freeaddrinfo(lang.cast_ptr[_, lang.i8](res.raw))
}

/// Returns the current value of `errno` for the calling thread.
///
/// Read it immediately after a failing libc call — any subsequent
/// syscall (including the success of an unrelated one) may overwrite
/// it. Implementation forwards to the platform-specific accessor:
/// `__error` on darwin, `__errno_location` on linux.
public func errno() -> Int32 {
    let ptr = __errno_ptr();
    Int32(raw: lang.ptr_read(ptr))
}
