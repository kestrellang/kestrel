// UDP sockets.

module std.network

import std.numeric.(Int32, Int64, UInt8)
import std.collections.(Slice)
import std.core.(Bool)
import std.result.(Result, Optional)
import std.streams.(IoError, FileDescriptor)
import std.ffi.(malloc, free)
import std.time.(Duration)

/// A UDP socket. Closed when dropped.
///
/// # Examples
///
/// ```
/// var socket = try UdpSocket.bind(to: SocketAddress.parse("127.0.0.1:0").unwrap());
/// try socket.send("ping".asByteSlice(), to: server);
/// var buf = Array[UInt8](repeating: 0, count: 1500);
/// let (n, from) = try socket.receiveFrom(into: buf.asMutableSlice());
/// ```
public struct UdpSocket: not Copyable {
    var fd: Int32

    /// A socket bound to `address`. Port 0 picks a free port.
    public static func bind(to address: SocketAddress) -> Result[UdpSocket, IoError] {
        let fd = try bindSocket(address, datagramKind(), false);
        .Ok(UdpSocket(unsafeDescriptor: FileDescriptor(fd)))
    }

    /// @name From Descriptor
    /// Takes ownership of a UDP socket: this value closes it.
    public init(unsafeDescriptor descriptor: FileDescriptor) {
        self.fd = descriptor.value
    }

    /// Sends one datagram of `buf` to `address`; returns the bytes sent.
    public mutating func send(buf: Slice[UInt8], to address: SocketAddress) -> Result[Int64, IoError] {
        let cells = unsafe malloc(nativeAddressSize());
        address.writeNative(into: cells);
        let n = unsafe native_send_to(self.fd, buf.pointer.asRaw(), buf.count, cells);
        let error = IoError.last();
        unsafe free(cells);
        if n < 0 { .Err(error) } else { .Ok(n) }
    }

    /// Waits for one datagram; returns its size and sender. A datagram
    /// longer than `buf` is cut short.
    public mutating func receiveFrom(mutating into buf: Slice[UInt8]) -> Result[(Int64, SocketAddress), IoError] {
        let cells = unsafe malloc(nativeAddressSize());
        let n = unsafe native_receive_from(self.fd, buf.pointer.asRaw(), buf.count, cells);
        let error = IoError.last();
        let result = if n < 0 { .Err(error) } else { .Ok((n, SocketAddress.readNative(from: cells))) };
        unsafe free(cells);
        result
    }

    /// Fixes the peer: `send(buf)` goes to `address`, and only its
    /// datagrams are received.
    public mutating func connect(to address: SocketAddress) -> Result[(), IoError] {
        let fd = self.fd;
        withNativeAddress(address, { (cells) in unsafe native_connect(fd, cells, -1) })
    }

    /// Sends one datagram to the connected peer.
    public mutating func send(buf: Slice[UInt8]) -> Result[Int64, IoError] { sendOn(self.fd, buf) }

    /// Waits for one datagram from the connected peer; returns its size.
    public mutating func receive(mutating into buf: Slice[UInt8]) -> Result[Int64, IoError] {
        receiveOn(self.fd, buf)
    }

    /// The connected peer.
    public func peerAddress() -> Result[SocketAddress, IoError] { addressOf(self.fd, true) }

    /// The local address: the port picked after binding port 0.
    public func localAddress() -> Result[SocketAddress, IoError] { addressOf(self.fd, false) }

    /// Allows sending to a broadcast address.
    public mutating func setBroadcast(on: Bool) -> Result[(), IoError] { setFlag(self.fd, broadcastOption(), on) }

    /// Makes a receive that waits longer than `timeout` fail with
    /// `WouldBlock`. `None` waits forever.
    public mutating func setReadTimeout(timeout: Optional[Duration]) -> Result[(), IoError] {
        setTimeout(self.fd, false, timeout)
    }

    /// Makes a send that waits longer than `timeout` fail with
    /// `WouldBlock`. `None` waits forever.
    public mutating func setWriteTimeout(timeout: Optional[Duration]) -> Result[(), IoError] {
        setTimeout(self.fd, true, timeout)
    }

    /// Makes sends and receives return `WouldBlock` instead of waiting.
    public mutating func setNonblocking(on: Bool) -> Result[(), IoError] { setNonblockingOn(self.fd, on) }

    /// A second handle with its own descriptor (`dup`).
    public func tryClone() -> Result[UdpSocket, IoError] {
        .Ok(UdpSocket(unsafeDescriptor: FileDescriptor(try duplicate(self.fd))))
    }

    /// The descriptor, still owned by this socket.
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
