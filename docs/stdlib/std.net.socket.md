# std.net.socket

## struct `TcpListener`

```kestrel
public struct TcpListener { /* private fields */ }
```

A bound, listening TCP server socket.

Created by `TcpListener.bind(port:)` — sets `SO_REUSEADDR`,
binds to `INADDR_ANY:port`, and calls `listen(2)` with backlog
`128`. Accept connections via `accept()`, which blocks until
the next client arrives. The owned fd is closed by the deinit.

### Examples

```
let listener = match TcpListener.bind(port: 8080) {
    .Ok(l) => l,
    .Err(e) => return .Err(e)
};
while true {
    match listener.accept() {
        .Ok(stream) => /* handle stream */ {},
        .Err(e) => break
    }
}
```

### Representation

A single `Int32` field — the listening socket fd.

### Memory Model

Owns its fd; closed on drop.

_Defined in `lang/std/net/socket.ks`._

### Members

#### initializer `From Fd`

```kestrel
init(Int32)
```

Internal — wraps an existing fd. Callers should use `bind(port:)`.

_Defined in `lang/std/net/socket.ks`._

#### function `accept`

```kestrel
public func accept() -> Result[TcpStream, IoError]
```

Blocks until the next client connects, then returns it as a `TcpStream`.

Discards the client's address — pass non-null pointers to
`libc.accept` directly if you need it. Each accepted
connection has its own fd, independent of the listener.

##### Errors

Returns `Err(IoError.last())` if `accept(2)` fails — common
causes include `EINTR` (interrupted by signal) and
`EMFILE` (per-process fd limit).

_Defined in `lang/std/net/socket.ks`._

#### function `bind`

```kestrel
public static func bind(UInt16) -> Result[TcpListener, IoError]
```

Creates a server socket bound to `0.0.0.0:port` with `SO_REUSEADDR` and a backlog of 128.

Walks the full setup — `socket` → `setsockopt` → `bind` →
`listen` — and cleans up the partial fd on any failure.

##### Errors

Returns `Err(IoError.last())` (captured `errno`) at any of the
four steps; the most common case is `EADDRINUSE` if another
process holds the port and `SO_REUSEADDR` is not enough.

##### Examples

```
let listener = TcpListener.bind(port: 8080);
```

_Defined in `lang/std/net/socket.ks`._

#### field `fd`

```kestrel
var fd: Int32
```

_Defined in `lang/std/net/socket.ks`._

#### function `rawFd`

```kestrel
public func rawFd() -> Int32
```

Returns the underlying listening fd without giving up ownership.

_Defined in `lang/std/net/socket.ks`._

## struct `TcpStream`

```kestrel
public struct TcpStream { /* private fields */ }
```

A connected TCP byte stream — implements `Read` and `Write` on top of a POSIX socket fd.

Returned by `TcpListener.accept()` (server side) and
`TcpStream.connect(host:port:)` (client side). Reads and writes
go directly through `recv(2)` / `send(2)`; partial reads/writes
are surfaced — the caller is responsible for looping. The owned
fd is closed automatically by the deinit unless `detachFd` has
been called first.

### Examples

```
var stream = match TcpStream.connect(host: "example.com", port: 80) {
    .Ok(s) => s,
    .Err(e) => return .Err(e)
};
// stream is Read + Write
```

### Representation

A single `Int32` field holding the file descriptor; `-1` means
"detached, do not close on drop".

### Memory Model

Owns its fd. Cloning is not provided — duplicate explicitly via
`dup(2)` if you need it.

_Defined in `lang/std/net/socket.ks`._

### Members

#### initializer `From Fd`

```kestrel
public init(Int32)
```

Wraps an existing socket fd as a `TcpStream`.

The stream takes ownership; the deinit will close the fd.
Callers obtaining the fd from `accept` / `socket` should
hand it over and stop using it directly.

_Defined in `lang/std/net/socket.ks`._

#### function `connect`

```kestrel
public static func connect(String, UInt16) -> Result[TcpStream, IoError]
```

Resolves `host`:`port` and returns a connected `TcpStream`.

Uses `getaddrinfo` for resolution and tries the first result.
Constrained to IPv4 / TCP via the `hints` block. On any
failure the partially-built fd is closed and the resolver
list is freed before returning. Does not currently fall
through to the next `addrinfo` entry on a failed
`connect` — try one address.

##### Errors

- Returns `Err(IoError(code: eai))` with the `EAI_*` resolver code if
  `getaddrinfo` fails (note: this is a libc resolver code,
  not an `errno`).
- Returns `Err(IoError.last())` from `errno` if `socket()` or
  `connect()` fail.

##### Examples

```
match TcpStream.connect(host: "example.com", port: 80) {
    .Ok(stream) => /* use stream */ {},
    .Err(e) => print(e.message)
}
```

_Defined in `lang/std/net/socket.ks`._

#### function `detachFd`

```kestrel
public mutating func detachFd() -> Int32
```

Releases ownership of the fd and returns it.

Sets the internal fd to `-1` so the deinit becomes a no-op.
The caller takes responsibility for closing the returned fd.
Use this when handing the fd to another owner (e.g. an event
loop or a child process).

_Defined in `lang/std/net/socket.ks`._

#### field `fd`

```kestrel
var fd: Int32
```

_Defined in `lang/std/net/socket.ks`._

#### function `rawFd`

```kestrel
public func rawFd() -> Int32
```

Returns the underlying fd without giving up ownership.

Useful for passing the fd to syscalls that the wrapper does
not expose (`fcntl`, `setsockopt`, …). Do not close it
yourself — the deinit still will.

_Defined in `lang/std/net/socket.ks`._

### Implements `Read`

#### function `read`

```kestrel
public mutating func read(into: Slice[UInt8]) -> Result[Int64, IoError]
```

Reads up to `buf.count` bytes into `buf`. Returns the byte count actually read.

`0` indicates the peer closed the connection cleanly. Required
by the `Read` protocol.

##### Errors

Returns `Err(IoError)` from the captured `errno` if `recv`
returns `-1`.

_Defined in `lang/std/net/socket.ks`._

### Implements `Write`

#### function `flush`

```kestrel
public mutating func flush() -> Result[(), IoError]
```

No-op — TCP sockets do not have an application-level write buffer.

Always returns `Ok(())`. Provided to satisfy the `Write`
protocol so generic writers can call `flush` unconditionally.

_Defined in `lang/std/net/socket.ks`._

#### function `write`

```kestrel
public mutating func write(from: Slice[UInt8]) -> Result[Int64, IoError]
```

Writes up to `buf.count` bytes from `buf`. Returns the byte count actually written.

May write fewer bytes than requested under back-pressure;
loop until the buffer is drained. Required by the `Write`
protocol.

##### Errors

Returns `Err(IoError)` from the captured `errno` if `send`
returns `-1`.

_Defined in `lang/std/net/socket.ks`._

