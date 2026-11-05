// TCP connections and listeners.

module std.network

import std.numeric.(Int32, Int64, UInt8, UInt16)
import std.collections.(Array, Slice)
import std.text.(String)
import std.core.(Bool)
import std.result.(Result, Optional)
import std.iter.(Iterator)
import std.streams.(IoError, IoErrorKind, Readable, Writable, FileDescriptor)
import std.system.(native_monotonic_ms)

/// A TCP connection. Closed when dropped.
///
/// It is `Readable` and `Writable`, so every stream method works on it.
/// Writing after the peer has closed is a `BrokenPipe` error, never a
/// signal.
///
/// # Examples
///
/// ```
/// var stream = try TcpStream.connect("example.com", 80);
/// try stream.write(text: "GET / HTTP/1.0\r\nHost: example.com\r\n\r\n");
/// let page = try stream.readText();
/// ```
public struct TcpStream: Readable, Writable, not Copyable {
    var fd: Int32

    /// Resolves `host` and tries each address in turn until one connects.
    ///
    /// # Errors
    ///
    /// `HostNotFound` if `host` doesn't resolve, otherwise the last
    /// address's error, such as `ConnectionRefused`.
    public static func connect(host: String, port: UInt16) -> Result[TcpStream, IoError] {
        connectToAny(host, port, -1)
    }

    /// Like `connect(host, port)`, failing with `TimedOut` if no address
    /// connects within `timeoutMilliseconds`, lookup included.
    public static func connect(host: String, port: UInt16, timeoutMilliseconds timeoutMilliseconds: Int64) -> Result[TcpStream, IoError] {
        if timeoutMilliseconds < 0 {
            return .Err(IoError.invalidInput)
        }
        connectToAny(host, port, timeoutMilliseconds)
    }

    /// Connects to `address`.
    public static func connect(to address: SocketAddress) -> Result[TcpStream, IoError] {
        let fd = try connectSocket(address, streamKind(), -1);
        .Ok(TcpStream(unsafeDescriptor: FileDescriptor(fd)))
    }

    /// Connects to `address`, failing with `TimedOut` after
    /// `timeoutMilliseconds`.
    public static func connect(to address: SocketAddress, timeoutMilliseconds timeoutMilliseconds: Int64) -> Result[TcpStream, IoError] {
        if timeoutMilliseconds < 0 {
            return .Err(IoError.invalidInput)
        }
        let fd = try connectSocket(address, streamKind(), timeoutMilliseconds);
        .Ok(TcpStream(unsafeDescriptor: FileDescriptor(fd)))
    }

    /// @name From Descriptor
    /// Takes ownership of a connected socket: the stream closes it.
    public init(unsafeDescriptor descriptor: FileDescriptor) {
        self.fd = descriptor.value
    }

    /// Reads what has arrived, waiting for at least one byte; `0` once the
    /// peer has closed its side.
    public mutating func read(mutating into buf: Slice[UInt8]) -> Result[Int64, IoError] {
        receiveOn(self.fd, buf)
    }

    /// Sends up to `buf.count` bytes; returns how many were taken.
    public mutating func write(from buf: Slice[UInt8]) -> Result[Int64, IoError] {
        sendOn(self.fd, buf)
    }

    /// Nothing to flush: the stream doesn't buffer. Use `setNoDelay` to
    /// send small writes at once.
    public mutating func flush() -> Result[(), IoError] { .Ok(()) }

    /// The address of the other end.
    public func peerAddress() -> Result[SocketAddress, IoError] { addressOf(self.fd, true) }

    /// The local end's address.
    public func localAddress() -> Result[SocketAddress, IoError] { addressOf(self.fd, false) }

    /// Closes the reading side, the writing side, or both. After
    /// `shutdown(.Write)` the peer reads end of stream, and this side can
    /// still read its reply.
    public mutating func shutdown(how: Shutdown) -> Result[(), IoError] { shutdownOn(self.fd, how) }

    /// Turns off Nagle's algorithm, so small writes go out at once.
    public mutating func setNoDelay(on: Bool) -> Result[(), IoError] { setFlag(self.fd, noDelayOption(), on) }

    /// Turns on TCP keepalive probes on an idle connection.
    public mutating func setKeepAlive(on: Bool) -> Result[(), IoError] { setFlag(self.fd, keepAliveOption(), on) }

    /// Makes a read that waits longer than `milliseconds` fail with
    /// `WouldBlock`. `None` waits forever.
    public mutating func setReadTimeout(milliseconds milliseconds: Optional[Int64]) -> Result[(), IoError] {
        setTimeout(self.fd, false, milliseconds)
    }

    /// Makes a write that waits longer than `milliseconds` fail with
    /// `WouldBlock`. `None` waits forever.
    public mutating func setWriteTimeout(milliseconds milliseconds: Optional[Int64]) -> Result[(), IoError] {
        setTimeout(self.fd, true, milliseconds)
    }

    /// Makes reads and writes return `WouldBlock` instead of waiting, for
    /// an event loop polling `rawDescriptor`.
    public mutating func setNonblocking(on: Bool) -> Result[(), IoError] { setNonblockingOn(self.fd, on) }

