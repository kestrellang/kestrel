# std.network

## struct `Incoming`

```kestrel
public struct Incoming: not Copyable { /* private fields */ }
```

The connections a `TcpListener` accepts, endlessly. Each is a
`Result[TcpStream, IoError]`; an error doesn't end the iteration.

_Defined in `lang/std/network/tcp.ks`._

### Members

#### field `listener`

```kestrel
var listener: TcpListener
```

_Defined in `lang/std/network/tcp.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = Result[TcpStream, IoError]
```

_Defined in `lang/std/network/tcp.ks`._

#### function `next`

```kestrel
public mutating func next() -> Optional[Result[TcpStream, IoError]]
```

Waits for the next connection.

_Defined in `lang/std/network/tcp.ks`._

## enum `IpAddress`

```kestrel
public enum IpAddress
```

An IPv4 or IPv6 address.

### Examples

```
let ip = IpAddress.parse("::1").unwrap();
assert(ip.isLoopback);
```

_Defined in `lang/std/network/address.ks`._

### Members

#### case `V4`

```kestrel
case V4(Ipv4Address)
```

_Defined in `lang/std/network/address.ks`._

#### case `V6`

```kestrel
case V6(Ipv6Address)
```

_Defined in `lang/std/network/address.ks`._

#### field `isLoopback`

```kestrel
public var isLoopback: Bool { get }
```

`127.0.0.0/8` or `::1`.

_Defined in `lang/std/network/address.ks`._

#### field `isUnspecified`

```kestrel
public var isUnspecified: Bool { get }
```

`0.0.0.0` or `::`.

_Defined in `lang/std/network/address.ks`._

#### function `parse`

```kestrel
public static func parse(String) -> Optional[IpAddress]
```

Parses dotted IPv4 (`127.0.0.1`) or IPv6 (`::1`, `fe80::1%3`).

_Defined in `lang/std/network/address.ks`._

### Implements `Equatable`

#### function `isEqual`

```kestrel
public func isEqual(to: IpAddress) -> Bool
```

Same family and address.

_Defined in `lang/std/network/address.ks`._

### Implements `Hashable`

#### function `hash`

```kestrel
public func hash[H](into: mutating H) where H: Hasher
```

Hashes the family and address.

_Defined in `lang/std/network/address.ks`._

### Implements `Formattable`

#### function `format`

```kestrel
public func format(into: mutating some Formatter, FormatOptions)
```

The address in its usual text form.

_Defined in `lang/std/network/address.ks`._

## struct `Ipv4Address`

```kestrel
public struct Ipv4Address { /* private fields */ }
```

An IPv4 address, such as `192.168.1.1`.

_Defined in `lang/std/network/address.ks`._

### Members

#### initializer `From Bits`

```kestrel
public init(bits: UInt32)
```

The address whose big-endian number is `bits`.

_Defined in `lang/std/network/address.ks`._

#### initializer `From Octets`

```kestrel
public init(UInt8, UInt8, UInt8, UInt8)
```

The address `a.b.c.d`.

_Defined in `lang/std/network/address.ks`._

#### field `any`

```kestrel
public static var any: Ipv4Address { get }
```

`0.0.0.0`: every local address, when binding.

_Defined in `lang/std/network/address.ks`._

#### field `bits`

```kestrel
public var bits: UInt32
```

The address as one big-endian number: `127.0.0.1` is `0x7f000001`.

_Defined in `lang/std/network/address.ks`._

#### field `isLoopback`

```kestrel
public var isLoopback: Bool { get }
```

`127.0.0.0/8`.

_Defined in `lang/std/network/address.ks`._

#### field `isUnspecified`

```kestrel
public var isUnspecified: Bool { get }
```

`0.0.0.0`.

_Defined in `lang/std/network/address.ks`._

#### field `localhost`

```kestrel
public static var localhost: Ipv4Address { get }
```

`127.0.0.1`.

_Defined in `lang/std/network/address.ks`._

#### function `octet`

```kestrel
func octet(Int64) -> UInt8
```

_Defined in `lang/std/network/address.ks`._

#### field `octets`

```kestrel
public var octets: (UInt8, UInt8, UInt8, UInt8) { get }
```

The four octets, most significant first.

