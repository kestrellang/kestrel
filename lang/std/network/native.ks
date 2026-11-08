// Native bindings for std.network. Internal: the public API never exposes
// a raw pointer or a `sockaddr`; `network_shims.c` converts addresses.

module std.network

import std.numeric.(Int32, Int64)
import std.memory.(RawPointer)

@extern(.C, mangleName: "kestrel_socket")
internal func native_socket(family: Int32, kind: Int32) -> Int32

@extern(.C, mangleName: "kestrel_socket_pair")
internal func native_socket_pair(fds: RawPointer) -> Int32

@extern(.C, mangleName: "kestrel_socket_connect")
internal func native_connect(fd: Int32, address: RawPointer, timeoutMilliseconds: Int64) -> Int32

@extern(.C, mangleName: "kestrel_socket_bind")
internal func native_bind(fd: Int32, address: RawPointer) -> Int32

@extern(.C, mangleName: "kestrel_socket_listen")
internal func native_listen(fd: Int32, backlog: Int32) -> Int32

@extern(.C, mangleName: "kestrel_socket_accept")
internal func native_accept(fd: Int32, peer: RawPointer) -> Int32

@extern(.C, mangleName: "kestrel_socket_address")
internal func native_address(fd: Int32, peer: Int32, out: RawPointer) -> Int32

@extern(.C, mangleName: "kestrel_socket_send")
internal func native_send(fd: Int32, buf: RawPointer, len: Int64) -> Int64

@extern(.C, mangleName: "kestrel_socket_receive")
internal func native_receive(fd: Int32, buf: RawPointer, len: Int64) -> Int64

@extern(.C, mangleName: "kestrel_socket_send_to")
internal func native_send_to(fd: Int32, buf: RawPointer, len: Int64, to: RawPointer) -> Int64

@extern(.C, mangleName: "kestrel_socket_receive_from")
internal func native_receive_from(fd: Int32, buf: RawPointer, len: Int64, from: RawPointer) -> Int64

@extern(.C, mangleName: "kestrel_socket_shutdown")
internal func native_shutdown(fd: Int32, how: Int32) -> Int32

@extern(.C, mangleName: "kestrel_socket_set_flag")
internal func native_set_flag(fd: Int32, option: Int32, on: Int32) -> Int32

@extern(.C, mangleName: "kestrel_socket_set_timeout")
internal func native_set_timeout(fd: Int32, write: Int32, timeoutMilliseconds: Int64) -> Int32

@extern(.C, mangleName: "kestrel_socket_set_nonblocking")
internal func native_set_nonblocking(fd: Int32, on: Int32) -> Int32

@extern(.C, mangleName: "kestrel_socket_duplicate")
internal func native_duplicate(fd: Int32) -> Int32

@extern(.C, mangleName: "kestrel_lookup")
internal func native_lookup(host: RawPointer, port: Int64, out: RawPointer, capacity: Int64, errorKind: RawPointer) -> Int64

@extern(.C, mangleName: "kestrel_unix_connect")
internal func native_unix_connect(fd: Int32, path: RawPointer) -> Int32

@extern(.C, mangleName: "kestrel_unix_bind")
internal func native_unix_bind(fd: Int32, path: RawPointer) -> Int32

@extern(.C, mangleName: "close")
internal func native_close(fd: Int32) -> Int32
