// libc bindings for I/O
//
// This module provides raw bindings to libc I/O functions via @extern(.C).

module std.io.libc

import std.num.(Int64, Int32)
import std.memory.(Pointer)
import std.num.(UInt8)

// File descriptor type (just an int)
public type Fd = Int32

// Standard file descriptors
public func STDIN() -> Fd { 0 }
public func STDOUT() -> Fd { 1 }
public func STDERR() -> Fd { 2 }

// Open flags (POSIX)
public func O_RDONLY() -> Int32 { 0x0000 }
public func O_WRONLY() -> Int32 { 0x0001 }
public func O_RDWR() -> Int32 { 0x0002 }
public func O_CREAT() -> Int32 { 0x0200 }
public func O_TRUNC() -> Int32 { 0x0400 }
public func O_APPEND() -> Int32 { 0x0008 }
public func O_EXCL() -> Int32 { 0x0800 }

// Seek whence
public func SEEK_SET() -> Int32 { 0 }
public func SEEK_CUR() -> Int32 { 1 }
public func SEEK_END() -> Int32 { 2 }

// Default file mode (rw-r--r--)
public func MODE_DEFAULT() -> Int32 { 420 }  // 0o644 = 420 decimal

// libc functions via @extern (using lang.ptr for FFI safety)

@extern(.C, mangleName: "open")
func libc_open(path: lang.ptr[lang.i8], flags: lang.i32, mode: lang.i32) -> lang.i32

@extern(.C, mangleName: "close")
func libc_close(fd: lang.i32) -> lang.i32

@extern(.C, mangleName: "read")
func libc_read(fd: lang.i32, buf: lang.ptr[lang.i8], count: lang.i64) -> lang.i64

@extern(.C, mangleName: "write")
func libc_write(fd: lang.i32, buf: lang.ptr[lang.i8], count: lang.i64) -> lang.i64

@extern(.C, mangleName: "lseek")
func libc_lseek(fd: lang.i32, offset: lang.i64, whence: lang.i32) -> lang.i64

// errno is accessed via __error() on macOS
@extern(.C, mangleName: "__error")
func __error() -> lang.ptr[lang.i32]

// Public wrappers that convert between Kestrel types and lang types

public func open(path: Pointer[UInt8], flags: Int32, mode: Int32) -> Fd {
    Int32(raw: libc_open(lang.cast_ptr[lang.i8](path.raw), flags.raw, mode.raw))
}

public func close(fd: Int32) -> Int32 {
    Int32(raw: libc_close(fd.raw))
}

public func read(fd: Int32, buf: Pointer[UInt8], count: Int64) -> Int64 {
    Int64(raw: libc_read(fd.raw, lang.cast_ptr[lang.i8](buf.raw), count.raw))
}

public func write(fd: Int32, buf: Pointer[UInt8], count: Int64) -> Int64 {
    Int64(raw: libc_write(fd.raw, lang.cast_ptr[lang.i8](buf.raw), count.raw))
}

public func lseek(fd: Int32, offset: Int64, whence: Int32) -> Int64 {
    Int64(raw: libc_lseek(fd.raw, offset.raw, whence.raw))
}

public func errno() -> Int32 {
    let ptr = __error();
    Int32(raw: lang.ptr_read(ptr))
}