_Defined in `lang/std/network/address.ks`._

#### function `parse`

```kestrel
public static func parse(String) -> Optional[Ipv4Address]
```

Parses four decimal octets, `a.b.c.d`, with no leading zeros.

_Defined in `lang/std/network/address.ks`._

### Implements `Equatable`

#### function `isEqual`

```kestrel
public func isEqual(to: Ipv4Address) -> Bool
```

Same address.

_Defined in `lang/std/network/address.ks`._

### Implements `Hashable`

#### function `hash`

```kestrel
public func hash[H](into: mutating H) where H: Hasher
```

Hashes the address.

_Defined in `lang/std/network/address.ks`._

### Implements `Formattable`

#### function `format`

```kestrel
public func format(into: mutating some Formatter, FormatOptions)
```

`a.b.c.d`.

_Defined in `lang/std/network/address.ks`._

## struct `Ipv6Address`

```kestrel
public struct Ipv6Address { /* private fields */ }
```

An IPv6 address, such as `2001:db8::1`.

_Defined in `lang/std/network/address.ks`._

### Members

#### initializer `From Halves`

```kestrel
public init(high: UInt64, low: UInt64, scopeId: UInt32)
```

The address whose first and last 64 bits are `high` and `low`.

_Defined in `lang/std/network/address.ks`._

#### initializer `From Segments`

```kestrel
public init(segments: (UInt16, UInt16, UInt16, UInt16, UInt16, UInt16, UInt16, UInt16))
```

The address of eight 16-bit segments, most significant first.

_Defined in `lang/std/network/address.ks`._

#### field `any`

```kestrel
public static var any: Ipv6Address { get }
```

`::`: every local address, when binding.

_Defined in `lang/std/network/address.ks`._

#### field `high`

```kestrel
var high: UInt64
```

The first 64 bits, big-endian.

_Defined in `lang/std/network/address.ks`._

#### field `isLoopback`

```kestrel
public var isLoopback: Bool { get }
```

`::1`.

_Defined in `lang/std/network/address.ks`._

#### field `isUnspecified`

```kestrel
public var isUnspecified: Bool { get }
```

`::`.

_Defined in `lang/std/network/address.ks`._

#### field `localhost`

```kestrel
public static var localhost: Ipv6Address { get }
```

`::1`.

_Defined in `lang/std/network/address.ks`._

#### field `low`

```kestrel
var low: UInt64
```

The last 64 bits, big-endian.

_Defined in `lang/std/network/address.ks`._

#### function `parse`

```kestrel
public static func parse(String) -> Optional[Ipv6Address]
```

Parses RFC 4291 text: up to eight hex segments, one `::` for a run
of zeros, an optional trailing dotted IPv4 (`::ffff:1.2.3.4`) and an
optional numeric zone (`%3`).

_Defined in `lang/std/network/address.ks`._

#### field `scopeId`

```kestrel
public var scopeId: UInt32
```

The interface for a link-local address (`fe80::1%3`), or 0.

_Defined in `lang/std/network/address.ks`._

#### function `segment`

```kestrel
public func segment(Int64) -> UInt16
```

The segment at `index` (0 to 7).

_Defined in `lang/std/network/address.ks`._

#### field `segments`

```kestrel
public var segments: (UInt16, UInt16, UInt16, UInt16, UInt16, UInt16, UInt16, UInt16) { get }
```

The eight segments, most significant first.

_Defined in `lang/std/network/address.ks`._

### Implements `Equatable`

#### function `isEqual`

```kestrel
public func isEqual(to: Ipv6Address) -> Bool
```

Same address and zone.

_Defined in `lang/std/network/address.ks`._

### Implements `Hashable`

#### function `hash`

```kestrel
public func hash[H](into: mutating H) where H: Hasher
```

Hashes the address and zone.

_Defined in `lang/std/network/address.ks`._

### Implements `Formattable`

#### function `format`

```kestrel
public func format(into: mutating some Formatter, FormatOptions)
```

RFC 5952 text: lowercase hex, the longest run of two or more zero
segments as `::`, and `%zone` when set.

_Defined in `lang/std/network/address.ks`._

## enum `Shutdown`

```kestrel
public enum Shutdown
```

