// TCP socket types

module std.net.socket

import std.num.(Int64, Int32, UInt8, UInt16)
import std.result.(Result)
import std.memory.(Slice, Pointer)
import std.collections.(Array)
import std.text.(String)
import std.core.(Bool)
import std.net.libc
import std.io.error.(Error)
import std.io.read.(Read)
import std.io.write.(Write)

/// Size in bytes of `struct addrinfo` on darwin (`48`).
@platform(.darwin)
func ADDRINFO_SIZE() -> Int64 { 48 }

/// Size in bytes of `struct addrinfo` on linux (`48`).
@platform(.linux)
func ADDRINFO_SIZE() -> Int64 { 48 }

/// Byte offset of the `ai_addr` pointer within `addrinfo` on darwin.
@platform(.darwin)
func AI_ADDR_OFFSET() -> Int64 { 32 }

/// Byte offset of the `ai_addr` pointer within `addrinfo` on linux. Differs from darwin because of `ai_canonname` ordering.
@platform(.linux)
func AI_ADDR_OFFSET() -> Int64 { 24 }

/// Builds a 16-byte `sockaddr_in` for binding to the wildcard IPv4 address on `port`.
///
/// On darwin, `sockaddr_in` begins with a `sin_len` byte before
/// `sin_family`; this accessor lays out the bytes accordingly. The
/// port is encoded big-endian (network byte order). The address is
/// hard-coded to `INADDR_ANY` (`0.0.0.0`).
@platform(.darwin)
func buildSockaddrIn(port: UInt16) -> Array[UInt8] {
    var addr = Array[UInt8]();
    // sin_len = 16, sin_family = AF_INET (2)
    addr.append(16);
    addr.append(2);
    // sin_port in network byte order (big-endian)
    let port64 = Int64(from: port);
    let portHi = port64 / 256;
    let portLo = port64 % 256;
    addr.append(UInt8(from: portHi));
    addr.append(UInt8(from: portLo));
    // sin_addr = INADDR_ANY + zero padding (12 bytes)
    var pad: Int64 = 0;
    while pad < 12 {
        addr.append(0);
        pad = pad + 1
    }
    addr
}

/// Linux variant of `buildSockaddrIn` — `sin_family` is a 16-bit field with no leading `sin_len`.
@platform(.linux)
func buildSockaddrIn(port: UInt16) -> Array[UInt8] {
    var addr = Array[UInt8]();
    // sin_family = AF_INET (2) as little-endian uint16
    addr.append(2);
    addr.append(0);
    // sin_port in network byte order (big-endian)
    let port64 = Int64(from: port);
    let portHi = port64 / 256;
    let portLo = port64 % 256;
    addr.append(UInt8(from: portHi));
    addr.append(UInt8(from: portLo));
    // sin_addr = INADDR_ANY + zero padding (12 bytes)
    var pad: Int64 = 0;
    while pad < 12 {
        addr.append(0);
        pad = pad + 1
    }
    addr
}

/// A connected TCP byte stream — implements `Read` and `Write` on top of a POSIX socket fd.
///
/// Returned by `TcpListener.accept()` (server side) and
/// `TcpStream.connect(host:port:)` (client side). Reads and writes
/// go directly through `recv(2)` / `send(2)`; partial reads/writes
/// are surfaced — the caller is responsible for looping. The owned
/// fd is closed automatically by the deinit unless `detachFd` has
/// been called first.
///
/// # Examples
///
/// ```
/// var stream = match TcpStream.connect(host: "example.com", port: UInt16(intLiteral: 80)) {
///     .Ok(s) => s,
///     .Err(e) => return .Err(e)
/// };
/// // stream is Read + Write
/// ```
///
/// # Representation
///
/// A single `Int32` field holding the file descriptor; `-1` means
/// "detached, do not close on drop".
///
/// # Memory Model
///
/// Owns its fd. Cloning is not provided — duplicate explicitly via
/// `dup(2)` if you need it.
public struct TcpStream: Read, Write {
    var fd: Int32

    /// @name From Fd
    /// Wraps an existing socket fd as a `TcpStream`.
    ///
    /// The stream takes ownership; the deinit will close the fd.
    /// Callers obtaining the fd from `accept` / `socket` should
    /// hand it over and stop using it directly.
    public init(fd: Int32) {
        self.fd = fd
    }

    /// Reads up to `buf.count` bytes into `buf`. Returns the byte count actually read.
    ///
    /// `0` indicates the peer closed the connection cleanly. Required
    /// by the `Read` protocol.
    ///
    /// # Errors
    ///
    /// Returns `Err(Error)` from the captured `errno` if `recv`
    /// returns `-1`.
    public mutating func read(into buf: Slice[UInt8]) -> Result[Int64, Error] {
        let n = libc.recv(self.fd, buf.pointer, buf.count, 0);
        if n < 0 {
            return .Err(Error.last())
        }
        .Ok(n)
    }

