// libc bindings for I/O
//
// Thin `@extern(.C)` wrappers around the POSIX I/O syscalls plus the
// platform constants the higher-level types in `std.io` need (open
// flags, seek anchors, default mode bits). Prefer `File`, `Stdin`,
// `Stdout`, `Stderr`, and the `read`/`write` helpers in `std.io` over
// these raw bindings; reach for `libc.*` only for FFI interop.

module std.io.libc

import std.num.(Int64, Int32)
import std.memory.(Pointer, RawPointer)
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

// errno access
@platform(.darwin)
@extern(.C, mangleName: "__error")
func __errno_ptr() -> Pointer[Int32]

@platform(.linux)
@extern(.C, mangleName: "__errno_location")
func __errno_ptr() -> Pointer[Int32]

// Open flags (platform-specific values)

/// Create file if it doesn't exist.
@platform(.darwin)
public func O_CREAT() -> Int32 { 0x0200 }

/// Create file if it doesn't exist.
@platform(.linux)
public func O_CREAT() -> Int32 { 0x0040 }

/// Truncate file to zero length.
@platform(.darwin)
public func O_TRUNC() -> Int32 { 0x0400 }

/// Truncate file to zero length.
@platform(.linux)
public func O_TRUNC() -> Int32 { 0x0200 }

/// Append to end of file.
@platform(.darwin)
public func O_APPEND() -> Int32 { 0x0008 }

/// Append to end of file.
@platform(.linux)
public func O_APPEND() -> Int32 { 0x0400 }

/// Fail if file exists (with O_CREAT).
@platform(.darwin)
public func O_EXCL() -> Int32 { 0x0800 }

/// Fail if file exists (with O_CREAT).
@platform(.linux)
public func O_EXCL() -> Int32 { 0x0080 }

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
func libc_open(path: RawPointer, flags: Int32, mode: Int32) -> Int32

@extern(.C, mangleName: "close")
func libc_close(fd: Int32) -> Int32

@extern(.C, mangleName: "read")
func libc_read(fd: Int32, buf: RawPointer, count: Int64) -> Int64

@extern(.C, mangleName: "write")
func libc_write(fd: Int32, buf: RawPointer, count: Int64) -> Int64

@extern(.C, mangleName: "lseek")
func libc_lseek(fd: Int32, offset: Int64, whence: Int32) -> Int64


// ============================================================================
// PUBLIC WRAPPERS
// ============================================================================

/// Opens a file. Returns file descriptor or -1 on error.
public func open(path: Pointer[UInt8], flags: Int32, mode: Int32) -> Fd {
    libc_open(path.asRaw(), flags, mode)
}

/// Closes a file descriptor. Returns 0 on success, -1 on error.
public func close(fd: Int32) -> Int32 {
    libc_close(fd)
}

/// Reads from a file descriptor. Returns bytes read, 0 on EOF, -1 on error.
public func read(fd: Int32, buf: Pointer[UInt8], count: Int64) -> Int64 {
    libc_read(fd, buf.asRaw(), count)
}

/// Writes to a file descriptor. Returns bytes written or -1 on error.
public func write(fd: Int32, buf: Pointer[UInt8], count: Int64) -> Int64 {
    libc_write(fd, buf.asRaw(), count)
}

/// Seeks to a position in a file. Returns new position or -1 on error.
public func lseek(fd: Int32, offset: Int64, whence: Int32) -> Int64 {
    libc_lseek(fd, offset, whence)
}

/// Returns the current errno value.
public func errno() -> Int32 {
    __errno_ptr().read()
}
