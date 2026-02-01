// File I/O

module std.io.file

import std.num.(Int64, Int32, UInt8)
import std.result.(Result, Optional)
import std.memory.(Slice, Pointer)
import std.collections.(Array)
import std.text.(String)
import std.core.(Bool, Copyable)
import std.io.libc
import std.io.error.(Error)
import std.io.read.(Read, readAll)
import std.io.write.(Write, writeAll, writeStr)

// ============================================================================
// SEEK POSITION
// ============================================================================

/// Represents a seek position for file operations.
///
/// Used with `File.seek()` to reposition the file cursor.
///
/// Example:
///     try file.seek(to: .Start(0))     // beginning
///     try file.seek(to: .Current(-10)) // back 10 bytes
///     try file.seek(to: .End(0))       // end of file
public enum Seek {
    /// Seek to an absolute position from the start of the file.
    ///
    /// Example:
    ///     try file.seek(to: .Start(100))  // go to byte 100
    case Start(Int64)

    /// Seek relative to the current position.
    ///
    /// Positive values move forward, negative values move backward.
    ///
    /// Example:
    ///     try file.seek(to: .Current(10))   // forward 10 bytes
    ///     try file.seek(to: .Current(-5))   // back 5 bytes
    case Current(Int64)

    /// Seek relative to the end of the file.
    ///
    /// Use negative values to seek before the end.
    ///
    /// Example:
    ///     try file.seek(to: .End(0))    // go to end
    ///     try file.seek(to: .End(-10))  // 10 bytes before end
    case End(Int64)
}

// ============================================================================
// FILE
// ============================================================================

/// An owned file handle that automatically closes when dropped.
///
/// File provides safe, RAII-style file access. The underlying file descriptor
/// is closed automatically when the File goes out of scope. File is not
/// copyable to ensure exclusive ownership of the file descriptor.
///
/// File implements both Read and Write protocols, though not all open modes
/// support both operations (e.g., a file opened with `open()` is read-only).
///
/// Example:
///     // RAII: file automatically closed at end of scope
///     {
///         var file = try File.open(path: "data.txt")
///         // use file...
///     } // file closed here
///
///     // Read entire file
///     var file = try File.open(path: "input.txt")
///     var contents = [UInt8]()
///     try readAll(reader: file, into: contents)
public struct File: Read, Write, not Copyable {
    var fd: libc.Fd

    /// Private: create from raw fd.
    init(fd: libc.Fd) {
        self.fd = fd
    }

    // ========================================================================
    // OPENING FILES
    // ========================================================================

    /// Opens a file for reading.
    ///
    /// The file must exist. Returns Err with ENOENT if the file doesn't exist,
    /// or EACCES if permission is denied.
    ///
    /// Example:
    ///     var file = try File.open(path: "existing.txt")
    ///     var buf = [UInt8](repeating: 0, count: 100)
    ///     let n = try file.read(into: buf.asSlice())
    public static func open(path: String) -> File throws Error {
        // Get pointer to string bytes (need null-terminated for libc)
        // For now, we'll copy to a buffer with null terminator
        let len = path.byteCount;
        var pathBuf = [UInt8](capacity: len + 1);
        var i: Int64 = 0;
        while i < len {
            pathBuf.append(path.byteAtUnchecked(i));
            i = i + 1
        }
        pathBuf.append(0); // null terminator

        let fd = libc.open(pathBuf.pointer(), libc.O_RDONLY(), 0);
        if fd < 0 {
            throw Error.last()
        }
        File(fd)
    }

    /// Creates a file for writing, truncating if it exists.
    ///
    /// If the file exists, it is truncated to zero length.
    /// If the file doesn't exist, it is created with default permissions (0644).
    ///
    /// Example:
    ///     var file = try File.create(path: "output.txt")
    ///     try writeStr(writer: file, s: "New content")
    public static func create(path: String) -> File throws Error {
        let len = path.byteCount;
        var pathBuf = [UInt8](capacity: len + 1);
        var i: Int64 = 0;
        while i < len {
            pathBuf.append(path.byteAtUnchecked(i));
            i = i + 1
        }
        pathBuf.append(0);

        let flags = libc.O_WRONLY() | libc.O_CREAT() | libc.O_TRUNC();
        let fd = libc.open(pathBuf.pointer(), flags, libc.MODE_DEFAULT());
        if fd < 0 {
            throw Error.last()
        }
        File(fd)
    }

    /// Opens a file for both reading and writing.
    ///
    /// The file must exist. Use for in-place modification of existing files.
    ///
    /// Example:
    ///     var file = try File.openReadWrite(path: "data.bin")
    ///     var header = [UInt8](repeating: 0, count: 16)
    ///     try file.read(into: header.asSlice())
    ///     try file.seek(to: .Start(0))
    ///     try file.write(from: newHeader.asSlice())
    public static func openReadWrite(path: String) -> File throws Error {
        let len = path.byteCount;
        var pathBuf = [UInt8](capacity: len + 1);
        var i: Int64 = 0;
        while i < len {
            pathBuf.append(path.byteAtUnchecked(i));
            i = i + 1
        }
        pathBuf.append(0);

        let fd = libc.open(pathBuf.pointer(), libc.O_RDWR(), 0);
        if fd < 0 {
            throw Error.last()
        }
        File(fd)
    }

