// Unix domain sockets: local connections named by a file system path.

module std.network

import std.numeric.(Int32, Int64, UInt8)
import std.collections.(Slice)
import std.core.(Bool)
import std.result.(Result, Optional)
import std.streams.(IoError, Readable, Writable, FileDescriptor)
import std.files.(Path)
import std.memory.(RawPointer)
import std.ffi.(malloc, free)

/// A connected Unix stream socket. Closed when dropped.
///
/// # Examples
///
/// ```
/// var (a, b) = try UnixStream.pair();
/// try a.write(line: "hello");
/// let line = try b.buffered().readLine();
/// ```
public struct UnixStream: Readable, Writable, not Copyable {
    var fd: Int32

    /// Connects to the socket at `path`.
    public static func connect(path: Path) -> Result[UnixStream, IoError] {
        let fd = try openSocket(unixFamily(), streamKind());
        let name = try path.string.toNativeString();
        let r = unsafe native_unix_connect(fd, name.raw.asRaw());
        let error = IoError.last();
        name.free();
        if r < 0 {
            return closeAfter(fd, error)
        }
        .Ok(UnixStream(unsafeDescriptor: FileDescriptor(fd)))
    }

    /// Two streams connected to each other (`socketpair`).
    public static func pair() -> Result[(UnixStream, UnixStream), IoError] {
        let cells = unsafe malloc(8);
        let r = unsafe native_socket_pair(cells);
        let error = IoError.last();
        let fds = cells.cast[Int32]();
        let (a, b) = (unsafe fds.read(), unsafe fds.offset(by: 1).read());
        unsafe free(cells);
        if r < 0 {
            return .Err(error)
        }
        .Ok((UnixStream(unsafeDescriptor: FileDescriptor(a)), UnixStream(unsafeDescriptor: FileDescriptor(b))))
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

    /// Nothing to flush: the stream doesn't buffer.
    public mutating func flush() -> Result[(), IoError] { .Ok(()) }

    /// Closes the reading side, the writing side, or both.
    public mutating func shutdown(how: Shutdown) -> Result[(), IoError] { shutdownOn(self.fd, how) }

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

    /// Makes reads and writes return `WouldBlock` instead of waiting.
    public mutating func setNonblocking(on: Bool) -> Result[(), IoError] { setNonblockingOn(self.fd, on) }

    /// A second handle with its own descriptor (`dup`).
    public func tryClone() -> Result[UnixStream, IoError] {
        .Ok(UnixStream(unsafeDescriptor: FileDescriptor(try duplicate(self.fd))))
    }

    /// The descriptor, still owned by this stream.
    public var rawDescriptor: FileDescriptor { FileDescriptor(self.fd) }

    /// Closes the socket and reports an error from `close`, which dropping
    /// it can't.
    public consuming func close() -> Result[(), IoError] {
        let fd = self.fd;
        self.fd = -1;
        check(unsafe native_close(fd))
    }

    /// Gives up ownership: the descriptor stays open and the caller closes
    /// it.
    public consuming func intoRawDescriptor() -> FileDescriptor {
        let fd = self.fd;
        self.fd = -1;
        FileDescriptor(fd)
    }

    deinit {
        if self.fd >= 0 {
            unsafe native_close(self.fd);
        }
    }
}

/// A Unix stream server socket. Closed when dropped; the socket file stays
/// until removed.
public struct UnixListener: not Copyable {
    var fd: Int32

    /// Listens on a new socket file at `path`, which must not exist.
    public static func bind(path: Path) -> Result[UnixListener, IoError] {
        let fd = try openSocket(unixFamily(), streamKind());
        let name = try path.string.toNativeString();
        let r = unsafe native_unix_bind(fd, name.raw.asRaw());
        let error = IoError.last();
        name.free();
        if r < 0 {
            return closeAfter(fd, error)
        }
        match listenOn(fd) {
            .Ok(_) => .Ok(UnixListener(fd: fd)),
            .Err(e) => closeAfter(fd, e)
        }
    }

    /// Waits for a connection.
    public func accept() -> Result[UnixStream, IoError] {
        let (client, _) = try acceptOn(self.fd);
        .Ok(UnixStream(unsafeDescriptor: FileDescriptor(client)))
    }

    /// Makes `accept` return `WouldBlock` instead of waiting.
    public mutating func setNonblocking(on: Bool) -> Result[(), IoError] { setNonblockingOn(self.fd, on) }

    /// The descriptor, still owned by this listener.
    public var rawDescriptor: FileDescriptor { FileDescriptor(self.fd) }

    deinit {
        if self.fd >= 0 {
            unsafe native_close(self.fd);
        }
    }
}
