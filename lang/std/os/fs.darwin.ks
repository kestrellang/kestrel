// Platform-specific filesystem constants (macOS)

module std.os.fs

import std.num.(Int64)

// errno is accessed via __error() on macOS
@extern(.C, mangleName: "__error")
func __errno_ptr() -> lang.ptr[lang.i32]

// stat struct layout (macOS)
func ST_MODE_OFFSET() -> Int64 { 4 }

// dirent struct layout (macOS)
func DIRENT_NAME_OFFSET() -> Int64 { 21 }