    /// Writes up to `buf.count` bytes from `buf`. Returns the byte count actually written.
    ///
    /// May write fewer bytes than requested under back-pressure;
    /// loop until the buffer is drained. Required by the `Write`
    /// protocol.
    ///
    /// # Errors
    ///
    /// Returns `Err(Error)` from the captured `errno` if `send`
    /// returns `-1`.
    public mutating func write(from buf: Slice[UInt8]) -> Result[Int64, Error] {
        let n = libc.send(self.fd, buf.pointer, buf.count, 0);
        if n < 0 {
            return .Err(Error.last())
        }
        .Ok(n)
    }

    /// No-op — TCP sockets do not have an application-level write buffer.
    ///
    /// Always returns `Ok(())`. Provided to satisfy the `Write`
    /// protocol so generic writers can call `flush` unconditionally.
    public mutating func flush() -> Result[(), Error] {
        .Ok(())
    }

    /// Returns the underlying fd without giving up ownership.
    ///
    /// Useful for passing the fd to syscalls that the wrapper does
    /// not expose (`fcntl`, `setsockopt`, …). Do not close it
    /// yourself — the deinit still will.
    public func rawFd() -> Int32 {
        self.fd
    }

    /// Releases ownership of the fd and returns it.
    ///
    /// Sets the internal fd to `-1` so the deinit becomes a no-op.
    /// The caller takes responsibility for closing the returned fd.
    /// Use this when handing the fd to another owner (e.g. an event
    /// loop or a child process).
    public mutating func detachFd() -> Int32 {
        let fd = self.fd;
        self.fd = -1;
        fd
    }

    /// Closes the owned fd if any. The `-1` sentinel set by `detachFd` makes this a no-op.
    deinit {
        if self.fd >= 0 {
            let _ = libc.close(self.fd);
        }
    }
}

extend TcpStream {
    /// Resolves `host`:`port` and returns a connected `TcpStream`.
    ///
    /// Uses `getaddrinfo` for resolution and tries the first result.
    /// Constrained to IPv4 / TCP via the `hints` block. On any
    /// failure the partially-built fd is closed and the resolver
    /// list is freed before returning. Does not currently fall
    /// through to the next `addrinfo` entry on a failed
    /// `connect` — try one address.
    ///
    /// # Errors
    ///
    /// - Returns `Err(Error(eai))` with the `EAI_*` resolver code if
    ///   `getaddrinfo` fails (note: this is a libc resolver code,
    ///   not an `errno`).
    /// - Returns `Err(Error.last())` from `errno` if `socket()` or
    ///   `connect()` fail.
    ///
    /// # Examples
    ///
    /// ```
    /// match TcpStream.connect(host: "example.com", port: UInt16(intLiteral: 80)) {
    ///     .Ok(stream) => /* use stream */ {},
    ///     .Err(e) => print(e.message)
    /// }
    /// ```
    public static func connect(host: String, port: UInt16) -> Result[TcpStream, Error] {
        // Build port string for getaddrinfo
        let port64 = Int64(from: port);
        let portStr = port64.format();

        // Set up hints: AF_INET, SOCK_STREAM, IPPROTO_TCP
        let addrinfoSize = ADDRINFO_SIZE();
        var hints = Array[UInt8](capacity: addrinfoSize);
        var hi: Int64 = 0;
        while hi < addrinfoSize {
            hints.append(0);
            hi = hi + 1
        }
        // ai_family = AF_INET (2) at offset 4
        let hintsPtr = hints.asPointer();
        hintsPtr.offset(by: 4).cast[Int32]().write(libc.AF_INET());
        // ai_socktype = SOCK_STREAM (1) at offset 8
        hintsPtr.offset(by: 8).cast[Int32]().write(libc.SOCK_STREAM());
        // ai_protocol = IPPROTO_TCP (6) at offset 12
        hintsPtr.offset(by: 12).cast[Int32]().write(libc.IPPROTO_TCP());

        // Null-terminate host and port strings for C
        var hostBuf = Array[UInt8]();
        var hci: Int64 = 0;
        while hci < host.byteCount {
            hostBuf.append(host.byteAtUnchecked(hci));
            hci = hci + 1
        }
        hostBuf.append(0);

        var portBuf = Array[UInt8]();
        var pci: Int64 = 0;
        while pci < portStr.byteCount {
            portBuf.append(portStr.byteAtUnchecked(pci));
            pci = pci + 1
        }
        portBuf.append(0);

        // Call getaddrinfo
        var resultPtr = Pointer[UInt8].nullPointer();
        let gaiResult = libc.getaddrinfo(
            hostBuf.asPointer(),
            portBuf.asPointer(),
            hints.asPointer(),
            Pointer(to: resultPtr).cast[Pointer[UInt8]]()
        );
        if gaiResult != 0 {
            return .Err(Error(gaiResult))
        }

        // Extract address info from first result
        // ai_family at offset 4, ai_socktype at offset 8, ai_protocol at offset 12
        // ai_addrlen at offset 16, ai_addr offset is platform-specific
        let infoPtr = resultPtr;
        let family = infoPtr.offset(by: 4).cast[Int32]().read();
        let socktype = infoPtr.offset(by: 8).cast[Int32]().read();
        let proto = infoPtr.offset(by: 12).cast[Int32]().read();
        let addrlen = infoPtr.offset(by: 16).cast[Int32]().read();
        let addrPtr = infoPtr.offset(by: AI_ADDR_OFFSET()).cast[Pointer[UInt8]]().read();

        // Create socket
        let fd = libc.socket(family, socktype, proto);
        if fd < 0 {
            libc.freeaddrinfo(resultPtr);
            return .Err(Error.last())
        }

        // Connect
        let connResult = libc.connect(fd, addrPtr, addrlen);
        libc.freeaddrinfo(resultPtr);

        if connResult < 0 {
            let _ = libc.close(fd);
            return .Err(Error.last())
        }

        .Ok(TcpStream(fd))
    }
}

