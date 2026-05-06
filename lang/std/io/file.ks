// File I/O

module std.io.file

import std.numeric.(Int64, Int32, UInt8)
import std.result.(Result, Optional)
import std.memory.(ArraySlice, Pointer)
import std.collections.(Array)
import std.text.(String)
import std.core.(Bool, Copyable)
import std.io.libc
import std.io.error.(IoError)
import std.io.read.(Readable, readAll)
import std.io.write.(Writable, writeAll, writeString)

// ============================================================================
// SEEK POSITION
// ============================================================================

/// Anchor + offset pair passed to `File.seek`. The three variants match
/// POSIX `SEEK_SET`, `SEEK_CUR`, and `SEEK_END`; the payload is the
/// offset in bytes (signed, so backwards seeks work).
///
/// # Examples
///
/// ```
/// try file.seek(to: .Start(0));        // beginning
/// try file.seek(to: .Current(-10));    // back 10 bytes
/// try file.seek(to: .End(0));          // end of file
/// ```
public enum Seek {
    /// Seek to an absolute byte offset from the start of the file.
    case Start(Int64)

    /// Seek by `n` bytes from the current position. Negative values move
    /// backwards.
    case Current(Int64)

    /// Seek by `n` bytes from EOF. Use `0` to land exactly at EOF;
    /// negative values move backwards from the end.
    case End(Int64)
}

// ============================================================================
// FILE
// ============================================================================

/// RAII-owned POSIX file handle.
///
/// The wrapped file descriptor is closed automatically when the `File`
/// goes out of scope, so explicit `close` is never necessary. `File` is
/// `not Copyable` to keep the descriptor uniquely owned — pass by
/// reference or move it instead. Conforms to both `Readable` and `Writable`,
/// although calls fail with `EBADF` if the open mode does not permit the
/// direction (e.g. writing to a file opened with `open()`).
///
/// # Examples
///
/// ```
/// // Readable whole file in 4 KiB chunks.
/// var file = try File.open("input.txt");
/// var buf = Array[UInt8](repeating: 0, count: 4096);
/// while true {
///     let n = try file.read(into: buf.asSlice());
///     if n == 0 { break }
///     // process buf[0..n]
/// }
/// ```
///
/// # Representation
///
/// One `libc.Fd` (32-bit signed integer) field.
///
/// # Memory Model
///
/// Owning, unique. The `deinit` calls `close(fd)` if `fd >= 0`; close
/// errors are silently ignored — there's no caller to surface them to.
public struct File: Readable, Writable, not Copyable {
    var fd: libc.Fd

    /// @name From Fd
    /// Internal init wrapping a raw descriptor; not for general use.
    init(fd: libc.Fd) {
        self.fd = fd
    }

    // ========================================================================
    // OPENING FILES
    // ========================================================================

    /// @name Open
    /// Opens an existing file for reading. The file must exist; missing
    /// paths surface as `Err(IoError.last())` carrying `ENOENT`, and
    /// permission failures as `EACCES`.
    public static func open(path: String) -> Result[File, IoError] {
        // Get pointer to string bytes (need null-terminated for libc)
        // For now, we'll copy to a buffer with null terminator
        let len = path.byteCount;
        var pathBuf = Array[UInt8](capacity: len + 1);
        var i: Int64 = 0;
        while i < len {
            pathBuf.append(path.bytes(unchecked: i));
            i = i + 1
        }
        pathBuf.append(0); // null terminator

        let fd = libc.open(pathBuf.asPointer(), libc.O_RDONLY(), 0);
        if fd < 0 {
            return .Err(IoError.last())
        }
        .Ok(File(fd))
    }

    /// Creates (or truncates) `path` for writing with mode `0644`.
    /// Existing contents are discarded.
    ///
    /// # Examples
    ///
    /// ```
    /// var file = try File.create("output.txt");
    /// try writeString(file, "New content");
    /// ```
    public static func create(path: String) -> Result[File, IoError] {
        let len = path.byteCount;
        var pathBuf = Array[UInt8](capacity: len + 1);
        var i: Int64 = 0;
        while i < len {
            pathBuf.append(path.bytes(unchecked: i));
            i = i + 1
        }
        pathBuf.append(0);

        let flags = libc.O_WRONLY() | libc.O_CREAT() | libc.O_TRUNC();
        let fd = libc.open(pathBuf.asPointer(), flags, libc.MODE_DEFAULT());
        if fd < 0 {
            return .Err(IoError.last())
        }
        .Ok(File(fd))
    }