How `shutdown` closes a connection.

_Defined in `lang/std/network/socket.ks`._

### Members

#### case `Both`

```kestrel
case Both
```

Both.

_Defined in `lang/std/network/socket.ks`._

#### case `Read`

```kestrel
case Read
```

No more reading: reads return end of stream.

_Defined in `lang/std/network/socket.ks`._

#### case `Write`

```kestrel
case Write
```

No more writing: the peer reads end of stream.

_Defined in `lang/std/network/socket.ks`._

## struct `SocketAddress`

```kestrel
public struct SocketAddress { /* private fields */ }
```

An IP address and port, such as `127.0.0.1:8080` or `[::1]:8080`.

_Defined in `lang/std/network/address.ks`._

### Members

#### initializer `From IP`

```kestrel
public init(IpAddress, port: UInt16)
```

`ip` at `port`.

_Defined in `lang/std/network/address.ks`._

#### field `ip`

```kestrel
public var ip: IpAddress
```

The IP address.

_Defined in `lang/std/network/address.ks`._

#### function `parse`

```kestrel
public static func parse(String) -> Optional[SocketAddress]
```

Parses `1.2.3.4:80` or `[::1]:80`.

_Defined in `lang/std/network/address.ks`._

#### field `port`

```kestrel
public var port: UInt16
```

The port.

_Defined in `lang/std/network/address.ks`._

#### function `readNative`

```kestrel
static func readNative(from: RawPointer) -> SocketAddress
```

Reads a `kestrel_address` the shim wrote.

_Defined in `lang/std/network/address.ks`._

#### function `writeNative`

```kestrel
func writeNative(into: RawPointer)
```

Writes the address into a 40-byte `kestrel_address` for the shim.

_Defined in `lang/std/network/address.ks`._

### Implements `Equatable`

#### function `isEqual`

```kestrel
public func isEqual(to: SocketAddress) -> Bool
```

Same address and port.

_Defined in `lang/std/network/address.ks`._

### Implements `Hashable`

#### function `hash`

```kestrel
public func hash[H](into: mutating H) where H: Hasher
```

Hashes the address and port.

_Defined in `lang/std/network/address.ks`._

### Implements `Formattable`

#### function `format`

```kestrel
public func format(into: mutating some Formatter, FormatOptions)
```

`a.b.c.d:port` or `[v6]:port`.

_Defined in `lang/std/network/address.ks`._

## struct `TcpListener`

```kestrel
public struct TcpListener: not Copyable { /* private fields */ }
```

A TCP server socket. Closed when dropped.

### Examples

```
let listener = try TcpListener.bind(8080);
loop {
    var (client, peer) = try listener.accept();
    try client.write(line: "hello, \(peer)");
}
```

_Defined in `lang/std/network/tcp.ks`._

### Members

#### function `accept`

```kestrel
public func accept() -> Result[(TcpStream, SocketAddress), IoError]
```

Waits for a connection; returns it and the peer's address.

_Defined in `lang/std/network/tcp.ks`._

#### function `bind`

```kestrel
public static func bind(to: SocketAddress) -> Result[TcpListener, IoError]
```

Listens on `address`. Port 0 picks a free port; `localAddress()`
says which.

_Defined in `lang/std/network/tcp.ks`._

#### function `bind`

```kestrel
public static func bind(UInt16) -> Result[TcpListener, IoError]
```

Listens on `port` on this machine only (`[::1]`, or `127.0.0.1`
where IPv6 is unavailable), so a dev server isn't reachable from the
network by accident. Use `bind(anyInterface:)` to accept outside
connections.

_Defined in `lang/std/network/tcp.ks`._

#### function `bind`

```kestrel
public static func bind(anyInterface: UInt16) -> Result[TcpListener, IoError]
```

Listens on `port` on every local address, IPv6 and IPv4 (`[::]`,
or `0.0.0.0` where IPv6 is unavailable). Reachable from the network.

_Defined in `lang/std/network/tcp.ks`._

#### function `bindDualStack`

```kestrel
static func bindDualStack(IpAddress, IpAddress, UInt16) -> Result[TcpListener, IoError]
```

Binds `v6`, falling back to `v4` only when IPv6 itself is missing.