    /// Opens a file for appending, creating if it doesn't exist.
    ///
    /// All writes are atomically appended to the end of the file,
    /// regardless of seek position. Useful for log files.
    ///
    /// Example:
    ///     var log = try File.openAppend(path: "app.log")
    ///     try writeLine(writer: log, s: "Application started")
    public static func openAppend(path: String) -> File throws Error {
        let len = path.byteCount;
        var pathBuf = [UInt8](capacity: len + 1);
        var i: Int64 = 0;
        while i < len {
            pathBuf.append(path.byteAtUnchecked(i));
            i = i + 1
        }
        pathBuf.append(0);

        let flags = libc.O_WRONLY() | libc.O_CREAT() | libc.O_APPEND();
        let fd = libc.open(pathBuf.pointer(), flags, libc.MODE_DEFAULT());
        if fd < 0 {
            throw Error.last()
        }
        File(fd)
    }

    /// Creates a new file, failing if it already exists.
    ///
    /// Use for exclusive file creation when overwriting would be an error.
    /// Returns Err with EEXIST if the file already exists.
    ///
    /// Example:
    ///     match File.createNew(path: "lock.pid") {
    ///         case .Ok(file) => // we have the lock
    ///         case .Err(e) => // another process has the lock
    ///     }
    public static func createNew(path: String) -> File throws Error {
        let len = path.byteCount;
        var pathBuf = [UInt8](capacity: len + 1);
        var i: Int64 = 0;
        while i < len {
            pathBuf.append(path.byteAtUnchecked(i));
            i = i + 1
        }
        pathBuf.append(0);

        let flags = libc.O_WRONLY() | libc.O_CREAT() | libc.O_EXCL();
        let fd = libc.open(pathBuf.pointer(), flags, libc.MODE_DEFAULT());
        if fd < 0 {
            throw Error.last()
        }
        File(fd)
    }

    // ========================================================================
    // READ/WRITE IMPLEMENTATION
    // ========================================================================

    /// Reads bytes from the file into the buffer.
    ///
    /// Reads up to buf.count bytes starting at the current position.
    /// Advances the file position by the number of bytes read.
    ///
    /// Returns:
    /// - Ok(n) where n > 0: read n bytes into buf[0..n]
    /// - Ok(0): end of file reached
    /// - Err: an error occurred
    ///
    /// Note: May read fewer bytes than requested even if more data exists.
    /// This is not an error; call read again for more data.
    ///
    /// Example:
    ///     var file = try File.open(path: "data.bin")
    ///     var buf = [UInt8](repeating: 0, count: 4096)
    ///     while true {
    ///         let n = try file.read(into: buf.asSlice())
    ///         if n == 0 { break }  // EOF
    ///         // process buf[0..n]
    ///     }
    public mutating func read(into buf: Slice[UInt8]) -> Int64 throws Error {
        let n = libc.read(self.fd, buf.pointer, buf.count);
        if n < 0 {
            throw Error.last()
        }
        n
    }

    /// Writes bytes from the buffer to the file.
    ///
    /// Writes up to buf.count bytes starting at the current position
    /// (or at end of file if opened with openAppend).
    /// Advances the file position by the number of bytes written.
    ///
    /// Returns:
    /// - Ok(n): wrote n bytes from buf[0..n]
    /// - Err: an error occurred (e.g., disk full, permission denied)
    ///
    /// Note: May write fewer bytes than provided. Use writeAll() to ensure
    /// all bytes are written, or retry with the remaining bytes.
    ///
    /// Example:
    ///     var file = try File.create(path: "output.bin")
    ///     let data: [UInt8] = [0x00, 0x01, 0x02, 0x03]
    ///     try writeAll(writer: file, from: data.asSlice())
    public mutating func write(from buf: Slice[UInt8]) -> Int64 throws Error {
        let n = libc.write(self.fd, buf.pointer, buf.count);
        if n < 0 {
            throw Error.last()
        }
        n
    }

    /// Flushes buffered writes to the underlying file.
    ///
    /// Ensures data written to the file handle reaches the OS.
    /// Note: This does not guarantee data is persisted to disk;
    /// the OS may still buffer the data.
    ///
    /// Example:
    ///     var file = try File.create(path: "important.dat")
    ///     try writeAll(writer: file, from: data.asSlice())
    ///     try file.flush()  // ensure data reaches OS
    public mutating func flush() -> () throws Error {
        ()
    }

    // ========================================================================
    // SEEKING
    // ========================================================================

