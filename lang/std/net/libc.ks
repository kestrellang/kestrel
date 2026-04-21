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

/// IPv4 protocol family.
public func AF_INET() -> Int32 { 2 }

/// Stream socket (TCP).
public func SOCK_STREAM() -> Int32 { 1 }

/// TCP protocol number.
public func IPPROTO_TCP() -> Int32 { 6 }

// errno access
@platform(.darwin)
@extern(.C, mangleName: "__error")
func __errno_ptr() -> lang.ptr[lang.i32]

@platform(.linux)
@extern(.C, mangleName: "__errno_location")
func __errno_ptr() -> lang.ptr[lang.i32]

// Socket constants (platform-specific values)

/// Socket-level options.
@platform(.darwin)
public func SOL_SOCKET() -> Int32 { 0xFFFF }

/// Socket-level options.
@platform(.linux)
public func SOL_SOCKET() -> Int32 { 1 }

/// Allow address reuse.
@platform(.darwin)
public func SO_REUSEADDR() -> Int32 { 0x0004 }

/// Allow address reuse.
@platform(.linux)
public func SO_REUSEADDR() -> Int32 { 2 }

/// Bind to all interfaces.
public func INADDR_ANY() -> Int32 { 0 }

/// Size of sockaddr_in struct.
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

/// Creates a socket. Returns fd or -1 on error.
public func socket(domain: Int32, type_: Int32, proto: Int32) -> Int32 {
    Int32(raw: libc_socket(domain.raw, type_.raw, proto.raw))
}

/// Binds a socket to an address. Returns 0 on success, -1 on error.
public func bind(sockfd: Int32, addr: Pointer[UInt8], addrlen: Int32) -> Int32 {
    Int32(raw: libc_bind(sockfd.raw, lang.cast_ptr[_, lang.i8](addr.raw), addrlen.raw))
}

/// Listens for connections. Returns 0 on success, -1 on error.
public func listen(sockfd: Int32, backlog: Int32) -> Int32 {
    Int32(raw: libc_listen(sockfd.raw, backlog.raw))
}

/// Accepts a connection. Returns new fd or -1 on error.
/// Pass null pointers to ignore client address.
public func accept(sockfd: Int32, addr: Pointer[UInt8], addrlen: Pointer[Int32]) -> Int32 {
    Int32(raw: libc_accept(sockfd.raw, lang.cast_ptr[_, lang.i8](addr.raw), lang.cast_ptr[_, lang.i32](addrlen.raw)))
}

/// Receives data from a socket. Returns bytes read, 0 on close, -1 on error.
public func recv(sockfd: Int32, buf: Pointer[UInt8], len: Int64, flags: Int32) -> Int64 {
    Int64(raw: libc_recv(sockfd.raw, lang.cast_ptr[_, lang.i8](buf.raw), len.raw, flags.raw))
}

/// Sends data on a socket. Returns bytes sent or -1 on error.
public func send(sockfd: Int32, buf: Pointer[UInt8], len: Int64, flags: Int32) -> Int64 {
    Int64(raw: libc_send(sockfd.raw, lang.cast_ptr[_, lang.i8](buf.raw), len.raw, flags.raw))
}

/// Closes a file descriptor. Returns 0 on success, -1 on error.
public func close(fd: Int32) -> Int32 {
    Int32(raw: libc_close(fd.raw))
}

/// Sets a socket option. Returns 0 on success, -1 on error.
public func setsockopt(sockfd: Int32, level: Int32, optname: Int32, optval: Pointer[UInt8], optlen: Int32) -> Int32 {
    Int32(raw: libc_setsockopt(sockfd.raw, level.raw, optname.raw, lang.cast_ptr[_, lang.i8](optval.raw), optlen.raw))
}

/// Converts host byte order to network byte order (16-bit).
public func htons(hostshort: UInt16) -> UInt16 {
    UInt16(raw: libc_htons(hostshort.raw))
}

/// Connects a socket to a remote address. Returns 0 on success, -1 on error.
public func connect(sockfd: Int32, addr: Pointer[UInt8], addrlen: Int32) -> Int32 {
    Int32(raw: libc_connect(sockfd.raw, lang.cast_ptr[_, lang.i8](addr.raw), addrlen.raw))
}

/// Resolves a hostname to socket addresses.
/// Returns 0 on success, non-zero error code on failure.
/// The result pointer must be freed with freeaddrinfo().
///
/// addrinfo struct layout (macOS, 48 bytes):
///   offset 0:  ai_flags    (i32)
///   offset 4:  ai_family   (i32)
///   offset 8:  ai_socktype (i32)
///   offset 12: ai_protocol (i32)
///   offset 16: ai_addrlen  (u32)
///   offset 20: padding     (4 bytes on macOS, differs from Linux)
///   offset 24: ai_canonname (ptr)
///   offset 32: ai_addr     (ptr)
///   offset 40: ai_next     (ptr)
public func getaddrinfo(node: Pointer[UInt8], service: Pointer[UInt8], hints: Pointer[UInt8], res: Pointer[Pointer[UInt8]]) -> Int32 {
    Int32(raw: libc_getaddrinfo(
        lang.cast_ptr[_, lang.i8](node.raw),
        lang.cast_ptr[_, lang.i8](service.raw),
        lang.cast_ptr[_, lang.i8](hints.raw),
        lang.cast_ptr[_, lang.ptr[lang.i8]](res.raw)
    ))
}

/// Frees the addrinfo linked list returned by getaddrinfo().
public func freeaddrinfo(res: Pointer[UInt8]) {
    libc_freeaddrinfo(lang.cast_ptr[_, lang.i8](res.raw))
}

/// Returns the current errno value.
public func errno() -> Int32 {
    let ptr = __errno_ptr();
    Int32(raw: lang.ptr_read(ptr))
}