_Defined in `lang/std/network/tcp.ks`._

#### field `fd`

```kestrel
var fd: Int32
```

_Defined in `lang/std/network/tcp.ks`._

#### function `incoming`

```kestrel
public consuming func incoming() -> Incoming
```

The connections, as they arrive. Takes the listener, which is
closed when the iterator is dropped.

_Defined in `lang/std/network/tcp.ks`._

#### function `localAddress`

```kestrel
public func localAddress() -> Result[SocketAddress, IoError]
```

The address it listens on: the port picked after binding port 0.

_Defined in `lang/std/network/tcp.ks`._

#### field `rawDescriptor`

```kestrel
public var rawDescriptor: FileDescriptor { get }
```

The descriptor, still owned by this listener.

_Defined in `lang/std/network/tcp.ks`._

#### function `setNonblocking`

```kestrel
public mutating func setNonblocking(Bool) -> Result[(), IoError]
```

Makes `accept` return `WouldBlock` instead of waiting.

_Defined in `lang/std/network/tcp.ks`._

## struct `TcpStream`

```kestrel
public struct TcpStream: not Copyable { /* private fields */ }
```

A TCP connection. Closed when dropped.

It is `Readable` and `Writable`, so every stream method works on it.
Writing after the peer has closed is a `BrokenPipe` error, never a
signal.

### Examples

```
var stream = try TcpStream.connect("example.com", 80);
try stream.write(text: "GET / HTTP/1.0\r\nHost: example.com\r\n\r\n");
let page = try stream.readText();
```

_Defined in `lang/std/network/tcp.ks`._

### Members

#### initializer `From Descriptor`

```kestrel
public init(unsafeDescriptor: FileDescriptor)
```

Takes ownership of a connected socket: the stream closes it.

_Defined in `lang/std/network/tcp.ks`._

#### function `close`

```kestrel
public consuming func close() -> Result[(), IoError]
```

Closes the socket and reports an error from `close`, which dropping
it can't.

_Defined in `lang/std/network/tcp.ks`._

#### function `connect`

```kestrel
public static func connect(String, UInt16) -> Result[TcpStream, IoError]
```

Resolves `host` and tries each address in turn until one connects.

##### Errors

`HostNotFound` if `host` doesn't resolve, otherwise the last
address's error, such as `ConnectionRefused`.

_Defined in `lang/std/network/tcp.ks`._

#### function `connect`

```kestrel
public static func connect(String, UInt16, timeout: Duration) -> Result[TcpStream, IoError]
```

Like `connect(host, port)`, failing with `TimedOut` if no address
connects within `timeout`, lookup included. A negative timeout is
`InvalidInput`.

_Defined in `lang/std/network/tcp.ks`._

#### function `connect`

```kestrel
public static func connect(to: SocketAddress) -> Result[TcpStream, IoError]
```

Connects to `address`.

_Defined in `lang/std/network/tcp.ks`._

#### function `connect`

```kestrel
public static func connect(to: SocketAddress, timeout: Duration) -> Result[TcpStream, IoError]
```

Connects to `address`, failing with `TimedOut` after `timeout`. A
negative timeout is `InvalidInput`.

_Defined in `lang/std/network/tcp.ks`._

#### field `fd`

```kestrel
var fd: Int32
```

_Defined in `lang/std/network/tcp.ks`._

#### function `intoRawDescriptor`

```kestrel
public consuming func intoRawDescriptor() -> FileDescriptor
```

Gives up ownership: the descriptor stays open and the caller closes
it, as when handing the connection to a TLS library.

_Defined in `lang/std/network/tcp.ks`._

#### function `localAddress`

```kestrel
public func localAddress() -> Result[SocketAddress, IoError]
```

The local end's address.

_Defined in `lang/std/network/tcp.ks`._

#### function `peerAddress`

```kestrel
public func peerAddress() -> Result[SocketAddress, IoError]
```

The address of the other end.

_Defined in `lang/std/network/tcp.ks`._

#### field `rawDescriptor`

```kestrel
public var rawDescriptor: FileDescriptor { get }
```

The descriptor, still owned by this stream.

_Defined in `lang/std/network/tcp.ks`._

#### function `setKeepAlive`