    /// Opens an existing file for both reading and writing. Use for
    /// in-place modification of a file that already exists; for "create
    /// or open" semantics combine with `create` / `createNew` as
    /// appropriate.
    public static func openReadWrite(path: String) -> Result[File, IoError] {
        let len = path.byteCount;
        var pathBuf = Array[UInt8](capacity: len + 1);
        var i: Int64 = 0;
        while i < len {
            pathBuf.append(path.bytes(unchecked: i));
            i = i + 1
        }
        pathBuf.append(0);

        let fd = libc.open(pathBuf.asPointer(), libc.O_RDWR(), 0);
        if fd < 0 {
            return .Err(IoError.last())
        }
        .Ok(File(fd))
    }

    /// Opens (or creates) a file in append mode. Every write atomically
    /// lands at the current end of file regardless of where `seek` last
    /// left the cursor — the standard idiom for log files and any
    /// concurrent appender.
    public static func openAppend(path: String) -> Result[File, IoError] {
        let len = path.byteCount;
        var pathBuf = Array[UInt8](capacity: len + 1);
        var i: Int64 = 0;
        while i < len {
            pathBuf.append(path.bytes(unchecked: i));
            i = i + 1
        }
        pathBuf.append(0);

        let flags = libc.O_WRONLY() | libc.O_CREAT() | libc.O_APPEND();
        let fd = libc.open(pathBuf.asPointer(), flags, libc.MODE_DEFAULT());
        if fd < 0 {
            return .Err(IoError.last())
        }
        .Ok(File(fd))
    }

    /// Creates a new file, failing if the path already exists. The
    /// canonical pattern for cooperative locking via lockfiles.
    ///
    /// # Errors
    ///
    /// Returns `Err` carrying `EEXIST` if the path already exists.
    ///
    /// # Examples
    ///
    /// ```
    /// match File.createNew("lock.pid") {
    ///     .Ok(f) => /* we hold the lock */ holdLock(f),
    ///     .Err(e) => /* somebody else has it */ retryLater()
    /// }
    /// ```
    public static func createNew(path: String) -> Result[File, IoError] {
        let len = path.byteCount;
        var pathBuf = Array[UInt8](capacity: len + 1);
        var i: Int64 = 0;
        while i < len {
            pathBuf.append(path.bytes(unchecked: i));
            i = i + 1
        }
        pathBuf.append(0);

        let flags = libc.O_WRONLY() | libc.O_CREAT() | libc.O_EXCL();
        let fd = libc.open(pathBuf.asPointer(), flags, libc.MODE_DEFAULT());
        if fd < 0 {
            return .Err(IoError.last())
        }
        .Ok(File(fd))
    }

    // ========================================================================
    // READ/WRITE IMPLEMENTATION
    // ========================================================================

    /// Calls `read(2)`. Advances the file position by the byte count
    /// returned. Short reads (`n < buf.count`) are normal — keep calling
    /// until `0` is returned (EOF) or an error fires. Use `readAll`/
    /// `readExact` from `std.io.read` when looping by hand isn't wanted.
    public mutating func read(into buf: ArraySlice[UInt8]) -> Result[Int64, IoError] {
        let n = libc.read(self.fd, buf.pointer, buf.count);
        if n < 0 {
            return .Err(IoError.last())
        }
        .Ok(n)
    }

    /// Calls `write(2)`. May write fewer bytes than supplied — wrap with
    /// `writeAll` from `std.io.write` to loop until done.
    public mutating func write(from buf: ArraySlice[UInt8]) -> Result[Int64, IoError] {
        let n = libc.write(self.fd, buf.pointer, buf.count);
        if n < 0 {
            return .Err(IoError.last())
        }
        .Ok(n)
    }

    /// No-op; `File` does no internal buffering. Reaches the kernel as
    /// soon as `write` returns, but does not call `fsync` — durability
    /// across power loss requires a separate, currently-unwrapped libc
    /// call.
    public mutating func flush() -> Result[(), IoError] {
        .Ok(())
    }

    // ========================================================================
    // SEEKING
    // ========================================================================

