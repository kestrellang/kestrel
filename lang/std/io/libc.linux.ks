// Platform-specific libc bindings (Linux)

module std.io.libc

import std.num.(Int32)

// errno is accessed via __errno_location() on Linux
@extern(.C, mangleName: "__errno_location")
func __errno_ptr() -> lang.ptr[lang.i32]

// Open flags (Linux values)

/// Create file if it doesn't exist.
public func O_CREAT() -> Int32 { 0x0040 }

/// Truncate file to zero length.
public func O_TRUNC() -> Int32 { 0x0200 }

/// Append to end of file.
public func O_APPEND() -> Int32 { 0x0400 }

/// Fail if file exists (with O_CREAT).
public func O_EXCL() -> Int32 { 0x0080 }
