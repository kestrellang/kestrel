// Platform-specific libc bindings (macOS)

module std.io.libc

import std.num.(Int32)

// errno is accessed via __error() on macOS
@extern(.C, mangleName: "__error")
func __errno_ptr() -> lang.ptr[lang.i32]

// Open flags (macOS values)

/// Create file if it doesn't exist.
public func O_CREAT() -> Int32 { 0x0200 }

/// Truncate file to zero length.
public func O_TRUNC() -> Int32 { 0x0400 }

/// Append to end of file.
public func O_APPEND() -> Int32 { 0x0008 }

/// Fail if file exists (with O_CREAT).
public func O_EXCL() -> Int32 { 0x0800 }