    /// Seeks to a position in the file.
    ///
    /// Returns the new absolute position from the start of the file.
    /// Seeking past the end of a file is allowed; a subsequent write
    /// will extend the file (creating a hole on some filesystems).
    ///
    /// Example:
    ///     var file = try File.openReadWrite(path: "data.bin")
    ///
    ///     // Go to beginning
    ///     try file.seek(to: .Start(0))
    ///
    ///     // Skip forward
    ///     try file.seek(to: .Current(100))
    ///
    ///     // Go to end and get file size
    ///     let size = try file.seek(to: .End(0))
    public mutating func seek(to pos: Seek) -> Int64 throws Error {
        let pair = match pos {
            .Start(o) => (o, libc.SEEK_SET()),
            .Current(o) => (o, libc.SEEK_CUR()),
            .End(o) => (o, libc.SEEK_END())
        };
        let offset = pair.0;
        let whence = pair.1;
        let result = libc.lseek(self.fd, offset, whence);
        if result < 0 {
            throw Error.last()
        }
        result
    }

    /// Returns the current position in the file.
    ///
    /// Equivalent to `seek(to: .Current(0))`.
    ///
    /// Example:
    ///     var file = try File.open(path: "data.bin")
    ///     let start = try file.position()  // 0
    ///     try file.read(into: buf.asSlice())
    ///     let after = try file.position()  // advanced by bytes read
    public func position() -> Int64 throws Error {
        try self.seek(to: .Current(0))
    }

    /// Seeks to the beginning of the file.
    ///
    /// Equivalent to `seek(to: .Start(0))` but discards the position result.
    ///
    /// Example:
    ///     var file = try File.open(path: "data.txt")
    ///     // read some data...
    ///     try file.rewind()  // back to start
    ///     // read again from beginning
    public mutating func rewind() -> () throws Error {
        try self.seek(to: .Start(0));
        ()
    }

    // ========================================================================
    // LOW-LEVEL ACCESS
    // ========================================================================

    /// Returns the raw file descriptor.
    ///
    /// Use for interop with libc functions or FFI. The File retains
    /// ownership; do not close the returned fd manually.
    ///
    /// Example:
    ///     let file = try File.open(path: "data.bin")
    ///     let fd = file.rawFd()
    ///     // use fd with libc functions...
    public func rawFd() -> libc.Fd {
        self.fd
    }

    // ========================================================================
    // DESTRUCTOR
    // ========================================================================

    /// Closes the file descriptor.
    ///
    /// Called automatically when the File goes out of scope.
    /// Errors during close are silently ignored (RAII semantics).
    deinit {
        if self.fd >= 0 {
            let _ = libc.close(self.fd);
        }
    }
}

// ============================================================================
// CONVENIENCE FUNCTIONS
// ============================================================================

/// Reads an entire file into a string.
///
/// Opens the file, reads all contents, closes the file, and returns
/// the contents as a UTF-8 string. Suitable for small to medium files.
///
/// Returns Err if the file cannot be opened or read.
///
/// Example:
///     let config = try readFileString(path: "config.json")
///     let readme = try readFileString(path: "/etc/hosts")
public func readFileString(path: String) -> String throws Error {
    var file = try File.open(path);
    var bytes = [UInt8]();
    try readAll(file, into: bytes);
    // Build string from bytes
    var result = String();
    var i: Int64 = 0;
    let count = bytes.count();
    while i < count {
        result.appendByte(bytes.getUnchecked(i));
        i = i + 1
    }
    result
}

/// Reads an entire file into a byte array.
///
/// Opens the file, reads all contents, closes the file, and returns
/// the contents as a byte array. Suitable for binary files.
///
/// Example:
///     let bytes = try readFileBytes(path: "image.png")
public func readFileBytes(path: String) -> [UInt8] throws Error {
    var file = try File.open(path);
    var bytes = [UInt8]();
    try readAll(file, into: bytes);
    bytes
}

/// Writes a string to a file, creating or truncating as needed.
///
/// Creates the file if it doesn't exist, truncates if it does.
/// Writes the string as UTF-8 bytes.
///
/// Example:
///     try writeFileString(path: "output.txt", content: "Hello, World!")
public func writeFileString(path: String, content: String) -> () throws Error {
    var file = try File.create(path);
    try writeStr(file, content)
}

/// Writes bytes to a file, creating or truncating as needed.
///
/// Creates the file if it doesn't exist, truncates if it does.
///
/// Example:
///     let data: [UInt8] = [0x89, 0x50, 0x4E, 0x47]  // PNG header
///     try writeFileBytes(path: "header.bin", content: data)
public func writeFileBytes(path: String, content: [UInt8]) -> () throws Error {
    var file = try File.create(path);
    try writeAll(file, from: content.asSlice())
}

/// Appends a string to a file, creating if it doesn't exist.
///
/// Opens the file in append mode and writes the string as UTF-8.
/// Useful for log files or accumulating data.
///
/// Example:
///     try appendFileString(path: "log.txt", content: "Event occurred\n")
public func appendFileString(path: String, content: String) -> () throws Error {
    var file = try File.openAppend(path);
    try writeStr(file, content)
}

/// Appends bytes to a file, creating if it doesn't exist.
///
/// Example:
///     try appendFileBytes(path: "data.bin", content: newData)
public func appendFileBytes(path: String, content: [UInt8]) -> () throws Error {
    var file = try File.openAppend(path);
    try writeAll(file, from: content.asSlice())
}
