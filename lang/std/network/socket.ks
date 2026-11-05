// Operations every socket type shares, on a raw descriptor. The public
// types (`TcpStream`, `UdpSocket`, ...) own the descriptor and forward here,
// so each system call is wrapped once.

module std.network

import std.numeric.(Int32, Int64, UInt8)
import std.collections.(Slice)
import std.core.(Bool)
import std.result.(Result, Optional)
import std.memory.(RawPointer)
import std.streams.(IoError)
import std.ffi.(malloc, free)

/// How `shutdown` closes a connection.
public enum Shutdown {
    /// No more reading: reads return end of stream.
    case Read
    /// No more writing: the peer reads end of stream.
    case Write
    /// Both.
    case Both
}

/// Option numbers `kestrel_socket_set_flag` takes.
internal func noDelayOption() -> Int32 { 1 }
internal func keepAliveOption() -> Int32 { 2 }
internal func broadcastOption() -> Int32 { 3 }
internal func reuseAddressOption() -> Int32 { 4 }
internal func v6OnlyOption() -> Int32 { 5 }

/// Socket kinds `kestrel_socket` takes.
internal func streamKind() -> Int32 { 1 }
internal func datagramKind() -> Int32 { 2 }
internal func unixFamily() -> Int32 { 1 }

/// The `kestrel_socket` family for `address`.
internal func familyOf(address: SocketAddress) -> Int32 {
    match address.ip { .V4(_) => 4, .V6(_) => 6 }
}

/// `.Ok(())` for a zero result, the current errno otherwise.
internal func check(result: Int32) -> Result[(), IoError] {
    if result < 0 { .Err(IoError.last()) } else { .Ok(()) }
}

/// Runs `body` with `address` laid out for the shim.
internal func withNativeAddress(address: SocketAddress, body: (RawPointer) -> Int32) -> Result[(), IoError] {
    let cells = malloc(nativeAddressSize());
    address.writeNative(into: cells);
    let r = body(cells);
    let error = IoError.last();
    free(cells);
    if r < 0 { .Err(error) } else { .Ok(()) }
}

/// A new socket for `family` and `kind`, or the error.
internal func openSocket(family: Int32, kind: Int32) -> Result[Int32, IoError] {
    let fd = native_socket(family, kind);
    if fd < 0 { .Err(IoError.last()) } else { .Ok(fd) }
}

/// Closes `fd` and passes `error` on: for a setup step that failed.
internal func closeAfter[T](fd: Int32, error: IoError) -> Result[T, IoError] where T: not Copyable {
    native_close(fd);
    .Err(error)
}

/// A socket of `kind` connected to `address`. A negative timeout blocks.
internal func connectSocket(address: SocketAddress, kind: Int32, timeoutMilliseconds: Int64) -> Result[Int32, IoError] {
    let fd = try openSocket(familyOf(address), kind);
    match withNativeAddress(address, { (cells) in native_connect(fd, cells, timeoutMilliseconds) }) {
        .Ok(_) => .Ok(fd),
        .Err(e) => closeAfter(fd, e)
    }
}

/// A socket of `kind` bound to `address`. A `[::]` address also accepts
/// IPv4, and `reuse` lets a restarted server bind a port in TIME_WAIT.
internal func bindSocket(address: SocketAddress, kind: Int32, reuse: Bool) -> Result[Int32, IoError] {
    let fd = try openSocket(familyOf(address), kind);
    if reuse and native_set_flag(fd, reuseAddressOption(), 1) < 0 {
        return closeAfter(fd, IoError.last())
    }
    if address.ip == IpAddress.V6(Ipv6Address.any) {
        // Dual-stack where the system allows it; IPv6-only otherwise.
        native_set_flag(fd, v6OnlyOption(), 0);
    }
    match withNativeAddress(address, { (cells) in native_bind(fd, cells) }) {
        .Ok(_) => .Ok(fd),
        .Err(e) => closeAfter(fd, e)
    }
}

/// `listen(2)` with the system's largest backlog.
internal func listenOn(fd: Int32) -> Result[(), IoError] {
    check(native_listen(fd, 4096))
}

/// The local or peer address of `fd`.
internal func addressOf(fd: Int32, peer: Bool) -> Result[SocketAddress, IoError] {
    let cells = malloc(nativeAddressSize());
    let r = native_address(fd, if peer { 1 } else { 0 }, cells);
    let error = IoError.last();
    let result = if r < 0 { .Err(error) } else { .Ok(SocketAddress.readNative(from: cells)) };
    free(cells);
    result
}

/// `accept(2)`: the new descriptor and the peer (meaningless for Unix
/// sockets).
internal func acceptOn(fd: Int32) -> Result[(Int32, SocketAddress), IoError] {
    let cells = malloc(nativeAddressSize());
    let client = native_accept(fd, cells);
    let error = IoError.last();
    let result = if client < 0 { .Err(error) } else { .Ok((client, SocketAddress.readNative(from: cells))) };
    free(cells);
    result
}

internal func receiveOn(fd: Int32, buf: Slice[UInt8]) -> Result[Int64, IoError] {
    let n = native_receive(fd, buf.pointer.asRaw(), buf.count);
    if n < 0 { .Err(IoError.last()) } else { .Ok(n) }
}

internal func sendOn(fd: Int32, buf: Slice[UInt8]) -> Result[Int64, IoError] {
    let n = native_send(fd, buf.pointer.asRaw(), buf.count);
    if n < 0 { .Err(IoError.last()) } else { .Ok(n) }
}

internal func shutdownOn(fd: Int32, how: Shutdown) -> Result[(), IoError] {
    check(native_shutdown(fd, match how { .Read => 0, .Write => 1, .Both => 2 }))
}

internal func setFlag(fd: Int32, option: Int32, on: Bool) -> Result[(), IoError] {
    check(native_set_flag(fd, option, if on { 1 } else { 0 }))
}

/// Sets the read (`write` false) or write timeout; `None` blocks.
internal func setTimeout(fd: Int32, write: Bool, milliseconds: Optional[Int64]) -> Result[(), IoError] {
    let ms = match milliseconds {
        .Some(n) => if n < 0 { return .Err(IoError.invalidInput) } else { n },
        .None => -1
    };
    check(native_set_timeout(fd, if write { 1 } else { 0 }, ms))
}

internal func setNonblockingOn(fd: Int32, on: Bool) -> Result[(), IoError] {
    check(native_set_nonblocking(fd, if on { 1 } else { 0 }))
}

/// `dup(2)`, close-on-exec.
internal func duplicate(fd: Int32) -> Result[Int32, IoError] {
    let copy = native_duplicate(fd);
    if copy < 0 { .Err(IoError.last()) } else { .Ok(copy) }
}
