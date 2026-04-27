# std.net.libc

## function `AF_INET`

```kestrel
public func AF_INET() -> Int32
```

`AF_INET` — the IPv4 address family.

Pass to `socket()` as the `domain` argument when opening an IPv4
socket. The numeric value is `2` on every POSIX platform we
target.

_Defined in `lang/std/net/libc.ks`._

## function `INADDR_ANY`

```kestrel
public func INADDR_ANY() -> Int32
```

`INADDR_ANY` — wildcard IPv4 address (`0.0.0.0`).

Use as the `sin_addr` field of a `sockaddr_in` to bind a server
socket to every local interface.

_Defined in `lang/std/net/libc.ks`._

## function `IPPROTO_TCP`

```kestrel
public func IPPROTO_TCP() -> Int32
```

`IPPROTO_TCP` — the TCP transport protocol number (`6`).

Pass to `socket()` as the `proto` argument when opening a TCP
socket.

_Defined in `lang/std/net/libc.ks`._

## function `SOCKADDR_IN_SIZE`

```kestrel
public func SOCKADDR_IN_SIZE() -> Int32
```

Size in bytes of `struct sockaddr_in` (`16`).

Pass as the `addrlen` argument to `bind()` / `connect()` /
`accept()` when working with IPv4 addresses.

_Defined in `lang/std/net/libc.ks`._

## function `SOCK_STREAM`

```kestrel
public func SOCK_STREAM() -> Int32
```

`SOCK_STREAM` — connection-oriented byte stream (TCP).

Pass to `socket()` as the `type_` argument. Pair with
`IPPROTO_TCP` for explicit TCP, or `0` to let the kernel pick the
default protocol for the family/type pair.

_Defined in `lang/std/net/libc.ks`._

## function `SOL_SOCKET`

```kestrel
public func SOL_SOCKET() -> Int32
```

_Defined in `lang/std/net/libc.ks`._

## function `SO_REUSEADDR`

```kestrel
public func SO_REUSEADDR() -> Int32
```

`SO_REUSEADDR` — allow rebinding to an address still in `TIME_WAIT`.

Pass as the `optname` argument of `setsockopt()` with `SOL_SOCKET`
to let a server bind to a recently-used port without waiting for
the kernel's grace period. Numeric value differs across
platforms.

_Defined in `lang/std/net/libc.ks`._

## function `accept`

```kestrel
public func accept(Int32, Pointer[UInt8], Pointer[Int32]) -> Int32
```

Wraps `accept(2)` — pulls the next connection off `sockfd`'s queue.

Blocks until a connection arrives. Returns the new connected fd
on success, `-1` on error. Pass null pointers for `addr` and
`addrlen` if you don't need the client's address.

_Defined in `lang/std/net/libc.ks`._

## function `bind`

```kestrel
public func bind(Int32, Pointer[UInt8], Int32) -> Int32
```

Wraps `bind(2)` — assigns a local address to `sockfd`.

`addr` points at a packed `sockaddr_in` (or other family-
appropriate layout) of length `addrlen`. Returns `0` on success,
`-1` on error.

_Defined in `lang/std/net/libc.ks`._

## function `close`

```kestrel
public func close(Int32) -> Int32
```

Wraps `close(2)` — releases a file descriptor.

Safe to call on any fd (sockets, files, pipes). Returns `0` on
success, `-1` on error. After `close`, `fd` becomes available for
reuse by the kernel — do not use the value again.

_Defined in `lang/std/net/libc.ks`._

## function `connect`

```kestrel
public func connect(Int32, Pointer[UInt8], Int32) -> Int32
```

Wraps `connect(2)` — initiates a connection on `sockfd` to the address at `addr`.

Blocks until the handshake completes (for connection-oriented
sockets). Returns `0` on success, `-1` on error.

_Defined in `lang/std/net/libc.ks`._

## function `errno`

```kestrel
public func errno() -> Int32
```

Returns the current value of `errno` for the calling thread.

Read it immediately after a failing libc call — any subsequent
syscall (including the success of an unrelated one) may overwrite
it. Implementation forwards to the platform-specific accessor:
`__error` on darwin, `__errno_location` on linux.

_Defined in `lang/std/net/libc.ks`._

## function `freeaddrinfo`