/// A bound, listening TCP server socket.
///
/// Created by `TcpListener.bind(port:)` — sets `SO_REUSEADDR`,
/// binds to `INADDR_ANY:port`, and calls `listen(2)` with backlog
/// `128`. Accept connections via `accept()`, which blocks until
/// the next client arrives. The owned fd is closed by the deinit.
///
/// # Examples
///
/// ```
/// let listener = match TcpListener.bind(port: UInt16(intLiteral: 8080)) {
///     .Ok(l) => l,
///     .Err(e) => return .Err(e)
/// };
/// while true {
///     match listener.accept() {
///         .Ok(stream) => /* handle stream */ {},
///         .Err(e) => break
///     }
/// }
/// ```
///
/// # Representation
///
/// A single `Int32` field — the listening socket fd.
///
/// # Memory Model
///
/// Owns its fd; closed on drop.
public struct TcpListener {
    var fd: Int32

    /// @name From Fd
    /// Internal — wraps an existing fd. Callers should use `bind(port:)`.
    init(fd: Int32) {
        self.fd = fd
    }

    /// Creates a server socket bound to `0.0.0.0:port` with `SO_REUSEADDR` and a backlog of 128.
    ///
    /// Walks the full setup — `socket` → `setsockopt` → `bind` →
    /// `listen` — and cleans up the partial fd on any failure.
    ///
    /// # Errors
    ///
    /// Returns `Err(Error.last())` (captured `errno`) at any of the
    /// four steps; the most common case is `EADDRINUSE` if another
    /// process holds the port and `SO_REUSEADDR` is not enough.
    ///
    /// # Examples
    ///
    /// ```
    /// let listener = TcpListener.bind(port: UInt16(intLiteral: 8080));
    /// ```
    public static func bind(port: UInt16) -> Result[TcpListener, Error] {
        let fd = libc.socket(libc.AF_INET(), libc.SOCK_STREAM(), libc.IPPROTO_TCP());
        if fd < 0 {
            return .Err(Error.last())
        }

        // Set SO_REUSEADDR
        var optval: Int32 = 1;
        let optPtr = Pointer(to: optval).cast[UInt8]();
        let optResult = libc.setsockopt(fd, libc.SOL_SOCKET(), libc.SO_REUSEADDR(), optPtr, 4);
        if optResult < 0 {
            let _ = libc.close(fd);
            return .Err(Error.last())
        }

        // Build sockaddr_in (platform-specific layout, 16 bytes)
        var addr = buildSockaddrIn(port);

        let bindResult = libc.bind(fd, addr.asPointer(), libc.SOCKADDR_IN_SIZE());
        if bindResult < 0 {
            let _ = libc.close(fd);
            return .Err(Error.last())
        }

        let listenResult = libc.listen(fd, 128);
        if listenResult < 0 {
            let _ = libc.close(fd);
            return .Err(Error.last())
        }

        .Ok(TcpListener(fd))
    }

    /// Blocks until the next client connects, then returns it as a `TcpStream`.
    ///
    /// Discards the client's address — pass non-null pointers to
    /// `libc.accept` directly if you need it. Each accepted
    /// connection has its own fd, independent of the listener.
    ///
    /// # Errors
    ///
    /// Returns `Err(Error.last())` if `accept(2)` fails — common
    /// causes include `EINTR` (interrupted by signal) and
    /// `EMFILE` (per-process fd limit).
    public func accept() -> Result[TcpStream, Error] {
        let clientFd = libc.accept(self.fd, Pointer[UInt8].nullPointer(), Pointer[Int32].nullPointer());
        if clientFd < 0 {
            return .Err(Error.last())
        }
        .Ok(TcpStream(clientFd))
    }

    /// Returns the underlying listening fd without giving up ownership.
    public func rawFd() -> Int32 {
        self.fd
    }

    /// Closes the listening fd if any.
    deinit {
        if self.fd >= 0 {
            let _ = libc.close(self.fd);
        }
    }
}
