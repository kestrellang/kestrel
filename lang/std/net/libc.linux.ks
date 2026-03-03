// Platform-specific socket bindings (Linux)

module std.net.libc

import std.num.(Int32)

// errno is accessed via __errno_location() on Linux
@extern(.C, mangleName: "__errno_location")
func __errno_ptr() -> lang.ptr[lang.i32]

// Socket constants (Linux values)

/// Socket-level options.
public func SOL_SOCKET() -> Int32 { 1 }

/// Allow address reuse.
public func SO_REUSEADDR() -> Int32 { 2 }