```kestrel
public mutating func setKeepAlive(Bool) -> Result[(), IoError]
```

Turns on TCP keepalive probes on an idle connection.

_Defined in `lang/std/network/tcp.ks`._

#### function `setNoDelay`

```kestrel
public mutating func setNoDelay(Bool) -> Result[(), IoError]
```

Turns off Nagle's algorithm, so small writes go out at once.

_Defined in `lang/std/network/tcp.ks`._

#### function `setNonblocking`

```kestrel
public mutating func setNonblocking(Bool) -> Result[(), IoError]
```

Makes reads and writes return `WouldBlock` instead of waiting, for
an event loop polling `rawDescriptor`.

_Defined in `lang/std/network/tcp.ks`._

#### function `setReadTimeout`

```kestrel
public mutating func setReadTimeout(Optional[Duration]) -> Result[(), IoError]
```

Makes a read that waits longer than `timeout` fail with
`WouldBlock`. `None` waits forever.

_Defined in `lang/std/network/tcp.ks`._

#### function `setWriteTimeout`

```kestrel
public mutating func setWriteTimeout(Optional[Duration]) -> Result[(), IoError]
```

Makes a write that waits longer than `timeout` fail with
`WouldBlock`. `None` waits forever.

_Defined in `lang/std/network/tcp.ks`._

#### function `shutdown`

```kestrel
public mutating func shutdown(Shutdown) -> Result[(), IoError]
```

Closes the reading side, the writing side, or both. After
`shutdown(.Write)` the peer reads end of stream, and this side can
still read its reply.

_Defined in `lang/std/network/tcp.ks`._

#### function `tryClone`

```kestrel
public func tryClone() -> Result[TcpStream, IoError]
```

A second handle to the same connection with its own descriptor
(`dup`). Each closes its own descriptor.

_Defined in `lang/std/network/tcp.ks`._

### Implements `Readable`

#### function `read`

```kestrel
public mutating func read(into: mutating Slice[UInt8]) -> Result[Int64, IoError]
```

Reads what has arrived, waiting for at least one byte; `0` once the
peer has closed its side.

_Defined in `lang/std/network/tcp.ks`._

### Implements `Writable`

#### function `flush`

```kestrel
public mutating func flush() -> Result[(), IoError]
```

Nothing to flush: the stream doesn't buffer. Use `setNoDelay` to
send small writes at once.

_Defined in `lang/std/network/tcp.ks`._

#### function `write`

```kestrel
public mutating func write(from: Slice[UInt8]) -> Result[Int64, IoError]
```

Sends up to `buf.count` bytes; returns how many were taken.

_Defined in `lang/std/network/tcp.ks`._

## struct `UdpSocket`

```kestrel
public struct UdpSocket: not Copyable { /* private fields */ }
```

A UDP socket. Closed when dropped.

### Examples

```
var socket = try UdpSocket.bind(to: SocketAddress.parse("127.0.0.1:0").unwrap());
try socket.send("ping".asByteSlice(), to: server);
var buf = Array[UInt8](repeating: 0, count: 1500);
let (n, from) = try socket.receiveFrom(into: buf.asMutableSlice());
```

_Defined in `lang/std/network/udp.ks`._

### Members

#### initializer `From Descriptor`

```kestrel
public init(unsafeDescriptor: FileDescriptor)
```

Takes ownership of a UDP socket: this value closes it.

_Defined in `lang/std/network/udp.ks`._

#### function `bind`

```kestrel
public static func bind(to: SocketAddress) -> Result[UdpSocket, IoError]
```

A socket bound to `address`. Port 0 picks a free port.

_Defined in `lang/std/network/udp.ks`._

#### function `close`

```kestrel
public consuming func close() -> Result[(), IoError]
```

Closes the socket and reports an error from `close`, which dropping
it can't.

_Defined in `lang/std/network/udp.ks`._

#### function `connect`

```kestrel
public mutating func connect(to: SocketAddress) -> Result[(), IoError]
```

Fixes the peer: `send(buf)` goes to `address`, and only its
datagrams are received.

_Defined in `lang/std/network/udp.ks`._

#### field `fd`

```kestrel
var fd: Int32
```

_Defined in `lang/std/network/udp.ks`._

#### function `intoRawDescriptor`

