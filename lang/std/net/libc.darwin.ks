// Platform-specific socket bindings (macOS)

module std.net.libc

import std.num.(Int32)

// errno is accessed via __error() on macOS
@extern(.C, mangleName: "__error")
func __errno_ptr() -> lang.ptr[lang.i32]

// Socket constants (macOS values)

/// Socket-level options.
public func SOL_SOCKET() -> Int32 { 0xFFFF }

/// Allow address reuse.
public func SO_REUSEADDR() -> Int32 { 0x0004 }
