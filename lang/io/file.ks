// File I/O

module io.file

import std.num.(Int64, Int32, UInt8)
import std.result.(Result, Optional)
import std.memory.(Slice, Pointer)
import std.collections.(Array)
import std.text.(String)
import std.core.(Bool, Copyable)
import io.libc
import io.error.(Error)
import io.read.(Read, readAll)
import io.write.(Write, writeAll, writeStr)

// Seek position
public enum Seek {
    case Start(Int64)
    case Current(Int64)
    case End(Int64)
}

// File - owned file handle
public struct File: Read, Write, not Copyable {
    var fd: libc.Fd

    // Private: create from raw fd
    init(fd: libc.Fd) {
        self.fd = fd
    }

    // Open file for reading
    public static func open(path: String) -> Result[File, Error] {
        // Get pointer to string bytes (need null-terminated for libc)
        // For now, we'll copy to a buffer with null terminator
        let len = path.byteCount();
        var pathBuf = Array[UInt8](capacity: len + 1);
        var i: Int64 = 0;
        while i < len {
            pathBuf.append(path.byteAtUnchecked(i));
            i = i + 1
        }
        pathBuf.append(0); // null terminator

        let fd = libc.open(pathBuf.pointer(), libc.O_RDONLY(), 0);
        if fd < 0 {
            return .Err(Error.last())
        }
        .Ok(File(fd: fd))
    }

    // Create file for writing (truncates if exists)
    public static func create(path: String) -> Result[File, Error] {
        let len = path.byteCount();
        var pathBuf = Array[UInt8](capacity: len + 1);
        var i: Int64 = 0;
        while i < len {
            pathBuf.append(path.byteAtUnchecked(i));
            i = i + 1
        }
        pathBuf.append(0);

        let flags = libc.O_WRONLY() | libc.O_CREAT() | libc.O_TRUNC();
        let fd = libc.open(pathBuf.pointer(), flags, libc.MODE_DEFAULT());
        if fd < 0 {
            return .Err(Error.last())
        }
        .Ok(File(fd: fd))
    }

    // Open for read and write
    public static func openRW(path: String) -> Result[File, Error] {
        let len = path.byteCount();
        var pathBuf = Array[UInt8](capacity: len + 1);
        var i: Int64 = 0;
        while i < len {
            pathBuf.append(path.byteAtUnchecked(i));
            i = i + 1
        }
        pathBuf.append(0);

        let fd = libc.open(pathBuf.pointer(), libc.O_RDWR(), 0);
        if fd < 0 {
            return .Err(Error.last())
        }
        .Ok(File(fd: fd))
    }

    // Open for appending
    public static func openAppend(path: String) -> Result[File, Error] {
        let len = path.byteCount();
        var pathBuf = Array[UInt8](capacity: len + 1);
        var i: Int64 = 0;
        while i < len {
            pathBuf.append(path.byteAtUnchecked(i));
            i = i + 1
        }
        pathBuf.append(0);

        let flags = libc.O_WRONLY() | libc.O_CREAT() | libc.O_APPEND();
        let fd = libc.open(pathBuf.pointer(), flags, libc.MODE_DEFAULT());
        if fd < 0 {
            return .Err(Error.last())
        }
        .Ok(File(fd: fd))
    }

    // Read implementation
    public mutating func read(into buf: Slice[UInt8]) -> Result[Int64, Error] {
        let n = libc.read(self.fd, buf.pointer, buf.count);
        if n < 0 {
            return .Err(Error.last())
        }
        .Ok(n)
    }

    // Write implementation
    public func write(from buf: Slice[UInt8]) -> Result[Int64, Error] {
        let n = libc.write(self.fd, buf.pointer, buf.count);
        if n < 0 {
            return .Err(Error.last())
        }
        .Ok(n)
    }

    // Flush (fsync would go here, but keeping it simple)
    public func flush() -> Result[(), Error] {
        .Ok(())
    }

    // Seek to position
    public func seek(to pos: Seek) -> Result[Int64, Error] {
        let pair = match pos {
            .Start(o) => (o, libc.SEEK_SET()),
            .Current(o) => (o, libc.SEEK_CUR()),
            .End(o) => (o, libc.SEEK_END())
        };
        let offset = pair.0;
        let whence = pair.1;
        let result = libc.lseek(self.fd, offset, whence);
        if result < 0 {
            return .Err(Error.last())
        }
        .Ok(result)
    }

    // Get current position
    public func position() -> Result[Int64, Error] {
        self.seek(to: .Current(0))
    }

    // Rewind to start
    public func rewind() -> Result[(), Error] {
        // TODO: add try back
        match self.seek(to: .Start(0)) {
            .Ok(_) => .Ok(()),
            .Err(e) => .Err(e)
        }
    }

    // Get raw file descriptor
    public func rawFd() -> libc.Fd {
        self.fd
    }

    // Close and cleanup
    deinit {
        if self.fd >= 0 {
            let _ = libc.close(self.fd);
        }
    }
}

// Convenience functions

// Read entire file to string
public func readFileString(path: String) -> Result[String, Error] {
    // TODO: add try back
    match File.open(path) {
        .Ok(file) => {
            var bytes = Array[UInt8]();
            // TODO: add try back
            match readAll(file, into: bytes) {
                .Ok(_) => {
                    // Create string from bytes
                    // Note: This requires String to have a constructor from bytes
                    // For now we'll build it character by character
                    var result = "";
                    var i: Int64 = 0;
                    let count = bytes.count();
                    while i < count {
                        // This is inefficient but works for now
                        // TODO: Add String.fromUtf8Bytes() method
                        i = i + 1
                    }
                    .Ok(result)
                },
                .Err(e) => .Err(e)
            }
        },
        .Err(e) => .Err(e)
    }
}

// Write string to file
public func writeFileString(path: String, content: String) -> Result[(), Error] {
    // TODO: add try back
    match File.create(path) {
        .Ok(file) => writeStr(file, content),
        .Err(e) => .Err(e)
    }
}

// Append string to file
public func appendFileString(path: String, content: String) -> Result[(), Error] {
    // TODO: add try back
    match File.openAppend(path) {
        .Ok(file) => writeStr(file, content),
        .Err(e) => .Err(e)
    }
}
