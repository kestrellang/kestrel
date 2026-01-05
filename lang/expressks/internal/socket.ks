// Socket types and C extern bindings for networking
//
// This module provides low-level socket operations via @extern(.C) bindings.

module expressks.internal.socket;

import std.memory.pointer;
import std.ffi.(FFISafe)

// Socket constants (POSIX)
public let AF_INET: Int32 = 2;
public let SOCK_STREAM: Int32 = 1;
public let IPPROTO_TCP: Int32 = 6;
public let SOL_SOCKET: Int32 = 0xFFFF;  // macOS value
public let SO_REUSEADDR: Int32 = 0x0004; // macOS value
public let INADDR_ANY: UInt32 = 0;

// sockaddr_in equivalent (16 bytes on most systems)
// struct sockaddr_in {
//     sa_family_t    sin_family;  // 2 bytes
//     in_port_t      sin_port;    // 2 bytes (network byte order)
//     struct in_addr sin_addr;    // 4 bytes
//     char           sin_zero[8]; // 8 bytes padding
// }
// FFISafe because all fields are FFI-safe primitive types and tuples
public struct SockAddrIn: FFISafe {
    public var sin_len: UInt8;       // macOS has length prefix
    public var sin_family: UInt8;
    public var sin_port: UInt16;     // Network byte order (big endian)
    public var sin_addr: UInt32;     // Network byte order
    public var sin_zero: (UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8);

    public init(port: UInt16, addr: UInt32) {
        self.sin_len = 16;
        self.sin_family = AF_INET as UInt8;
        self.sin_port = Self.htons(port);
        self.sin_addr = addr;
        self.sin_zero = (0, 0, 0, 0, 0, 0, 0, 0);
    }

    public static func any(port: UInt16) -> SockAddrIn {
        SockAddrIn(port: port, addr: INADDR_ANY)
    }

    // Convert host byte order to network byte order (big endian)
    public static func htons(value: UInt16) -> UInt16 {
        ((value & 0xFF) << 8) | ((value >> 8) & 0xFF)
    }

    public static func htonl(value: UInt32) -> UInt32 {
        ((value & 0xFF) << 24) |
        ((value & 0xFF00) << 8) |
        ((value >> 8) & 0xFF00) |
        ((value >> 24) & 0xFF)
    }

    public static func ntohs(value: UInt16) -> UInt16 {
        Self.htons(value)  // Same operation for both directions
    }

    public static func ntohl(value: UInt32) -> UInt32 {
        Self.htonl(value)  // Same operation for both directions
    }
}

// C extern declarations for socket operations

@extern(.C)
public func socket(domain: Int32, type: Int32, protocol: Int32) -> Int32 {}

@extern(.C)
public func bind(sockfd: Int32, addr: Pointer[SockAddrIn], addrlen: UInt32) -> Int32 {}

@extern(.C)
public func listen(sockfd: Int32, backlog: Int32) -> Int32 {}

@extern(.C)
public func accept(sockfd: Int32, addr: Pointer[SockAddrIn], addrlen: Pointer[UInt32]) -> Int32 {}

@extern(.C)
public func setsockopt(sockfd: Int32, level: Int32, optname: Int32, optval: Pointer[Int32], optlen: UInt32) -> Int32 {}

@extern(.C, mangleName: "read")
public func readSocket(fd: Int32, buf: Pointer[UInt8], count: Int) -> Int {}

@extern(.C, mangleName: "write")
public func writeSocket(fd: Int32, buf: Pointer[UInt8], count: Int) -> Int {}

@extern(.C)
public func close(fd: Int32) -> Int32 {}

// Helper to create and configure a server socket
public func createServerSocket(port: UInt16) -> Result[Int32, String] {
    // Create socket
    let fd = socket(domain: AF_INET, type: SOCK_STREAM, protocol: IPPROTO_TCP);

    if fd < 0 {
        return .Err("Failed to create socket");
    }

    // Set SO_REUSEADDR
    var optval: Int32 = 1;
    let optResult = setsockopt(
        sockfd: fd,
        level: SOL_SOCKET,
        optname: SO_REUSEADDR,
        optval: Pointer(to: ref optval),
        optlen: 4
    );

    if optResult < 0 {
        close(fd: fd);
        return .Err("Failed to set socket options");
    }

    // Bind
    var addr = SockAddrIn.any(port: port);
    let bindResult = bind(
        sockfd: fd,
        addr: Pointer(to: ref addr),
        addrlen: 16
    );

    if bindResult < 0 {
        close(fd: fd);
        return .Err("Failed to bind to port");
    }

    // Listen
    let listenResult = listen(sockfd: fd, backlog: 128);
    if listenResult < 0 {
        close(fd: fd);
        return .Err("Failed to listen");
    }

    .Ok(fd)
}
