// libc bindings for I/O
//
// This module provides raw bindings to libc I/O functions via @extern(.C).

module std.io.libc

import std.num.(Int64, Int32)
import std.memory.(Pointer)
import std.num.(UInt8)

// ============================================================================
// TYPE ALIASES
// ============================================================================

/// File descriptor type (wraps an int).
public type Fd = Int32

// ============================================================================
// STANDARD FILE DESCRIPTORS
// ============================================================================

/// Standard input file descriptor.
public func STDIN() -> Fd { 0 }

/// Standard output file descriptor.
public func STDOUT() -> Fd { 1 }

/// Standard error file descriptor.
public func STDERR() -> Fd { 2 }

// ============================================================================
// OPEN FLAGS (POSIX)
// ============================================================================

/// Open for reading only.
public func O_RDONLY() -> Int32 { 0x0000 }

/// Open for writing only.
public func O_WRONLY() -> Int32 { 0x0001 }

/// Open for reading and writing.
public func O_RDWR() -> Int32 { 0x0002 }

// O_CREAT, O_TRUNC, O_APPEND, O_EXCL are in libc.darwin.ks / libc.linux.ks

// ============================================================================
// SEEK WHENCE CONSTANTS
// ============================================================================

/// Seek from beginning of file.
public func SEEK_SET() -> Int32 { 0 }

/// Seek from current position.
public func SEEK_CUR() -> Int32 { 1 }

/// Seek from end of file.
public func SEEK_END() -> Int32 { 2 }

// ============================================================================
// FILE MODE CONSTANTS
// ============================================================================

/// Default file mode (rw-r--r-- = 0o644 = 420 decimal).
public func MODE_DEFAULT() -> Int32 { 420 }

// ============================================================================
// RAW LIBC BINDINGS
// ============================================================================

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

// __errno_ptr() is in libc.darwin.ks / libc.linux.ks

// ============================================================================
// PUBLIC WRAPPERS
// ============================================================================

/// Opens a file. Returns file descriptor or -1 on error.
public func open(path: Pointer[UInt8], flags: Int32, mode: Int32) -> Fd {
    Int32(raw: libc_open(lang.cast_ptr[lang.i8](path.raw), flags.raw, mode.raw))
}

/// Closes a file descriptor. Returns 0 on success, -1 on error.
public func close(fd: Int32) -> Int32 {
    Int32(raw: libc_close(fd.raw))
}

/// Reads from a file descriptor. Returns bytes read, 0 on EOF, -1 on error.
public func read(fd: Int32, buf: Pointer[UInt8], count: Int64) -> Int64 {
    Int64(raw: libc_read(fd.raw, lang.cast_ptr[lang.i8](buf.raw), count.raw))
}

/// Writes to a file descriptor. Returns bytes written or -1 on error.
public func write(fd: Int32, buf: Pointer[UInt8], count: Int64) -> Int64 {
    Int64(raw: libc_write(fd.raw, lang.cast_ptr[lang.i8](buf.raw), count.raw))
}

/// Seeks to a position in a file. Returns new position or -1 on error.
public func lseek(fd: Int32, offset: Int64, whence: Int32) -> Int64 {
    Int64(raw: libc_lseek(fd.raw, offset.raw, whence.raw))
}

/// Returns the current errno value.
public func errno() -> Int32 {
    let ptr = __errno_ptr();
    Int32(raw: lang.ptr_read(ptr))
}
