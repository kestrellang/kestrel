// Platform-specific filesystem constants (Linux x86_64)

module std.os.fs

import std.num.(Int64)

// errno is accessed via __errno_location() on Linux
@extern(.C, mangleName: "__errno_location")
func __errno_ptr() -> lang.ptr[lang.i32]

// stat struct layout (Linux x86_64)
func ST_MODE_OFFSET() -> Int64 { 24 }

// dirent struct layout (Linux)
func DIRENT_NAME_OFFSET() -> Int64 { 19 }