    /// Calls `lseek(2)` with the requested anchor and offset. Returns
    /// the new absolute position from the start of the file. Seeking
    /// past EOF is allowed; a subsequent write extends the file (with a
    /// hole on filesystems that support sparse files).
    ///
    /// # Examples
    ///
    /// ```
    /// var file = try File.openReadWrite("data.bin");
    /// try file.seek(to: .Start(0));        // rewind
    /// try file.seek(to: .Current(100));    // skip 100 bytes
    /// let size = try file.seek(to: .End(0));   // size of file
    /// ```
    public mutating func seek(to pos: Seek) -> Result[Int64, IoError] {
        let pair = match pos {
            .Start(o) => (o, libc.SEEK_SET()),
            .Current(o) => (o, libc.SEEK_CUR()),
            .End(o) => (o, libc.SEEK_END())
        };
        let result = libc.lseek(self.fd, pair.0, pair.1);
        if result < 0 {
            return .Err(IoError.last())
        }
        .Ok(result)
    }

    /// Convenience for `seek(.Current(0))`.
    public mutating func position() -> Result[Int64, IoError] {
        self.seek(to: .Current(0))
    }

    /// Convenience for `seek(.Start(0))` that drops the returned offset.
    public mutating func rewind() -> Result[(), IoError] {
        try self.seek(to: .Start(0));
        .Ok(())
    }

    // ========================================================================
    // LOW-LEVEL ACCESS
    // ========================================================================

    /// Returns the underlying libc file descriptor for direct FFI use.
    /// Ownership stays with the `File`; do not call `close` on the
    /// returned value or the `deinit` will hit `EBADF`.
    public func rawFd() -> libc.Fd {
        self.fd
    }

    // ========================================================================
    // DESTRUCTOR
    // ========================================================================

    /// Closes the descriptor on scope exit. Errors are swallowed —
    /// there's no caller to receive them; if `close` failure matters,
    /// flush and `close` explicitly via the libc bindings before drop.
    deinit {
        if self.fd >= 0 {
            let _ = libc.close(self.fd);
        }
    }
}

// ============================================================================
// CONVENIENCE FUNCTIONS
// ============================================================================

/// Reads `path` into a `String`, decoding the bytes as UTF-8. Convenient
/// for config files, source files, and other small/medium text. Slurps
/// the entire file into memory — for huge inputs prefer streaming via
/// `File` + `readAll`.
///
/// # Examples
///
/// ```
/// let cfg = try readFileString("config.json");
/// ```
public func readFileString(path: String) -> Result[String, IoError] {
    var file = try File.open(path);
    var bytes = Array[UInt8]();
    try readAll(file, into: bytes);
    // Build string from bytes
    var result = String();
    var i: Int64 = 0;
    let count = bytes.count;
    while i < count {
        result.appendByte(bytes(unchecked: i));
        i = i + 1
    }
    .Ok(result)
}

/// Reads `path` into an `Array[UInt8]`. The binary counterpart to
/// `readFileString` — does no UTF-8 decoding.
public func readFileBytes(path: String) -> Result[Array[UInt8], IoError] {
    var file = try File.open(path);
    var bytes = Array[UInt8]();
    try readAll(file, into: bytes);
    .Ok(bytes)
}

/// Writes `content` to `path`, creating or truncating as needed. Bytes
/// are the UTF-8 encoding of the string. The mirror of `readFileString`.
public func writeFileString(path: String, content: String) -> Result[(), IoError] {
    var file = try File.create(path);
    writeString(file, content)
}

/// Writes `content` to `path`, creating or truncating as needed.
/// Binary equivalent of `writeFileString`.
public func writeFileBytes(path: String, content: Array[UInt8]) -> Result[(), IoError] {
    var file = try File.create(path);
    writeAll(file, from: content.asSlice())
}

/// Appends `content` to `path` as UTF-8, creating the file if absent.
/// Atomic per-write under POSIX `O_APPEND` semantics — safe to call from
/// multiple writers without intermediate locking, though writes longer
/// than `PIPE_BUF` may interleave.
public func appendFileString(path: String, content: String) -> Result[(), IoError] {
    var file = try File.openAppend(path);
    writeString(file, content)
}

/// Appends bytes to `path`, creating if absent. Binary counterpart to
/// `appendFileString`.
public func appendFileBytes(path: String, content: Array[UInt8]) -> Result[(), IoError] {
    var file = try File.openAppend(path);
    writeAll(file, from: content.asSlice())
}