```kestrel
public consuming func intoRawDescriptor() -> FileDescriptor
```

Gives up ownership: the descriptor stays open and the caller closes
it.

_Defined in `lang/std/network/udp.ks`._

#### function `localAddress`

```kestrel
public func localAddress() -> Result[SocketAddress, IoError]
```

The local address: the port picked after binding port 0.

_Defined in `lang/std/network/udp.ks`._

#### function `peerAddress`

```kestrel
public func peerAddress() -> Result[SocketAddress, IoError]
```

The connected peer.

_Defined in `lang/std/network/udp.ks`._

#### field `rawDescriptor`

```kestrel
public var rawDescriptor: FileDescriptor { get }
```

The descriptor, still owned by this socket.

_Defined in `lang/std/network/udp.ks`._

#### function `receive`

```kestrel
public mutating func receive(into: mutating Slice[UInt8]) -> Result[Int64, IoError]
```

Waits for one datagram from the connected peer; returns its size.

_Defined in `lang/std/network/udp.ks`._

#### function `receiveFrom`

```kestrel
public mutating func receiveFrom(into: mutating Slice[UInt8]) -> Result[(Int64, SocketAddress), IoError]
```

Waits for one datagram; returns its size and sender. A datagram
longer than `buf` is cut short.

_Defined in `lang/std/network/udp.ks`._

#### function `send`

```kestrel
public mutating func send(Slice[UInt8], to: SocketAddress) -> Result[Int64, IoError]
```

Sends one datagram of `buf` to `address`; returns the bytes sent.

_Defined in `lang/std/network/udp.ks`._

#### function `send`

```kestrel
public mutating func send(Slice[UInt8]) -> Result[Int64, IoError]
```

Sends one datagram to the connected peer.

_Defined in `lang/std/network/udp.ks`._

#### function `setBroadcast`

```kestrel
public mutating func setBroadcast(Bool) -> Result[(), IoError]
```

Allows sending to a broadcast address.

_Defined in `lang/std/network/udp.ks`._

#### function `setNonblocking`

```kestrel
public mutating func setNonblocking(Bool) -> Result[(), IoError]
```

Makes sends and receives return `WouldBlock` instead of waiting.

_Defined in `lang/std/network/udp.ks`._

#### function `setReadTimeout`

```kestrel
public mutating func setReadTimeout(Optional[Duration]) -> Result[(), IoError]
```

Makes a receive that waits longer than `timeout` fail with
`WouldBlock`. `None` waits forever.

_Defined in `lang/std/network/udp.ks`._

#### function `setWriteTimeout`

```kestrel
public mutating func setWriteTimeout(Optional[Duration]) -> Result[(), IoError]
```

Makes a send that waits longer than `timeout` fail with
`WouldBlock`. `None` waits forever.

_Defined in `lang/std/network/udp.ks`._

#### function `tryClone`

```kestrel
public func tryClone() -> Result[UdpSocket, IoError]
```

A second handle with its own descriptor (`dup`).

_Defined in `lang/std/network/udp.ks`._

## struct `UnixListener`

```kestrel
public struct UnixListener: not Copyable { /* private fields */ }
```

A Unix stream server socket. Closed when dropped; the socket file stays
until removed.

_Defined in `lang/std/network/unix.ks`._

### Members

#### function `accept`

```kestrel
public func accept() -> Result[UnixStream, IoError]
```

Waits for a connection.

_Defined in `lang/std/network/unix.ks`._

#### function `bind`

```kestrel
public static func bind(Path) -> Result[UnixListener, IoError]
```

Listens on a new socket file at `path`, which must not exist.

_Defined in `lang/std/network/unix.ks`._

#### field `fd`

```kestrel
var fd: Int32
```

_Defined in `lang/std/network/unix.ks`._

#### field `rawDescriptor`

```kestrel
public var rawDescriptor: FileDescriptor { get }
```

The descriptor, still owned by this listener.

_Defined in `lang/std/network/unix.ks`._

#### function `setNonblocking`

```kestrel
public mutating func setNonblocking(Bool) -> Result[(), IoError]
```

Makes `accept` return `WouldBlock` instead of waiting.

_Defined in `lang/std/network/unix.ks`._

