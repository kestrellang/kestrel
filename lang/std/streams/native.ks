// Native bindings for the standard streams. Internal to std.streams.

module std.streams

import std.numeric.(Int32, Int64)
import std.memory.(RawPointer)

/// Buffers bytes for standard output (`streams_shims.c`).
@extern(.C, mangleName: "kestrel_stdout_write")
internal func native_stdout_write(bytes: RawPointer, count: Int64) -> Int64

/// Writes the standard output buffer out. `0`, or `-1` with errno set.
@extern(.C, mangleName: "kestrel_stdout_flush")
internal func native_stdout_flush() -> Int64

/// Reads standard input through the shared buffer.
@extern(.C, mangleName: "kestrel_stdin_read")
internal func native_stdin_read(bytes: RawPointer, count: Int64) -> Int64

@extern(.C, mangleName: "write")
internal func native_stream_write(fd: Int32, bytes: RawPointer, count: Int64) -> Int64