```kestrel
public func freeaddrinfo(Pointer[UInt8])
```

Wraps `freeaddrinfo(3)` — frees the linked list returned by `getaddrinfo`.

Walks the `ai_next` chain and frees each node along with its
embedded `sockaddr` and (if present) `ai_canonname` buffer.
Always pair every successful `getaddrinfo` with one
`freeaddrinfo` to avoid leaking the resolver's allocation.

_Defined in `lang/std/net/libc.ks`._

## function `getaddrinfo`

```kestrel
public func getaddrinfo(Pointer[UInt8], Pointer[UInt8], Pointer[UInt8], Pointer[Pointer[UInt8]]) -> Int32
```

Wraps `getaddrinfo(3)` — DNS / service-name resolution.

Resolves `node` (hostname or numeric address) and `service`
(service name or port string) to a linked list of `addrinfo`
records, written through `res`. `hints` constrains the result
(family, socket type, protocol). Returns `0` on success or a
non-zero `EAI_*` code on failure (note: not an `errno`). The
caller must free the list with `freeaddrinfo`.

### `addrinfo` struct layout (macOS, 48 bytes)

```
  offset 0:  ai_flags    (i32)
  offset 4:  ai_family   (i32)
  offset 8:  ai_socktype (i32)
  offset 12: ai_protocol (i32)
  offset 16: ai_addrlen  (u32)
  offset 20: padding     (4 bytes on macOS, differs from Linux)
  offset 24: ai_canonname (ptr)
  offset 32: ai_addr     (ptr)
  offset 40: ai_next     (ptr)
```

_Defined in `lang/std/net/libc.ks`._

## function `htons`

```kestrel
public func htons(UInt16) -> UInt16
```

Wraps `htons(3)` — host-to-network byte order for 16-bit values.

On little-endian hosts this swaps the byte order; on big-endian
hosts it is the identity. Use to convert a port number before
writing it into a `sockaddr_in.sin_port` field.

_Defined in `lang/std/net/libc.ks`._

## function `listen`

```kestrel
public func listen(Int32, Int32) -> Int32
```

Wraps `listen(2)` — marks `sockfd` as accepting incoming connections.

`backlog` is the maximum length of the kernel's pending-
connection queue; once full, further connect attempts are
refused. Returns `0` on success, `-1` on error.

_Defined in `lang/std/net/libc.ks`._

## function `recv`

```kestrel
public func recv(Int32, Pointer[UInt8], Int64, Int32) -> Int64
```

Wraps `recv(2)` — reads up to `len` bytes from `sockfd` into `buf`.

Returns the byte count on success (which may be less than `len`),
`0` on orderly shutdown by the peer, or `-1` on error. `flags`
is a bitmask of `MSG_*` modifiers (`0` for the default).

_Defined in `lang/std/net/libc.ks`._

## function `send`

```kestrel
public func send(Int32, Pointer[UInt8], Int64, Int32) -> Int64
```

Wraps `send(2)` — writes up to `len` bytes from `buf` to `sockfd`.

May write fewer bytes than requested under back-pressure; the
caller must loop until the buffer is drained. Returns the byte
count on success or `-1` on error.

_Defined in `lang/std/net/libc.ks`._

## function `setsockopt`

```kestrel
public func setsockopt(Int32, Int32, Int32, Pointer[UInt8], Int32) -> Int32
```

Wraps `setsockopt(2)` — configures one option on `sockfd`.

`level` selects the option layer (e.g. `SOL_SOCKET`); `optname`
is the per-layer option code (e.g. `SO_REUSEADDR`); `optval` /
`optlen` describe the value. Returns `0` on success, `-1` on
error.

_Defined in `lang/std/net/libc.ks`._

## function `socket`

```kestrel
public func socket(Int32, Int32, Int32) -> Int32
```

Wraps `socket(2)` — creates a new socket fd.

Returns the new file descriptor on success, or `-1` on error
(call `errno()` for the cause). The caller owns the fd and is
responsible for closing it via `close`.

### Examples

```
let fd = socket(domain: AF_INET(), type_: SOCK_STREAM(), proto: IPPROTO_TCP());
if fd < 0 { /* errno() */ }
```

_Defined in `lang/std/net/libc.ks`._