## struct `UnixStream`

```kestrel
public struct UnixStream: not Copyable { /* private fields */ }
```

A connected Unix stream socket. Closed when dropped.

### Examples

```
var (a, b) = try UnixStream.pair();
try a.write(line: "hello");
let line = try b.buffered().readLine();
```

_Defined in `lang/std/network/unix.ks`._

### Members

#### initializer `From Descriptor`

```kestrel
public init(unsafeDescriptor: FileDescriptor)
```

Takes ownership of a connected socket: the stream closes it.

_Defined in `lang/std/network/unix.ks`._

#### function `close`

```kestrel
public consuming func close() -> Result[(), IoError]
```

Closes the socket and reports an error from `close`, which dropping
it can't.

_Defined in `lang/std/network/unix.ks`._

#### function `connect`

```kestrel
public static func connect(Path) -> Result[UnixStream, IoError]
```

Connects to the socket at `path`.

_Defined in `lang/std/network/unix.ks`._

#### field `fd`

```kestrel
var fd: Int32
```

_Defined in `lang/std/network/unix.ks`._

#### function `intoRawDescriptor`

```kestrel
public consuming func intoRawDescriptor() -> FileDescriptor
```

Gives up ownership: the descriptor stays open and the caller closes
it.

_Defined in `lang/std/network/unix.ks`._

#### function `pair`

```kestrel
public static func pair() -> Result[(UnixStream, UnixStream), IoError]
```

Two streams connected to each other (`socketpair`).

_Defined in `lang/std/network/unix.ks`._

#### field `rawDescriptor`

```kestrel
public var rawDescriptor: FileDescriptor { get }
```

The descriptor, still owned by this stream.

_Defined in `lang/std/network/unix.ks`._

#### function `setNonblocking`

```kestrel
public mutating func setNonblocking(Bool) -> Result[(), IoError]
```

Makes reads and writes return `WouldBlock` instead of waiting.

_Defined in `lang/std/network/unix.ks`._

#### function `setReadTimeout`

```kestrel
public mutating func setReadTimeout(Optional[Duration]) -> Result[(), IoError]
```

Makes a read that waits longer than `timeout` fail with
`WouldBlock`. `None` waits forever.

_Defined in `lang/std/network/unix.ks`._

#### function `setWriteTimeout`

```kestrel
public mutating func setWriteTimeout(Optional[Duration]) -> Result[(), IoError]
```

Makes a write that waits longer than `timeout` fail with
`WouldBlock`. `None` waits forever.

_Defined in `lang/std/network/unix.ks`._

#### function `shutdown`

```kestrel
public mutating func shutdown(Shutdown) -> Result[(), IoError]
```

Closes the reading side, the writing side, or both.

_Defined in `lang/std/network/unix.ks`._

#### function `tryClone`

```kestrel
public func tryClone() -> Result[UnixStream, IoError]
```

A second handle with its own descriptor (`dup`).

_Defined in `lang/std/network/unix.ks`._

### Implements `Readable`

#### function `read`

```kestrel
public mutating func read(into: mutating Slice[UInt8]) -> Result[Int64, IoError]
```

Reads what has arrived, waiting for at least one byte; `0` once the
peer has closed its side.

_Defined in `lang/std/network/unix.ks`._

### Implements `Writable`

#### function `flush`

```kestrel
public mutating func flush() -> Result[(), IoError]
```

Nothing to flush: the stream doesn't buffer.

_Defined in `lang/std/network/unix.ks`._

#### function `write`

```kestrel
public mutating func write(from: Slice[UInt8]) -> Result[Int64, IoError]
```

Sends up to `buf.count` bytes; returns how many were taken.

_Defined in `lang/std/network/unix.ks`._

## function `lookup`

```kestrel
public func lookup(String, UInt16) -> Result[Array[SocketAddress], IoError]
```

Resolves `host` (a name or an address literal) to the addresses to
try for `port`, IPv6 and IPv4 interleaved in the system's preferred
order.

### Errors

`HostNotFound` when the name doesn't resolve, `Other` for any other
resolver failure.

### Examples

```
for address in try lookup("example.com", 443) {
    println(address);
}
```

_Defined in `lang/std/network/lookup.ks`._

