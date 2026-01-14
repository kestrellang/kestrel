// File I/O

module io.file

import std.(Optional, Array, Slice, UInt8, String, Pointer)
import std.ops.(NonCopyable)
import io.libc
import io.error.(Error, Result)
import io.read.(Read)
import io.write.(Write)

// Seek position
public enum Seek {
    case Start(Int64)
    case Current(Int64)
    case End(Int64)
}

// File - owned file handle
public struct File: Read, Write, NonCopyable {
    var fd: libc.Fd

    // Private: create from raw fd
    init(fd: libc.Fd) {
        self.fd = fd
    }

    // Open file for reading
    public static func open(path: String) -> Result[File] {
        let ptr = path.bytes.pointer
        let fd = libc.open(ptr, libc.O_RDONLY, 0)
        if fd < 0 {
            return .Err(Error.last())
        }
        .Ok(File(fd: fd))
    }

    // Create file for writing (truncates if exists)
    public static func create(path: String) -> Result[File] {
        let ptr = path.bytes.pointer
        let fd = libc.open(ptr, libc.O_WRONLY | libc.O_CREAT | libc.O_TRUNC, libc.MODE_DEFAULT)
        if fd < 0 {
            return .Err(Error.last())
        }
        .Ok(File(fd: fd))
    }

    // Open for read and write
    public static func openRW(path: String) -> Result[File] {
        let ptr = path.bytes.pointer
        let fd = libc.open(ptr, libc.O_RDWR, 0)
        if fd < 0 {
            return .Err(Error.last())
        }
        .Ok(File(fd: fd))
    }

    // Open for appending
    public static func append(path: String) -> Result[File] {
        let ptr = path.bytes.pointer
        let fd = libc.open(ptr, libc.O_WRONLY | libc.O_CREAT | libc.O_APPEND, libc.MODE_DEFAULT)
        if fd < 0 {
            return .Err(Error.last())
        }
        .Ok(File(fd: fd))
    }

    // Create new file (fails if exists)
    public static func createNew(path: String) -> Result[File] {
        let ptr = path.bytes.pointer
        let fd = libc.open(ptr, libc.O_WRONLY | libc.O_CREAT | libc.O_EXCL, libc.MODE_DEFAULT)
        if fd < 0 {
            return .Err(Error.last())
        }
        .Ok(File(fd: fd))
    }

    // Open with custom flags
    public static func openWith(path: String, flags: Int32, mode: Int32) -> Result[File] {
        let ptr = path.bytes.pointer
        let fd = libc.open(ptr, flags, mode)
        if fd < 0 {
            return .Err(Error.last())
        }
        .Ok(File(fd: fd))
    }

    // Read implementation
    public func read(into buf: Slice[UInt8]) -> Result[Int] {
        let n = libc.read(self.fd, buf.pointer, UInt(buf.count))
        if n < 0 {
            return .Err(Error.last())
        }
        .Ok(n)
    }

    // Write implementation
    public func write(from buf: Slice[UInt8]) -> Result[Int] {
        let n = libc.write(self.fd, buf.pointer, UInt(buf.count))
        if n < 0 {
            return .Err(Error.last())
        }
        .Ok(n)
    }

    // Flush (fsync would go here, but keeping it simple)
    public func flush() -> Result[Unit] {
        .Ok(())
    }

    // Seek to position
    public func seek(to pos: Seek) -> Result[Int64] {
        let (offset, whence) = match pos {
            .Start(let o) => (o, libc.SEEK_SET),
            .Current(let o) => (o, libc.SEEK_CUR),
            .End(let o) => (o, libc.SEEK_END)
        }
        let result = libc.lseek(self.fd, offset, whence)
        if result < 0 {
            return .Err(Error.last())
        }
        .Ok(result)
    }

    // Get current position
    public func position() -> Result[Int64] {
        self.seek(to: .Current(0))
    }

    // Rewind to start
    public func rewind() -> Result[Unit] {
        _ = try self.seek(to: .Start(0))
        .Ok(())
    }

    // Get raw file descriptor
    public var rawFd: libc.Fd {
        self.fd
    }

    // Close and cleanup
    deinit {
        if self.fd >= 0 {
            _ = libc.close(self.fd)
        }
    }
}

// Convenience functions

// Read entire file to string
public func readString(path: String) -> Result[String] {
    var file = try File.open(path: path)
    var bytes: Array[UInt8] = []
    _ = try file.readAll(into: bytes)
    .Ok(String(utf8: bytes.asSlice()))
}

// Read entire file to bytes
public func readBytes(path: String) -> Result[Array[UInt8]] {
    var file = try File.open(path: path)
    var bytes: Array[UInt8] = []
    _ = try file.readAll(into: bytes)
    .Ok(bytes)
}

// Write string to file
public func writeString(path: String, content: String) -> Result[Unit] {
    var file = try File.create(path: path)
    file.writeStr(s: content)
}

// Write bytes to file
public func writeBytes(path: String, content: Slice[UInt8]) -> Result[Unit] {
    var file = try File.create(path: path)
    file.writeAll(from: content)
}

// Append string to file
public func appendString(path: String, content: String) -> Result[Unit] {
    var file = try File.append(path: path)
    file.writeStr(s: content)
}