    /// A second handle to the same connection with its own descriptor
    /// (`dup`). Each closes its own descriptor.
    public func tryClone() -> Result[TcpStream, IoError] {
        .Ok(TcpStream(unsafeDescriptor: FileDescriptor(try duplicate(self.fd))))
    }

    /// The descriptor, still owned by this stream.
    public var rawDescriptor: FileDescriptor { FileDescriptor(self.fd) }

    /// Closes the socket and reports an error from `close`, which dropping
    /// it can't.
    public consuming func close() -> Result[(), IoError] {
        let fd = self.fd;
        self.fd = -1;
        check(native_close(fd))
    }

    /// Gives up ownership: the descriptor stays open and the caller closes
    /// it, as when handing the connection to a TLS library.
    public consuming func intoRawDescriptor() -> FileDescriptor {
        let fd = self.fd;
        self.fd = -1;
        FileDescriptor(fd)
    }

    deinit {
        if self.fd >= 0 {
            native_close(self.fd);
        }
    }
}

/// Tries each address of `host` in order. A negative timeout waits
/// forever; otherwise it bounds the whole attempt.
func connectToAny(host: String, port: UInt16, timeoutMilliseconds: Int64) -> Result[TcpStream, IoError] {
    let deadline = native_monotonic_ms() + timeoutMilliseconds;
    var last = IoError.unexpectedEnd;
    for address in try lookup(host, port) {
        var remaining: Int64 = -1;
        if timeoutMilliseconds >= 0 {
            remaining = deadline - native_monotonic_ms();
            if remaining <= 0 {
                return .Err(IoError.timedOut)
            }
        }
        match connectSocket(address, streamKind(), remaining) {
            .Ok(fd) => return .Ok(TcpStream(unsafeDescriptor: FileDescriptor(fd))),
            .Err(e) => last = e
        }
    }
    .Err(last)
}

/// A TCP server socket. Closed when dropped.
///
/// # Examples
///
/// ```
/// let listener = try TcpListener.bind(8080);
/// loop {
///     var (client, peer) = try listener.accept();
///     try client.write(line: "hello, \(peer)");
/// }
/// ```
public struct TcpListener: not Copyable {
    var fd: Int32

    /// Listens on `address`. Port 0 picks a free port; `localAddress()`
    /// says which.
    public static func bind(to address: SocketAddress) -> Result[TcpListener, IoError] {
        let fd = try bindSocket(address, streamKind(), true);
        match listenOn(fd) {
            .Ok(_) => .Ok(TcpListener(fd: fd)),
            .Err(e) => closeAfter(fd, e)
        }
    }

    /// Listens on `port` on every local address, IPv6 and IPv4 (`[::]`,
    /// or `0.0.0.0` where IPv6 is unavailable).
    public static func bind(port: UInt16) -> Result[TcpListener, IoError] {
        match TcpListener.bind(to: SocketAddress(IpAddress.V6(Ipv6Address.any), port: port)) {
            .Ok(listener) => .Ok(listener),
            .Err(e) => {
                // Only a missing IPv6 stack falls back to IPv4.
                if e.kind == IoErrorKind.AddressInUse or e.kind == IoErrorKind.PermissionDenied {
                    return .Err(e)
                }
                TcpListener.bind(to: SocketAddress(IpAddress.V4(Ipv4Address.any), port: port))
            }
        }
    }

    /// Waits for a connection; returns it and the peer's address.
    public func accept() -> Result[(TcpStream, SocketAddress), IoError] {
        let (client, peer) = try acceptOn(self.fd);
        .Ok((TcpStream(unsafeDescriptor: FileDescriptor(client)), peer))
    }

    /// The connections, as they arrive. Takes the listener, which is
    /// closed when the iterator is dropped.
    public consuming func incoming() -> Incoming {
        Incoming(listener: self)
    }

    /// The address it listens on: the port picked after binding port 0.
    public func localAddress() -> Result[SocketAddress, IoError] { addressOf(self.fd, false) }

    /// Makes `accept` return `WouldBlock` instead of waiting.
    public mutating func setNonblocking(on: Bool) -> Result[(), IoError] { setNonblockingOn(self.fd, on) }

    /// The descriptor, still owned by this listener.
    public var rawDescriptor: FileDescriptor { FileDescriptor(self.fd) }

    deinit {
        if self.fd >= 0 {
            native_close(self.fd);
        }
    }
}

/// The connections a `TcpListener` accepts, endlessly. Each is a
/// `Result[TcpStream, IoError]`; an error doesn't end the iteration.
public struct Incoming: Iterator, not Copyable {
    type Item = Result[TcpStream, IoError]

    var listener: TcpListener

    /// Waits for the next connection.
    public mutating func next() -> Optional[Result[TcpStream, IoError]] {
        .Some(match self.listener.accept() {
            .Ok((stream, _)) => .Ok(stream),
            .Err(e) => .Err(e)
        })
    }
}
